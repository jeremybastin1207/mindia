use crate::auth::models::TenantContext;
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use mindia_core::models::RequestLogInput;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

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
/// Masks values for parameters containing: password, token, secret, key, authorization
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

                    // Check if key contains any sensitive keyword
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

/// Middleware to track analytics for all requests
///
/// This middleware persists request data to a database for analytics and audit purposes.
/// It runs alongside `wide_event_middleware` which handles structured logging with tail sampling.
///
/// **Purpose**: Database persistence for analytics queries, reporting, and audit trails
/// **Wide Event Middleware**: Structured JSON logging for debugging and observability
///
/// Both middlewares use similar normalization and masking logic but serve different purposes:
/// - Analytics: Long-term storage in database for business intelligence
/// - Wide Events: High-cardinality structured logs for debugging (with tail sampling)
pub async fn analytics_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();

    // Insert is_production flag into request extensions for error handling
    request.extensions_mut().insert(state.is_production);

    // Extract request metadata
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let normalized_path = normalize_path(&path);

    // Check if this request should be logged (skip health checks, metrics, etc.)
    let should_log = should_log_request(&path);

    // Mask sensitive data in query string before logging
    let query_string = mask_sensitive_data(request.uri().query().map(|q| q.to_string()));

    // Extract tenant_id from request extensions (if authenticated)
    let tenant_id = request
        .extensions()
        .get::<TenantContext>()
        .map(|ctx| ctx.tenant_id);

    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Extract IP from X-Forwarded-For header (first IP in chain for proxied requests)
    let ip_address = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            let ip = s.split(',').next().unwrap_or("").trim();
            if ip.is_empty() {
                None
            } else {
                Some(ip.to_string())
            }
        })
        .or_else(|| {
            request
                .extensions()
                .get::<std::net::SocketAddr>()
                .map(|addr| addr.ip().to_string())
        });

    // Get request size (approximate from content-length header)
    let request_size_bytes = request
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    // Execute the request
    let response = next.run(request).await;

    // Extract response metadata
    let status_code = response.status().as_u16() as i32;
    let duration_ms = start.elapsed().as_millis() as i64;

    // Get response size (approximate from content-length header)
    let response_size_bytes = response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    // Log the request asynchronously (only if should_log is true)
    if should_log {
        state.db.analytics.log_request(RequestLogInput {
            tenant_id,
            method,
            path,
            normalized_path,
            query_string,
            status_code,
            request_size_bytes,
            response_size_bytes,
            duration_ms,
            user_agent,
            ip_address,
        });
    }

    response
}
