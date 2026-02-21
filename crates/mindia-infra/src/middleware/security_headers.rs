use axum::http::HeaderValue;
use axum::{extract::Request, middleware::Next, response::Response};

static CACHED_IS_PRODUCTION: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
    std::env::var("ENVIRONMENT")
        .map(|e| e.to_lowercase() == "production" || e.to_lowercase() == "prod")
        .unwrap_or(false)
});

/// Security headers middleware
/// Adds security headers to all HTTP responses
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // X-Content-Type-Options: Prevent MIME type sniffing
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );

    // X-Frame-Options: Prevent clickjacking
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));

    // X-XSS-Protection intentionally omitted: deprecated in modern browsers.
    // CSP (Content-Security-Policy) below provides XSS protection instead.

    // Referrer-Policy: Control referrer information
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // HSTS header (only set in production over HTTPS, cached at first use)
    if *CACHED_IS_PRODUCTION {
        headers.insert(
            "Strict-Transport-Security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    // Content-Security-Policy: Restrict resource loading
    // Basic CSP - adjust based on your needs
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'"),
    );

    // Permissions-Policy: Restrict browser features
    headers.insert(
        "Permissions-Policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_headers_function_exists() {
        // Note: Testing middleware in isolation requires complex setup with tower::Service
        // Integration tests cover the actual behavior through the full server stack
        // This test just ensures the function exists and is public
        let _ = security_headers_middleware;
    }
}
