//! Metadata validation module
//!
//! Provides validation for user metadata:
//! - Key validation: regex pattern, max length, reserved prefixes
//! - Value validation: max length (JSON serialized)
//! - Key count limits
//! - Reserved key detection

use anyhow::{Context, Result};
use regex::Regex;
use serde_json;

/// Maximum length for metadata key names (64 characters)
pub const MAX_METADATA_KEY_LENGTH: usize = 64;

/// Maximum length for metadata values (512 characters when JSON serialized)
pub const MAX_METADATA_VALUE_LENGTH: usize = 512;

/// Maximum number of keys allowed in user metadata namespace (50 keys)
pub const MAX_USER_METADATA_KEYS: usize = 50;

/// Reserved key prefixes that users cannot use
const RESERVED_PREFIXES: &[&str] = &["_plugin_", "_system_", "_internal_", "plugin_", "plugins"];

/// Validate a metadata key name
///
/// Rules:
/// - Must match pattern: `^[a-zA-Z0-9_\\-\\.:]+$`
/// - Maximum 64 characters
/// - Cannot start with reserved prefixes
pub fn validate_metadata_key(key: &str) -> Result<()> {
    // Check length
    if key.len() > MAX_METADATA_KEY_LENGTH {
        return Err(anyhow::anyhow!(
            "Metadata key '{}' exceeds maximum length of {} characters",
            key,
            MAX_METADATA_KEY_LENGTH
        ));
    }

    // Check for empty key
    if key.is_empty() {
        return Err(anyhow::anyhow!("Metadata key cannot be empty"));
    }

    // Validate pattern: a-z, A-Z, 0-9, underscore, hyphen, dot, colon
    let pattern = Regex::new(r"^[a-zA-Z0-9_\-\.:]+$")
        .context("Failed to compile metadata key validation regex")?;

    if !pattern.is_match(key) {
        return Err(anyhow::anyhow!(
            "Metadata key '{}' contains invalid characters. Allowed: letters (a-z, A-Z), digits (0-9), underscore (_), hyphen (-), dot (.), colon (:)",
            key
        ));
    }

    // Check reserved prefixes
    if is_reserved_key(key) {
        return Err(anyhow::anyhow!(
            "Metadata key '{}' uses a reserved prefix. Reserved prefixes: {:?}",
            key,
            RESERVED_PREFIXES
        ));
    }

    Ok(())
}

/// Check if a key is reserved (starts with reserved prefix)
pub fn is_reserved_key(key: &str) -> bool {
    RESERVED_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
}

/// Validate a metadata value
///
/// Rules:
/// - When JSON serialized, must not exceed 512 characters
/// - Must be a valid JSON value (string, number, boolean, null, object, array)
pub fn validate_metadata_value(value: &serde_json::Value) -> Result<()> {
    // Serialize to check length
    let serialized = serde_json::to_string(value).context("Failed to serialize metadata value")?;

    if serialized.len() > MAX_METADATA_VALUE_LENGTH {
        return Err(anyhow::anyhow!(
            "Metadata value exceeds maximum length of {} characters when serialized",
            MAX_METADATA_VALUE_LENGTH
        ));
    }

    Ok(())
}

