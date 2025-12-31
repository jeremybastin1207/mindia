use axum::http::HeaderValue;
use axum::{extract::Request, middleware::Next, response::Response};
use uuid::Uuid;

/// Request ID extension type
#[derive(Clone, Debug)]
pub struct RequestId(
    #[allow(dead_code)] // Used via get_request_id() function
    pub  String,
);

/// Request ID middleware
/// Generates a unique request ID for each request and includes it in:
/// - Response headers (X-Request-ID)
/// - Request extensions (for logging)
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Check if request ID already exists in headers (for request tracing across services)
    let request_id = request
        .headers()
        .get("X-Request-ID")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Insert request ID into request extensions for use in handlers/logging
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    // Process request
    let mut response = next.run(request).await;

    // Add request ID to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("X-Request-ID", header_value);
    }

    response
}

/// Extract request ID from request extensions
pub fn get_request_id(request: &Request) -> Option<String> {
    request
        .extensions()
        .get::<RequestId>()
        .map(|id| id.0.clone())
}
