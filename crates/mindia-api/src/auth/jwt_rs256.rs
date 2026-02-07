//! RS256/ES256 JWT support with JWKS key rotation
//!
//! This module provides JWT validation using asymmetric algorithms (RS256/ES256)
//! with support for key rotation via JWKS (JSON Web Key Set) endpoints.

use crate::auth::models::{JwtClaims, UserRole};
use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use mindia_core::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// JWKS (JSON Web Key Set) structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// JSON Web Key structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    #[serde(rename = "kty")]
    pub key_type: String,
    #[serde(rename = "kid")]
    pub key_id: Option<String>,
    #[serde(rename = "use")]
    pub key_use: Option<String>,
    #[serde(rename = "alg")]
    pub algorithm: Option<String>,
    #[serde(rename = "n")]
    pub modulus: Option<String>, // For RSA
    #[serde(rename = "e")]
    pub exponent: Option<String>, // For RSA
    #[serde(rename = "x")]
    pub x_coordinate: Option<String>, // For EC
    #[serde(rename = "y")]
    pub y_coordinate: Option<String>, // For EC
    #[serde(rename = "crv")]
    pub curve: Option<String>, // For EC
}

/// Cached public key with expiration
#[derive(Clone)]
struct CachedKey {
    key: DecodingKey,
    expires_at: DateTime<Utc>,
}

/// JWT service with RS256/ES256 support and JWKS key rotation
pub struct JwtServiceRs256 {
    jwks_url: String,
    cache: Arc<RwLock<HashMap<String, CachedKey>>>,
    cache_ttl_seconds: i64,
    algorithms: Vec<Algorithm>,
}

impl JwtServiceRs256 {
    /// Create a new JWT service with JWKS URL
    ///
    /// # Arguments
    /// * `jwks_url` - URL to fetch JWKS (e.g., "https://your-auth-domain/.well-known/jwks.json")
    /// * `cache_ttl_seconds` - How long to cache keys (default: 3600 = 1 hour)
    pub fn new(jwks_url: String, cache_ttl_seconds: Option<i64>) -> Self {
        Self {
            jwks_url,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl_seconds: cache_ttl_seconds.unwrap_or(3600),
            algorithms: vec![Algorithm::RS256, Algorithm::ES256],
        }
    }

    /// Fetch JWKS from the configured URL
    async fn fetch_jwks(&self) -> Result<Jwks, AppError> {
        let response = reqwest::get(&self.jwks_url)
            .await
            .map_err(|e| AppError::Unauthorized(format!("Failed to fetch JWKS: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Unauthorized(format!(
                "JWKS endpoint returned error: {}",
                response.status()
            )));
        }

        let jwks: Jwks = response
            .json()
            .await
            .map_err(|e| AppError::Unauthorized(format!("Failed to parse JWKS: {}", e)))?;