/// Validate user metadata object
///
/// Validates:
/// - All keys are valid (pattern, length, reserved prefixes)
/// - All values are valid (length when serialized)
/// - Total key count doesn't exceed limit
pub fn validate_user_metadata(metadata: &serde_json::Value) -> Result<()> {
    // Must be an object
    let obj = metadata
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("User metadata must be a JSON object"))?;

    // Check key count
    if obj.len() > MAX_USER_METADATA_KEYS {
        return Err(anyhow::anyhow!(
            "User metadata contains {} keys, but maximum allowed is {}",
            obj.len(),
            MAX_USER_METADATA_KEYS
        ));
    }

    // Validate each key-value pair
    for (key, value) in obj.iter() {
        validate_metadata_key(key).with_context(|| format!("Invalid metadata key: '{}'", key))?;

        validate_metadata_value(value)
            .with_context(|| format!("Invalid metadata value for key '{}'", key))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_metadata_key_valid() {
        assert!(validate_metadata_key("userId").is_ok());
        assert!(validate_metadata_key("user_id").is_ok());
        assert!(validate_metadata_key("user-id").is_ok());
        assert!(validate_metadata_key("user.id").is_ok());
        assert!(validate_metadata_key("user:type").is_ok());
        assert!(validate_metadata_key("ABC123").is_ok());
        assert!(validate_metadata_key("a").is_ok());
    }

    #[test]
    fn test_validate_metadata_key_invalid_characters() {
        assert!(validate_metadata_key("user id").is_err()); // space
        assert!(validate_metadata_key("user@id").is_err()); // @
        assert!(validate_metadata_key("user#id").is_err()); // #
        assert!(validate_metadata_key("user$id").is_err()); // $
        assert!(validate_metadata_key("user%id").is_err()); // %
        assert!(validate_metadata_key("user/id").is_err()); // /
    }

    #[test]
    fn test_validate_metadata_key_too_long() {
        let long_key = "a".repeat(MAX_METADATA_KEY_LENGTH + 1);
        assert!(validate_metadata_key(&long_key).is_err());
    }

    #[test]
    fn test_validate_metadata_key_empty() {
        assert!(validate_metadata_key("").is_err());
    }

    #[test]
    fn test_is_reserved_key() {
        assert!(is_reserved_key("_plugin_test"));
        assert!(is_reserved_key("_system_info"));
        assert!(is_reserved_key("_internal_data"));
        assert!(is_reserved_key("plugin_name"));
        assert!(is_reserved_key("plugins_data"));
        assert!(!is_reserved_key("user_id"));
        assert!(!is_reserved_key("normal_key"));
    }

    #[test]
    fn test_validate_metadata_key_reserved() {
        assert!(validate_metadata_key("_plugin_test").is_err());
        assert!(validate_metadata_key("_system_info").is_err());
        assert!(validate_metadata_key("plugin_name").is_err());
    }

    #[test]
    fn test_validate_metadata_value_valid() {
        assert!(validate_metadata_value(&serde_json::json!("simple string")).is_ok());
        assert!(validate_metadata_value(&serde_json::json!(123)).is_ok());
        assert!(validate_metadata_value(&serde_json::json!(true)).is_ok());
        assert!(validate_metadata_value(&serde_json::json!(null)).is_ok());
        assert!(validate_metadata_value(&serde_json::json!({"nested": "object"})).is_ok());
        assert!(validate_metadata_value(&serde_json::json!(["array", "values"])).is_ok());
    }

    #[test]
    fn test_validate_metadata_value_too_long() {
        let long_string = "a".repeat(MAX_METADATA_VALUE_LENGTH + 1);
        let value = serde_json::json!(long_string);
        assert!(validate_metadata_value(&value).is_err());
    }

    #[test]
    fn test_validate_user_metadata_valid() {
        let metadata = serde_json::json!({
            "userId": "123",
            "type": "avatar",
            "category": "profile"
        });
        assert!(validate_user_metadata(&metadata).is_ok());
    }

    #[test]
    fn test_validate_user_metadata_too_many_keys() {
        let mut metadata_obj = serde_json::Map::new();
        for i in 0..=MAX_USER_METADATA_KEYS {
            metadata_obj.insert(format!("key_{}", i), serde_json::json!("value"));
        }
        let metadata = serde_json::Value::Object(metadata_obj);
        assert!(validate_user_metadata(&metadata).is_err());
    }

    #[test]
    fn test_validate_user_metadata_invalid_key() {
        let metadata = serde_json::json!({
            "user id": "123"  // space in key
        });
        assert!(validate_user_metadata(&metadata).is_err());
    }

    #[test]
    fn test_validate_user_metadata_not_object() {
        assert!(validate_user_metadata(&serde_json::json!("not an object")).is_err());
        assert!(validate_user_metadata(&serde_json::json!(123)).is_err());
        assert!(validate_user_metadata(&serde_json::json!([])).is_err());
    }
}
