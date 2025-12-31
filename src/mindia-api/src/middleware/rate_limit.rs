#![allow(dead_code)]

use crate::auth::models::TenantContext;
use crate::middleware::audit;
use crate::utils::ip_extraction::extract_client_ip;
use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Simple in-memory rate limiter for HTTP requests
#[derive(Clone)]
struct RateLimitBucket {
    count: u32,
    reset_at: Instant,
}

impl RateLimitBucket {
    fn new() -> Self {
        Self {
            count: 0,
            reset_at: Instant::now() + Duration::from_secs(60),
        }
    }

    fn check_and_increment(&mut self, limit: u32, window_seconds: u64) -> (bool, u32) {
        let now = Instant::now();

        // Reset if window expired
        if now >= self.reset_at {
            self.count = 0;
            self.reset_at = now + Duration::from_secs(window_seconds);
        }

        if self.count < limit {
            self.count += 1;
            let remaining = limit.saturating_sub(self.count);
            (true, remaining)
        } else {
            (false, 0)
        }
    }

    fn remaining(&self, limit: u32) -> u32 {
        limit.saturating_sub(self.count)
    }

    fn reset_in(&self) -> Duration {
        self.reset_at.saturating_duration_since(Instant::now())
    }
}

/// Sharded rate limiter to reduce lock contention
///
/// Uses multiple shards (separate HashMaps) to distribute load and reduce
/// contention on a single mutex. Keys are hashed to determine which shard to use.
#[derive(Clone)]
pub struct HttpRateLimiter {
    shards: Vec<Arc<Mutex<HashMap<String, RateLimitBucket>>>>,
    shard_count: usize,
    limit_per_minute: u32,
    tenant_limit_per_minute: Option<u32>, // Per-tenant limit (if set)
    window_seconds: u64,
    max_buckets: usize, // Maximum number of buckets per shard before cleanup
}

impl HttpRateLimiter {
    /// Get the shard index for a given key
    fn shard_index(&self, key: &str) -> usize {
        // Use a simple hash to distribute keys across shards
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shard_count
    }
}

impl HttpRateLimiter {
    /// Create a new rate limiter with default shard count (16 shards)
    pub fn new(limit_per_minute: u32) -> Self {
        Self::with_shards(limit_per_minute, 16)
    }

    /// Create rate limiter with custom shard count
    ///
    /// # Arguments
    /// * `limit_per_minute` - Global rate limit per minute
    /// * `shard_count` - Number of shards (should be a power of 2 for best distribution)
    pub fn with_shards(limit_per_minute: u32, shard_count: usize) -> Self {
        let shards = (0..shard_count)
            .map(|_| Arc::new(Mutex::new(HashMap::new())))
            .collect();
        Self {
            shards,
            shard_count,
            limit_per_minute,
            tenant_limit_per_minute: None, // Default: same as global limit
            window_seconds: 60,
            max_buckets: 10_000, // Default: 10k buckets per shard max
        }
    }

    /// Create rate limiter with per-tenant limits
    pub fn with_tenant_limit(limit_per_minute: u32, tenant_limit_per_minute: u32) -> Self {
        Self::with_tenant_limit_and_shards(limit_per_minute, tenant_limit_per_minute, 16)
    }

    /// Create rate limiter with per-tenant limits and custom shard count
    pub fn with_tenant_limit_and_shards(
        limit_per_minute: u32,
        tenant_limit_per_minute: u32,
        shard_count: usize,
    ) -> Self {
        let shards = (0..shard_count)
            .map(|_| Arc::new(Mutex::new(HashMap::new())))
            .collect();
        Self {
            shards,
            shard_count,
            limit_per_minute,
            tenant_limit_per_minute: Some(tenant_limit_per_minute),
            window_seconds: 60,
            max_buckets: 10_000,
        }
    }

    /// Cleanup expired buckets to prevent memory leak
    /// Removes buckets that have expired and are beyond their reset window
    /// Cleans up all shards in parallel
    pub async fn cleanup_expired_buckets(&self) {
        let now = Instant::now();
        let grace_period = Duration::from_secs(self.window_seconds);
        let mut total_cleaned = 0;

        // Clean up all shards in parallel
        let cleanup_tasks: Vec<_> = self
            .shards
            .iter()
            .map(|shard| {
                let shard = shard.clone();
                tokio::spawn(async move {
                    let mut buckets = shard.lock().await;
                    let before_count = buckets.len();
                    buckets.retain(|_key, bucket| {
                        // Keep buckets that haven't expired yet or recently expired (within grace period)
                        bucket.reset_at > now || (now - bucket.reset_at) < grace_period
                    });
                    before_count - buckets.len()
                })
            })
            .collect();

        // Wait for all cleanup tasks to complete
        for task in cleanup_tasks {
            if let Ok(cleaned) = task.await {
                total_cleaned += cleaned;
            }
        }

        if total_cleaned > 0 {
            tracing::debug!(
                buckets_cleaned = total_cleaned,
                "Cleaned up expired rate limit buckets across all shards"
            );
        }
    }