        Ok(jwks)
    }

    /// Convert JWK to DecodingKey
    fn jwk_to_decoding_key(&self, jwk: &Jwk) -> Result<DecodingKey, AppError> {
        match jwk.key_type.as_str() {
            "RSA" => {
                let n = jwk
                    .modulus
                    .as_ref()
                    .ok_or_else(|| AppError::Unauthorized("RSA key missing modulus".to_string()))?;
                let e = jwk.exponent.as_ref().ok_or_else(|| {
                    AppError::Unauthorized("RSA key missing exponent".to_string())
                })?;

                // Use jsonwebtoken's built-in RSA support which handles base64url decoding
                DecodingKey::from_rsa_components(n, e)
                    .map_err(|e| AppError::Unauthorized(format!("Failed to create RSA key: {}", e)))
            }
            "EC" => {
                let x = jwk.x_coordinate.as_ref().ok_or_else(|| {
                    AppError::Unauthorized("EC key missing x coordinate".to_string())
                })?;
                let y = jwk.y_coordinate.as_ref().ok_or_else(|| {
                    AppError::Unauthorized("EC key missing y coordinate".to_string())
                })?;
                let curve = jwk
                    .curve
                    .as_ref()
                    .ok_or_else(|| AppError::Unauthorized("EC key missing curve".to_string()))?;

                // For ES256, we need P-256 curve
                if curve != "P-256" {
                    return Err(AppError::Unauthorized(format!(
                        "Unsupported EC curve: {} (only P-256 is supported)",
                        curve
                    )));
                }

                // Use jsonwebtoken's built-in EC support which handles base64url decoding
                DecodingKey::from_ec_components(x, y)
                    .map_err(|e| AppError::Unauthorized(format!("Failed to create EC key: {}", e)))
            }
            _ => Err(AppError::Unauthorized(format!(
                "Unsupported key type: {}",
                jwk.key_type
            ))),
        }
    }

    /// Get decoding key for a given key ID, with caching
    async fn get_decoding_key(&self, kid: Option<&str>) -> Result<DecodingKey, AppError> {
        let cache_key = kid.unwrap_or("default").to_string();

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.expires_at > Utc::now() {
                    return Ok(cached.key.clone());
                }
            }
        }

        // Cache miss or expired - fetch fresh JWKS
        let jwks = self.fetch_jwks().await?;

        // Find the key by kid, or use the first key if no kid specified
        let jwk = if let Some(kid) = kid {
            jwks.keys
                .iter()
                .find(|k| k.key_id.as_ref().map(|k| k == kid).unwrap_or(false))
                .ok_or_else(|| {
                    AppError::Unauthorized(format!("Key ID {} not found in JWKS", kid))
                })?
        } else {
            jwks.keys
                .first()
                .ok_or_else(|| AppError::Unauthorized("No keys found in JWKS".to_string()))?
        };

        // Convert JWK to DecodingKey
        let decoding_key = self.jwk_to_decoding_key(jwk)?;

        // Cache the key
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                CachedKey {
                    key: decoding_key.clone(),
                    expires_at: Utc::now() + chrono::Duration::seconds(self.cache_ttl_seconds),
                },
            );
        }

        Ok(decoding_key)
    }

    /// Validate and decode a JWT token using RS256/ES256
    pub async fn validate_token(&self, token: &str) -> Result<JwtClaims, AppError> {
        // Decode header to get kid and alg
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| AppError::Unauthorized(format!("Invalid token header: {}", e)))?;

        // Get the decoding key based on kid
        let decoding_key = self.get_decoding_key(header.kid.as_deref()).await?;

        // Determine algorithm from header or use RS256 as default
        let algorithm = header.alg;
        if !self.algorithms.contains(&algorithm) {
            return Err(AppError::Unauthorized(format!(
                "Unsupported algorithm: {:?}. Supported: {:?}",
                algorithm, self.algorithms
            )));
        }

        // Create validation with strict settings
        let mut validation = Validation::new(algorithm);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 0;
        validation.algorithms = self.algorithms.clone();

        let token_data = decode::<JwtClaims>(token, &decoding_key, &validation).map_err(|e| {
            tracing::debug!("JWT validation failed: {}", e);
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AppError::Unauthorized("Token has expired".to_string())
                }
                jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                    AppError::Unauthorized("Invalid token issuer".to_string())
                }
                jsonwebtoken::errors::ErrorKind::InvalidSubject => {
                    AppError::Unauthorized("Invalid token subject".to_string())
                }
                jsonwebtoken::errors::ErrorKind::ImmatureSignature => {
                    AppError::Unauthorized("Token is not yet valid (nbf)".to_string())
                }
                _ => AppError::Unauthorized(format!("Invalid or expired token: {}", e)),
            }
        })?;

        Ok(token_data.claims)
    }

    /// Parse user role from string
    pub fn parse_role(role_str: &str) -> Result<UserRole, AppError> {
        match role_str {
            "admin" => Ok(UserRole::Admin),
            "member" => Ok(UserRole::Member),
            "viewer" => Ok(UserRole::Viewer),
            _ => Err(AppError::Unauthorized("Invalid user role".to_string())),
        }
    }
}
