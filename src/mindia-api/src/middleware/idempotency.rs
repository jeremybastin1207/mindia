//! Idempotency middleware
//!
//! Ensures that requests with the same Idempotency-Key header return the same response
//! within a time window. Critical for AI agents that may retry requests on network failures.

#![allow(dead_code)]

use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// In-memory idempotency store
/// In production, use Redis for distributed systems
type IdempotencyStore = Arc<RwLock<std::collections::HashMap<String, CachedResponse>>>;

#[derive(Clone)]
struct CachedResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    created_at: std::time::Instant,
}

/// Idempotency middleware state
pub struct IdempotencyState {
    store: IdempotencyStore,
    ttl: Duration,
}

impl IdempotencyState {
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: Arc::new(RwLock::new(std::collections::HashMap::new())),
            ttl,
        }
    }

    async fn get(&self, key: &str) -> Option<CachedResponse> {
        let store = self.store.read().await;
        store.get(key).cloned()
    }

    async fn set(&self, key: String, response: CachedResponse) {
        let mut store = self.store.write().await;
        store.insert(key, response);
    }

    async fn cleanup_expired(&self) {
        let mut store = self.store.write().await;
        let now = std::time::Instant::now();
        store.retain(|_, cached| now.duration_since(cached.created_at) < self.ttl);
    }
}

/// Idempotency middleware
pub async fn idempotency_middleware(
    State(state): State<Arc<IdempotencyState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    // Check for Idempotency-Key header
    let idempotency_key = if let Some(key_header) = headers.get("Idempotency-Key") {
        match key_header.to_str() {
            Ok(key) => key.to_string(),
            Err(_) => {
                warn!("Invalid Idempotency-Key header value (not UTF-8)");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(crate::error::ErrorResponse {
                        error: "Invalid Idempotency-Key header".to_string(),
                        details: None,
                        error_type: None,
                        code: "INVALID_HEADER".to_string(),
                        recoverable: false,
                        suggested_action: None,
                    }),
                )
                    .into_response();
            }
        }
    } else {
        // No idempotency key, proceed normally
        return next.run(request).await;
    };

    // Validate key format (should be reasonable length)
    if idempotency_key.len() > 256 {
        warn!("Idempotency-Key too long");
        return (
            StatusCode::BAD_REQUEST,
            Json(crate::error::ErrorResponse {
                error: "Idempotency-Key must be 256 characters or less".to_string(),
                details: None,
                error_type: None,
                code: "INVALID_KEY_LENGTH".to_string(),
                recoverable: false,
                suggested_action: None,
            }),
        )
            .into_response();
    }

    // Use state from middleware parameter
    {
        // Check for cached response
        if let Some(cached) = state.get(&idempotency_key).await {
            // Check if cached response is still valid
            if cached.created_at.elapsed() < state.ttl {
                debug!(
                    idempotency_key = %idempotency_key,
                    "Returning cached idempotent response"
                );

                // Rebuild response from cache
                let mut response = match Response::builder()
                    .status(cached.status)
                    .body(axum::body::Body::from(cached.body))
                {
                    Ok(resp) => resp,
                    Err(e) => {
                        warn!(
                            error = %e,
                            idempotency_key = %idempotency_key,
                            "Failed to rebuild cached response, executing request normally"
                        );
                        // If we can't rebuild the cached response, execute the request normally
                        return next.run(request).await;
                    }
                };

                // Restore headers
                for (name, value) in cached.headers {
                    if let (Ok(name), Ok(value)) = (
                        axum::http::HeaderName::from_bytes(name.as_bytes()),
                        HeaderValue::from_str(&value),
                    ) {
                        response.headers_mut().insert(name, value);
                    }
                }

                // Add idempotency header to indicate this was cached
                let header_value = HeaderValue::from_static("true");
                response
                    .headers_mut()
                    .insert("X-Idempotent-Replayed", header_value);

                return response;
            }
        }

        // Execute request
        let response = next.run(request).await;

        // Cache successful responses (2xx and 4xx, but not 5xx)
        let status = response.status();
        if status.is_success() || status.is_client_error() {
            // Extract response body and headers
            let (parts, body) = response.into_parts();
            let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(bytes) => bytes.to_vec(),
                Err(_) => {
                    // If we can't read the body, don't cache
                    return Response::from_parts(parts, axum::body::Body::empty());
                }
            };

            let headers: Vec<(String, String)> = parts
                .headers
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|v| (name.to_string(), v.to_string()))
                })
                .collect();

            let cached = CachedResponse {
                status: status.as_u16(),
                headers,
                body: body_bytes.clone(),
                created_at: std::time::Instant::now(),
            };

            let idempotency_key_for_header = idempotency_key.clone();
            state.set(idempotency_key_for_header.clone(), cached).await;

            // Rebuild response
            let mut response = Response::from_parts(parts, axum::body::Body::from(body_bytes));
            response.headers_mut().insert(
                "X-Idempotent-Key",
                HeaderValue::from_str(&idempotency_key_for_header)
                    .unwrap_or_else(|_| HeaderValue::from_static("invalid")),
            );
            response
        } else {
            // Don't cache 5xx errors
            response
        }
    }
}

/// Setup idempotency state
pub fn setup_idempotency_state(ttl_seconds: u64) -> Arc<IdempotencyState> {
    Arc::new(IdempotencyState::new(Duration::from_secs(ttl_seconds)))
}