    pub async fn check_rate_limit(&self, key: &str, limit: u32) -> Result<u32, Duration> {
        // Get the appropriate shard for this key
        let shard_index = self.shard_index(key);
        let shard = &self.shards[shard_index];
        let mut buckets = shard.lock().await;

        // Periodic cleanup: if we're approaching the limit, clean up expired buckets first
        if buckets.len() >= self.max_buckets {
            let now = Instant::now();
            let grace_period = Duration::from_secs(self.window_seconds);

            // Remove expired buckets
            buckets.retain(|_key, bucket| {
                bucket.reset_at > now || (now - bucket.reset_at) < grace_period
            });

            // If still at capacity after cleanup, remove oldest buckets (LRU eviction)
            if buckets.len() >= self.max_buckets {
                let oldest_key = buckets
                    .iter()
                    .min_by_key(|(_, bucket)| bucket.reset_at)
                    .map(|(k, _)| k.clone());

                if let Some(key_to_remove) = oldest_key {
                    buckets.remove(&key_to_remove);
                    tracing::debug!(
                        removed_key = %key_to_remove,
                        shard_index = shard_index,
                        remaining_buckets = buckets.len(),
                        "Evicted oldest rate limit bucket due to capacity limit"
                    );
                }
            }
        }

        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(RateLimitBucket::new);

        let (allowed, remaining) = bucket.check_and_increment(limit, self.window_seconds);
        if allowed {
            Ok(remaining)
        } else {
            Err(bucket.reset_in())
        }
    }
}

/// HTTP rate limiting middleware
///
/// Implements rate limiting for HTTP requests using a sharded in-memory store.
/// Supports both IP-based and tenant-based rate limiting with configurable limits.
///
/// # Rate Limiting Strategy
/// - Uses sharded HashMap to reduce lock contention (configurable via `RATE_LIMITER_SHARD_COUNT`)
/// - Tenant-based limiting takes precedence over IP-based limiting
/// - Implements sliding window algorithm with automatic cleanup of expired buckets
///
/// # Headers
/// Adds the following headers to responses:
/// - `X-RateLimit-Limit`: The rate limit per minute
/// - `X-RateLimit-Remaining`: Remaining requests in current window
/// - `Retry-After`: Seconds until rate limit resets (only on 429 responses)
///
/// # Errors
/// Returns `429 Too Many Requests` when rate limit is exceeded.
///
/// # Configuration
/// - `RATE_LIMITER_SHARD_COUNT`: Number of shards (default: 16)
/// - Rate limits configured via `Config::http_rate_limit_per_minute()` and
///   `Config::http_tenant_rate_limit_per_minute()`
pub async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<HttpRateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    // Determine rate limit key and limit
    // Priority: tenant_id > IP address
    let (rate_limit_key, limit) = if let Some(tenant_context) =
        request.extensions().get::<TenantContext>()
    {
        // Use tenant-based rate limiting if tenant context is available
        let tenant_limit = rate_limiter
            .tenant_limit_per_minute
            .unwrap_or(rate_limiter.limit_per_minute);
        (format!("tenant:{}", tenant_context.tenant_id), tenant_limit)
    } else {
        // Fall back to IP-based rate limiting
        // Use validated IP extraction to prevent X-Forwarded-For spoofing
        let trusted_proxy_count = std::env::var("TRUSTED_PROXY_COUNT")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1); // Default: trust 1 proxy (typical load balancer setup)

        let socket_addr = request.extensions().get::<std::net::SocketAddr>().copied();
        let ip = extract_client_ip(request.headers(), socket_addr.as_ref(), trusted_proxy_count);
        (format!("ip:{}", ip), rate_limiter.limit_per_minute)
    };

    // Check rate limit
    match rate_limiter.check_rate_limit(&rate_limit_key, limit).await {
        Ok(remaining) => {
            let mut response = next.run(request).await;

            // Add rate limit headers
            if let Ok(header_value) = HeaderValue::from_str(&limit.to_string()) {
                response
                    .headers_mut()
                    .insert("X-RateLimit-Limit", header_value);
            }
            if let Ok(header_value) = HeaderValue::from_str(&remaining.to_string()) {
                response
                    .headers_mut()
                    .insert("X-RateLimit-Remaining", header_value);
            }

            response
        }
        Err(reset_in) => {
            // Log rate limit violation for audit
            let tenant_id = request
                .extensions()
                .get::<TenantContext>()
                .map(|ctx| ctx.tenant_id);
            let request_path = request.uri().path().to_string();
            audit::log_rate_limit_exceeded(
                tenant_id,
                Some(rate_limit_key.clone()),
                Some(request_path),
                limit,
            );

            let reset_seconds = reset_in.as_secs().max(1);

            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({
                    "error": "Too many requests. Please slow down."
                })),
            )
                .into_response();

            if let Ok(header_value) = HeaderValue::from_str(&limit.to_string()) {
                response
                    .headers_mut()
                    .insert("X-RateLimit-Limit", header_value);
            }
            if let Ok(header_value) = HeaderValue::from_str("0") {
                response
                    .headers_mut()
                    .insert("X-RateLimit-Remaining", header_value);
            }
            if let Ok(header_value) = HeaderValue::from_str(&reset_seconds.to_string()) {
                response.headers_mut().insert("Retry-After", header_value);
            }

            response
        }
    }
}
