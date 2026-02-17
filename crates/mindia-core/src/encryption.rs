//! Encryption service for sensitive data (API keys, tokens, credentials)

use crate::AppError;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use std::env;

/// Encryption service for sensitive data (OAuth tokens, API keys, plugin configs)
/// Uses AES-256-GCM for authenticated encryption
#[derive(Clone)]
pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    /// Create a new encryption service from raw 32-byte key (e.g. for tests; avoids env mutation).
    pub fn from_key_bytes(key_bytes: &[u8]) -> Result<Self, AppError> {
        if key_bytes.len() != 32 {
            return Err(AppError::Internal(
                "Encryption key must be 32 bytes (256 bits)".to_string(),
            ));
        }
        let key = Key::<Aes256Gcm>::from_slice(key_bytes);
        Ok(Self {
            cipher: Aes256Gcm::new(key),
        })
    }

    /// Create a new encryption service from environment variable
    /// Expects ENCRYPTION_KEY to be a base64-encoded 32-byte key
    pub fn new() -> Result<Self, AppError> {
        let key_str = env::var("ENCRYPTION_KEY").map_err(|_| {
            AppError::Internal("ENCRYPTION_KEY environment variable not set".to_string())
        })?;

        let key_bytes = general_purpose::STANDARD
            .decode(&key_str)
            .map_err(|e| AppError::Internal(format!("Failed to decode encryption key: {}", e)))?;

        Self::from_key_bytes(&key_bytes)
    }

    /// Encrypt a plaintext string
    pub fn encrypt(&self, plaintext: &str) -> Result<String, AppError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

        // Combine nonce and ciphertext, then base64 encode
        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);
        let encoded = general_purpose::STANDARD.encode(&combined);

        Ok(encoded)
    }

    /// Decrypt an encrypted string
    pub fn decrypt(&self, encrypted: &str) -> Result<String, AppError> {
        let combined = general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| AppError::Internal(format!("Failed to decode encrypted data: {}", e)))?;

        if combined.len() < 12 {
            return Err(AppError::Internal("Encrypted data too short".to_string()));
        }

        // Extract nonce (first 12 bytes) and ciphertext (rest)
        let nonce = Nonce::from_slice(&combined[..12]);
        let ciphertext = &combined[12..];

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::Internal(format!("Invalid UTF-8 in decrypted data: {}", e)))
    }

    /// Encrypt sensitive fields in a JSON object.
    /// Sensitive keys are detected by name (best-effort): any key whose lowercase form contains
    /// `api_key`, `secret`, `token`, `password`, `credential`, or `private_key`. Add new
    /// substrings here if you introduce additional sensitive key names.
    pub fn encrypt_sensitive_json(
        &self,
        json: &serde_json::Value,
    ) -> Result<(serde_json::Value, Option<String>), AppError> {
        if !json.is_object() {
            return Ok((json.clone(), None));
        }

        let mut public_config = serde_json::Map::new();
        let mut sensitive_config = serde_json::Map::new();
        let mut has_sensitive = false;

        if let Some(obj) = json.as_object() {
            for (key, value) in obj {
                let key_lower = key.to_lowercase();
                let is_sensitive = key_lower.contains("api_key")
                    || key_lower.contains("secret")
                    || key_lower.contains("token")
                    || key_lower.contains("password")
                    || key_lower.contains("credential")
                    || key_lower.contains("private_key");

                if is_sensitive && value.is_string() {
                    sensitive_config.insert(key.clone(), value.clone());
                    has_sensitive = true;
                } else {
                    public_config.insert(key.clone(), value.clone());
                }
            }
        }

        let encrypted_data = if has_sensitive {
            let sensitive_json = serde_json::to_string(&sensitive_config).map_err(|e| {
                AppError::Internal(format!("Failed to serialize sensitive config: {}", e))
            })?;
            Some(self.encrypt(&sensitive_json)?)
        } else {
            None
        };

        Ok((serde_json::Value::Object(public_config), encrypted_data))
    }

    /// Decrypt and merge sensitive fields back into public config
    pub fn decrypt_and_merge_json(
        &self,
        public_json: &serde_json::Value,
        encrypted_data: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        let encrypted_data = match encrypted_data {
            Some(data) if !data.is_empty() => data,
            _ => return Ok(public_json.clone()),
        };

        let decrypted_str = self.decrypt(encrypted_data)?;
        let sensitive_config: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&decrypted_str).map_err(|e| {
                AppError::Internal(format!("Failed to parse decrypted config: {}", e))
            })?;

        let mut merged = public_json.clone();
        if let Some(obj) = merged.as_object_mut() {
            for (key, value) in sensitive_config {
                obj.insert(key, value);
            }
        }

        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> EncryptionService {
        let test_key = b"01234567890123456789012345678901";
        EncryptionService::from_key_bytes(test_key).unwrap()
    }

    #[test]
    fn test_encryption_decryption() {
        let service = test_service();
        let plaintext = "test_token_12345";

        let encrypted = service.encrypt(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = service.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_sensitive_json() {
        let service = test_service();

        let config = serde_json::json!({
            "api_key": "secret_key_123",
            "region": "us-east-1",
            "timeout": 30,
            "client_secret": "another_secret"
        });

        let (public, encrypted) = service.encrypt_sensitive_json(&config).unwrap();

        assert!(public.get("api_key").is_none());
        assert!(public.get("client_secret").is_none());

        assert_eq!(public.get("region").unwrap(), "us-east-1");
        assert_eq!(public.get("timeout").unwrap(), 30);

        assert!(encrypted.is_some());
    }

    #[test]
    fn test_decrypt_and_merge_json() {
        let service = test_service();

        let config = serde_json::json!({
            "api_key": "secret_key_123",
            "region": "us-east-1",
            "timeout": 30
        });

        let (public, encrypted) = service.encrypt_sensitive_json(&config).unwrap();

        // Decrypt and merge back
        let merged = service
            .decrypt_and_merge_json(&public, encrypted.as_deref())
            .unwrap();

        assert_eq!(merged.get("api_key").unwrap(), "secret_key_123");
        assert_eq!(merged.get("region").unwrap(), "us-east-1");
        assert_eq!(merged.get("timeout").unwrap(), 30);
    }

    #[test]
    fn test_encrypt_json_no_sensitive_fields() {
        let service = test_service();

        let config = serde_json::json!({
            "region": "us-east-1",
            "timeout": 30
        });

        let (public, encrypted) = service.encrypt_sensitive_json(&config).unwrap();

        // No sensitive fields, so no encryption
        assert!(encrypted.is_none());
        assert_eq!(public, config);
    }
}
