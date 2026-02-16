//! Wide event middleware for canonical log lines
//!
//! This middleware implements the "wide events" pattern where we build
//! a comprehensive event throughout the request lifecycle and emit it
//! once at the end with tail sampling.

use crate::auth::models::TenantContext;
use crate::middleware::get_request_id;
use crate::state::AppState;
use crate::telemetry::wide_event::{TailSamplingConfig, WideEvent};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

/// Extension key for wide event in request extensions
#[derive(Clone)]
pub struct WideEventExtension(pub WideEvent);

/// Axum extractor for accessing and enriching wide events in handlers
///
/// This allows handlers to easily access the wide event and enrich it with
/// business context. The enriched event should be stored back in response
/// extensions using `store_enriched_event_in_response`.
#[derive(Clone)]
pub struct WideEventCtx(pub WideEvent);

/// Rejection type for WideEventCtx extractor
#[derive(Debug)]
pub struct WideEventRejection;

impl axum::response::IntoResponse for WideEventRejection {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("WideEvent not found in request extensions - middleware may not be configured correctly");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": "Internal server error",
                "code": "WIDE_EVENT_MISSING"
            })),
        )
            .into_response()
    }
}

impl<S> FromRequestParts<S> for WideEventCtx
where
    S: Send + Sync,
{
    type Rejection = WideEventRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<WideEventExtension>()
            .map(|ext| WideEventCtx(ext.0.clone()))
            .ok_or(WideEventRejection)
    }
}

