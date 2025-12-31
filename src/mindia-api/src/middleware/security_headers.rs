use axum::http::HeaderValue;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Security headers configuration
#[derive(Clone)]
pub struct SecurityHeadersConfig {
    pub cdn_domains: Vec<String>,
    pub is_production: bool,
}

impl SecurityHeadersConfig {
    pub fn new(cdn_domains: Vec<String>, is_production: bool) -> Self {
        Self {
            cdn_domains,
            is_production,
        }
    }

    /// Build Content-Security-Policy header value
    fn build_csp(&self) -> String {
        #[allow(clippy::useless_vec)] // Vec needed for dynamic CSP building with CDN domains
        let mut csp_parts = vec![
            "default-src 'self'".to_string(),
            "script-src 'self'".to_string(),
            // Remove 'unsafe-inline' for better security - use nonces or hashes in production
            "style-src 'self'".to_string(),
            "img-src 'self' data: https:".to_string(),
            "font-src 'self' data:".to_string(),
            "connect-src 'self'".to_string(),
            "frame-ancestors 'none'".to_string(), // Prevent embedding in frames
        ];

        // Add CDN domains to img-src and connect-src if configured
        if !self.cdn_domains.is_empty() {
            let cdn_list = self.cdn_domains.join(" ");
            // Add CDN to img-src for media delivery
            if let Some(img_idx) = csp_parts.iter().position(|s| s.starts_with("img-src")) {
                csp_parts[img_idx] = format!("img-src 'self' data: https: {}", cdn_list);
            }
            // Add CDN to connect-src for API calls
            if let Some(connect_idx) = csp_parts.iter().position(|s| s.starts_with("connect-src")) {
                csp_parts[connect_idx] = format!("connect-src 'self' {}", cdn_list);
            }
        }

        csp_parts.join("; ")
    }
}

/// Security headers middleware
/// Adds security headers to all HTTP responses
pub async fn security_headers_middleware(
    State(config): State<Arc<SecurityHeadersConfig>>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // X-Content-Type-Options: Prevent MIME type sniffing
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );

    // X-Frame-Options: Prevent clickjacking (redundant with CSP frame-ancestors, but kept for older browsers)
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));

    // X-XSS-Protection: optional XSS filter
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Referrer-Policy: Control referrer information
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // HSTS header (only set in production over HTTPS)
    if config.is_production {
        // HSTS: Force HTTPS for 1 year, include subdomains
        headers.insert(
            "Strict-Transport-Security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
        );
    }

    // Content-Security-Policy: Restrict resource loading
    // Configurable CSP with CDN support
    let csp = config.build_csp();
    if let Ok(header_value) = HeaderValue::from_str(&csp) {
        headers.insert("Content-Security-Policy", header_value);
    }

    // Permissions-Policy: Restrict browser features
    headers.insert(
        "Permissions-Policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    // Cache-Control: Prevent caching of sensitive API responses
    // no-store: Don't cache the response at all
    // private: Response is intended for a single user and should not be stored by shared caches
    headers.insert(
        "Cache-Control",
        HeaderValue::from_static("no-store, private"),
    );

    response
}
