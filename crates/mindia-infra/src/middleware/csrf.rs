//! CSRF (Cross-Site Request Forgery) protection middleware
//!
//! This middleware implements CSRF token validation for state-changing operations.
//! It uses the double-submit cookie pattern for stateless CSRF protection.

use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// CSRF token response
#[derive(Debug, Serialize, Deserialize)]
pub struct CsrfTokenResponse {
    pub token: String,
}

/// Generate a CSRF token
/// Token format: <hmac>.<timestamp>.<nonce>
pub fn generate_csrf_token(secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let nonce = Uuid::new_v4().to_string();
    let message = format!("{}.{}", timestamp, nonce);

    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    let hmac = hex::encode(result.into_bytes());
    let token = format!("{}.{}.{}", hmac, timestamp, nonce);

    token
}

/// CSRF token expiration time (1 hour)
const CSRF_TOKEN_EXPIRATION_SECS: u64 = 3600;

/// Verify a CSRF token
/// Token format: <hmac>.<timestamp>.<nonce>
/// Validates:
/// 1. Token format (must have 3 parts: hmac.timestamp.nonce)
/// 2. HMAC signature matches the secret
/// 3. Token hasn't expired (within CSRF_TOKEN_EXPIRATION_SECS)
fn verify_csrf_token(token: &str, secret: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Token format: <hmac>.<timestamp>.<nonce>
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }

    let hmac_part = parts[0];
    let timestamp_str = parts[1];
    let nonce = parts[2];

    // Parse timestamp
    let timestamp = match timestamp_str.parse::<u64>() {
        Ok(ts) => ts,
        Err(_) => return false,
    };

    // Check expiration (1 hour)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if timestamp + CSRF_TOKEN_EXPIRATION_SECS < now {
        tracing::debug!("CSRF token expired");
        return false;
    }

    // Reconstruct message and verify HMAC
    let message = format!("{}.{}", timestamp, nonce);
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(message.as_bytes());
    let result = mac.finalize();
    let expected_hmac = hex::encode(result.into_bytes());

    // Constant-time comparison to prevent timing attacks
    use subtle::ConstantTimeEq;
    expected_hmac.as_bytes().ct_eq(hmac_part.as_bytes()).into()
}

/// CSRF protection middleware
///
/// This middleware:
/// 1. Allows GET, HEAD, OPTIONS without CSRF tokens (safe methods)
/// 2. Requires CSRF token in X-CSRF-Token header for state-changing methods
/// 3. Validates token against cookie (double-submit pattern)
///
/// For state-changing operations, clients must:
/// 1. First call GET /api/v0/csrf-token to get a token
/// 2. Include the token in X-CSRF-Token header
/// 3. The token will also be set as a cookie automatically
pub async fn csrf_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();

    // Safe methods don't require CSRF protection
    if matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return next.run(request).await;
    }

    // Get CSRF secret from environment (fallback to JWT_SECRET if CSRF_SECRET not set)
    let is_production = std::env::var("ENVIRONMENT")
        .map(|e| e.to_lowercase() == "production" || e.to_lowercase() == "prod")
        .unwrap_or(false);

    let csrf_secret = std::env::var("CSRF_SECRET")
        .or_else(|_| std::env::var("JWT_SECRET"))
        .unwrap_or_else(|_| {
            if is_production {
                tracing::error!("CSRF_SECRET not configured in production environment! Using insecure default. Please set CSRF_SECRET or JWT_SECRET environment variable.");
            } else {
                tracing::warn!("CSRF_SECRET not configured, using insecure default. This should be set in production.");
            }
            "default-csrf-secret-change-in-production".to_string()
        });

    // Extract CSRF token from header
    let header_token = request
        .headers()
        .get("X-CSRF-Token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Extract CSRF token from cookie
    let cookie_token = request
        .headers()
        .get("Cookie")
        .and_then(|h| h.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find(|part| part.trim().starts_with("csrf-token="))
                .and_then(|part| part.split('=').nth(1))
                .map(|s| s.trim().to_string())
        });

    // Validate CSRF token using double-submit cookie pattern
    // Both header and cookie must be present and match
    let is_valid = if let (Some(header), Some(cookie)) = (header_token, cookie_token) {
        // Both tokens must match (double-submit pattern)
        header == cookie && verify_csrf_token(&header, &csrf_secret)
    } else {
        false
    };

    if !is_valid {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "CSRF token validation failed",
                "error_type": "CsrfValidationError",
                "message": "Missing or invalid CSRF token. Include X-CSRF-Token header matching the csrf-token cookie."
            })),
        )
            .into_response();
    }

    next.run(request).await
}