/// Normalize URL path by replacing UUID segments with :id placeholder
fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            // Check if segment is a UUID
            if Uuid::parse_str(segment).is_ok() {
                ":id"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Check if a request should be logged for audit purposes
/// Excludes health checks, metrics, and analytics endpoints
fn should_log_request(path: &str) -> bool {
    // Skip logging for these paths
    let excluded_paths = ["/health", "/metrics"];

    // Check exact matches
    if excluded_paths.contains(&path) {
        return false;
    }

    // Skip analytics endpoints to avoid recursive logging
    if path.starts_with("/api/analytics/") || path.starts_with("/api/audit-logs") {
        return false;
    }

    true
}

/// Mask sensitive data in query strings
fn mask_sensitive_data(query: Option<String>) -> Option<String> {
    query.map(|q| {
        let sensitive_keywords = [
            "password",
            "token",
            "secret",
            "key",
            "authorization",
            "api_key",
            "apikey",
        ];

        q.split('&')
            .map(|param| {
                if let Some(eq_pos) = param.find('=') {
                    let key = &param[..eq_pos];
                    let key_lower = key.to_lowercase();

                    if sensitive_keywords.iter().any(|&kw| key_lower.contains(kw)) {
                        format!("{}=***MASKED***", key)
                    } else {
                        param.to_string()
                    }
                } else {
                    param.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("&")
    })
}

/// Wide event middleware that builds and emits canonical log lines
pub async fn wide_event_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();

    // Extract request metadata
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let normalized_path = normalize_path(&path);

    // Check if this request should be logged
    let should_log = should_log_request(&path);

    // Get or generate request ID
    let request_id = get_request_id(&request).unwrap_or_else(|| Uuid::new_v4().to_string());

    // Initialize wide event with request context
    let mut event = WideEvent::new(
        request_id.clone(),
        state.config.otel_service_name().to_string(),
        state.config.environment().to_string(),
        method.clone(),
        path.clone(),
        Utc::now(),
    );

    // Add service context
    event.version = Some(state.config.otel_service_version().to_string());
    event.deployment_id = std::env::var("DEPLOYMENT_ID").ok();
    event.region = std::env::var("AWS_REGION")
        .or_else(|_| std::env::var("REGION"))
        .ok();
    event.normalized_path = Some(normalized_path.clone());
    event.query_string = mask_sensitive_data(request.uri().query().map(|q| q.to_string()));

    // Extract client IP
    event.client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
        .or_else(|| {
            request
                .extensions()
                .get::<std::net::SocketAddr>()
                .map(|addr| addr.ip().to_string())
        });

    // Extract user agent
    event.user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Extract request size
    event.request_size_bytes = request
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    // Extract trace ID if available
    event.trace_id = request
        .headers()
        .get("traceparent")
        .and_then(|h| h.to_str().ok())
        .map(|s| {
            // Extract trace ID from W3C traceparent format: 00-{trace_id}-{parent_id}-{flags}
            s.split('-')
                .nth(1)
                .map(|tid| tid.to_string())
                .unwrap_or_else(|| s.to_string())
        })
        .or_else(|| {
            request
                .headers()
                .get("x-trace-id")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        });

    // Insert is_production flag into request extensions for error handling
    request.extensions_mut().insert(state.is_production);

    // Extract tenant context if available (auth middleware runs before this for protected routes)
    // For public routes, tenant context won't be available, which is fine
    if let Some(tenant_ctx) = request.extensions().get::<TenantContext>() {
        event.with_tenant_context(tenant_ctx);
    }

    // Insert wide event into request extensions so handlers can enrich it further
    // We clone it so handlers can modify their copy via helper functions
    request
        .extensions_mut()
        .insert(WideEventExtension(event.clone()));

    // Execute the request - after this, we can't access request extensions anymore
    // Handlers can enrich the event via helper functions, and optionally store it in response extensions
    let response = next.run(request).await;

    // Extract response metadata
    let status_code = response.status().as_u16();
    let duration_ms = start_time.elapsed().as_millis() as u64;

    // Try to get enriched event from response extensions (if handlers stored it there)
    // Otherwise, use the event we built (which already has tenant context from before next.run())
    let mut final_event = response
        .extensions()
        .get::<WideEventExtension>()
        .map(|ext| ext.0.clone())
        .unwrap_or_else(|| {
            // Fallback: use the event we built (already has tenant context if available)
            event
        });

    // Update response size (needed after response is available)
    final_event.response_size_bytes = response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    // Finalize the event with response information
    final_event.finalize(status_code, duration_ms);

    // Apply tail sampling
    let sampling_config = get_sampling_config(&state.config);
    let should_sample = sampling_config.should_sample(&final_event);

    // Emit the wide event as a structured log
    if should_log && should_sample {
        match final_event.to_json_string() {
            Ok(json) => {
                // Emit as a single structured log line
                match final_event.outcome {
                    crate::telemetry::wide_event::Outcome::Error => {
                        warn!(%request_id, %status_code, duration_ms = final_event.duration_ms, "{}", json);
                    }
                    crate::telemetry::wide_event::Outcome::ClientError => {
                        warn!(%request_id, %status_code, duration_ms = final_event.duration_ms, "{}", json);
                    }
                    crate::telemetry::wide_event::Outcome::Success => {
                        info!(%request_id, %status_code, duration_ms = final_event.duration_ms, "{}", json);
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize wide event to JSON");
            }
        }
    }

    response
}

/// Get sampling configuration from config
fn get_sampling_config(_config: &mindia_core::Config) -> TailSamplingConfig {
    let slow_threshold = std::env::var("WIDE_EVENT_SLOW_THRESHOLD_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .or({
            // Default to 2000ms (2 seconds) for p99 threshold
            Some(2000)
        });

    let sample_rate = std::env::var("WIDE_EVENT_SAMPLE_RATE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.05); // Default 5% sampling

    let keep_client_errors = std::env::var("WIDE_EVENT_KEEP_CLIENT_ERRORS")
        .ok()
        .map(|s| s == "true")
        .unwrap_or(false);

    let vip_tenants = std::env::var("WIDE_EVENT_VIP_TENANT_IDS").ok().map(|s| {
        s.split(',')
            .filter_map(|id| Uuid::parse_str(id.trim()).ok())
            .collect::<Vec<_>>()
    });

    let keep_paths = std::env::var("WIDE_EVENT_KEEP_PATHS").ok().map(|s| {
        s.split(',')
            .map(|p| p.trim().to_string())
            .collect::<Vec<_>>()
    });

    TailSamplingConfig {
        keep_all_errors: true, // Always keep errors
        keep_all_client_errors: keep_client_errors,
        slow_request_threshold_ms: slow_threshold,
        vip_tenant_ids: vip_tenants,
        keep_paths,
        random_sample_rate: sample_rate,
        always_keep_enabled: std::env::var("WIDE_EVENT_ALWAYS_KEEP")
            .ok()
            .map(|s| s == "true")
            .unwrap_or(false),
    }
}

/// Helper function to get wide event from request extensions
#[allow(dead_code)] // Public API for handlers to enrich events
pub fn get_wide_event(request: &Request) -> Option<&WideEventExtension> {
    request.extensions().get::<WideEventExtension>()
}

/// Helper function to get mutable wide event from request extensions
#[allow(dead_code)] // Public API for handlers to enrich events
pub fn get_wide_event_mut(request: &mut Request) -> Option<&mut WideEventExtension> {
    request.extensions_mut().get_mut::<WideEventExtension>()
}

/// Helper function for handlers to enrich the wide event with tenant context
/// This should be called by handlers after extracting TenantContext
#[allow(dead_code)] // Public API for handlers to enrich events
pub fn enrich_wide_event_with_tenant(
    request: &mut Request,
    tenant_ctx: &TenantContext,
) -> Option<()> {
    if let Some(ext) = request.extensions_mut().get_mut::<WideEventExtension>() {
        ext.0.with_tenant_context(tenant_ctx);
        Some(())
    } else {
        None
    }
}

/// Helper function for handlers to enrich the wide event with business context
#[allow(dead_code)] // Public API for handlers to enrich events
pub fn enrich_wide_event_with_business<F>(request: &mut Request, f: F) -> Option<()>
where
    F: FnOnce(&mut crate::telemetry::wide_event::BusinessContext),
{
    if let Some(ext) = request.extensions_mut().get_mut::<WideEventExtension>() {
        ext.0.with_business_context(f);
        Some(())
    } else {
        None
    }
}

/// Helper function for handlers to store enriched event in response extensions
/// This allows the middleware to pick up the enriched version after the request is processed
#[allow(dead_code)] // Public API for handlers to enrich events
pub fn store_enriched_event_in_response(response: &mut Response, event: WideEvent) {
    response.extensions_mut().insert(WideEventExtension(event));
}

/// Helper function to convert a Json response to a Response with enriched wide event
/// This is useful for handlers that return `Json<T>` but need to attach the enriched event
pub fn json_response_with_event<T: serde::Serialize>(json: Json<T>, event: WideEvent) -> Response {
    let mut response = json.into_response();
    response.extensions_mut().insert(WideEventExtension(event));
    response
}
