//! API key types and helpers (create/list/verify API keys).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// API Key stored in database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub key_hash: String,
    pub key_prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new API key
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    /// Human-readable name for the API key
    #[schema(example = "Production API Key")]
    pub name: String,

    /// Optional description
    #[schema(example = "Used for production media uploads")]
    pub description: Option<String>,

    /// Optional expiration time (in days from now)
    #[schema(example = 365)]
    pub expires_in_days: Option<i64>,
}

/// Response when creating an API key (includes the raw key - only shown once)
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    /// The API key ID
    pub id: Uuid,

    /// The full API key - save this securely, it won't be shown again
    #[schema(example = "mk_live_abc123def456ghi789jkl012mno345pqr678")]
    pub api_key: String,

    /// Human-readable name
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Key prefix for identification
    #[schema(example = "mk_live_abc123")]
    pub key_prefix: String,

    /// Optional expiration date
    pub expires_at: Option<DateTime<Utc>>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// API Key information (without the secret key)
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub key_prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            description: key.description,
            key_prefix: key.key_prefix,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
            is_active: key.is_active,
            created_at: key.created_at,
        }
    }
}

/// Generate a secure API key
pub fn generate_api_key() -> String {
    use rand::Rng;

    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..20).map(|_| rng.random()).collect();
    let random_part = hex::encode(random_bytes);

    // Format: mk_live_<40 hex chars>
    format!("mk_live_{}", random_part)
}

/// Hash an API key for storage
pub fn hash_api_key(key: &str) -> Result<String, mindia_core::AppError> {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };

    use rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(key.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| mindia_core::AppError::Internal(format!("Failed to hash API key: {}", e)))
}

/// Verify an API key against a hash (for API key management).
pub fn verify_api_key(key: &str, hash: &str) -> Result<bool, mindia_core::AppError> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| mindia_core::AppError::Internal(format!("Invalid hash format: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(key.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Extract the key prefix (first 16 chars) for identification.
pub fn extract_key_prefix(key: &str) -> String {
    if key.len() > 16 {
        key[..16].to_string()
    } else {
        key.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();
        assert!(key.starts_with("mk_live_"));
        assert_eq!(key.len(), 48); // "mk_live_" (8) + 64 hex chars / 2 * 2 = 40
    }

    #[test]
    fn test_hash_and_verify_api_key() {
        let key = generate_api_key();
        let hash = hash_api_key(&key).unwrap();

        assert!(verify_api_key(&key, &hash).unwrap());
        assert!(!verify_api_key("wrong_key", &hash).unwrap());
    }

    #[test]
    fn test_extract_key_prefix() {
        let key = "mk_live_abc123def456";
        let prefix = extract_key_prefix(key);
        assert_eq!(prefix, "mk_live_abc123de");
        assert_eq!(prefix.len(), 16);
    }
}