/// Generate and return a CSRF token endpoint handler
pub async fn get_csrf_token() -> impl IntoResponse {
    // Get CSRF secret from environment (fallback to JWT_SECRET if CSRF_SECRET not set)
    let is_production = std::env::var("ENVIRONMENT")
        .map(|e| e.to_lowercase() == "production" || e.to_lowercase() == "prod")
        .unwrap_or(false);

    let csrf_secret = std::env::var("CSRF_SECRET")
        .or_else(|_| std::env::var("JWT_SECRET"))
        .unwrap_or_else(|_| {
            if is_production {
                tracing::error!("CSRF_SECRET not configured in production environment! Using insecure default. Please set CSRF_SECRET or JWT_SECRET environment variable.");
            } else {
                tracing::warn!("CSRF_SECRET not configured, using insecure default. This should be set in production.");
            }
            "default-csrf-secret-change-in-production".to_string()
        });

    let token = generate_csrf_token(&csrf_secret);

    // Return token in both JSON response and Set-Cookie header
    let response = axum::Json(CsrfTokenResponse {
        token: token.clone(),
    });

    // Set cookie with SameSite=Strict and HttpOnly for additional security
    // Note: Secure flag should be set in production (HTTPS only)
    let is_production = std::env::var("ENVIRONMENT")
        .map(|e| e.to_lowercase() == "production" || e.to_lowercase() == "prod")
        .unwrap_or(false);

    let secure_flag = if is_production { "; Secure" } else { "" };
    let cookie_value = format!(
        "csrf-token={}; Path=/; SameSite=Strict; HttpOnly{}",
        token, secure_flag
    );

    (
        axum::http::StatusCode::OK,
        [("Set-Cookie", cookie_value)],
        response,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_csrf_token() {
        let secret = "test-secret";
        let token = generate_csrf_token(secret);

        // Token should have 3 parts: hmac.timestamp.nonce
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        // HMAC should be hex-encoded (64 chars for SHA256)
        assert_eq!(parts[0].len(), 64);

        // Timestamp should be parseable
        let timestamp: u64 = parts[1].parse().expect("Timestamp should be parseable");
        assert!(timestamp > 0);
    }

    #[test]
    fn test_verify_csrf_token_valid() {
        let secret = "test-secret";
        let token = generate_csrf_token(secret);

        assert!(verify_csrf_token(&token, secret));
    }

    #[test]
    fn test_verify_csrf_token_invalid_secret() {
        let secret = "test-secret";
        let token = generate_csrf_token(secret);

        // Should fail with different secret
        assert!(!verify_csrf_token(&token, "different-secret"));
    }

    #[test]
    fn test_verify_csrf_token_invalid_format() {
        let secret = "test-secret";

        // Invalid format - only 2 parts
        assert!(!verify_csrf_token("abc.123", secret));

        // Invalid format - no parts
        assert!(!verify_csrf_token("invalid", secret));
    }

    #[test]
    fn test_verify_csrf_token_expired() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        use std::time::{SystemTime, UNIX_EPOCH};

        let secret = "test-secret";
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - CSRF_TOKEN_EXPIRATION_SECS
            - 1; // 1 second past expiration

        let nonce = Uuid::new_v4().to_string();
        let message = format!("{}.{}", timestamp, nonce);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let result = mac.finalize();
        let hmac = hex::encode(result.into_bytes());
        let token = format!("{}.{}.{}", hmac, timestamp, nonce);

        // Should fail because token is expired
        assert!(!verify_csrf_token(&token, secret));
    }
}
