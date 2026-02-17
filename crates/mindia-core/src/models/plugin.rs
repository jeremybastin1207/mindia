//! Plugin models for database and API

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[cfg(feature = "sqlx")]
use sqlx::FromRow;

/// Plugin execution status (matches database enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "plugin_execution_status", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum PluginExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Plugin configuration model (database representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct PluginConfig {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub plugin_name: String,
    pub enabled: bool,
    /// Public (non-sensitive) configuration
    pub config: serde_json::Value,
    /// Encrypted sensitive configuration (API keys, secrets, tokens)
    pub encrypted_config: Option<String>,
    /// Whether this config uses encryption
    pub uses_encryption: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Plugin execution tracking model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct PluginExecution {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub plugin_name: String,
    pub media_id: Uuid,
    pub task_id: Option<Uuid>,
    pub status: PluginExecutionStatus,
    pub result: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Usage unit type (tokens, seconds, images, etc.)
    pub usage_unit_type: Option<String>,
    /// Input units consumed
    pub usage_input_units: Option<i64>,
    /// Output units consumed
    pub usage_output_units: Option<i64>,
    /// Total units consumed
    pub usage_total_units: Option<i64>,
    /// Raw usage data from provider
    pub usage_raw: Option<serde_json::Value>,
}

/// Plugin cost summary (aggregated usage per tenant/plugin/period)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct PluginCostSummary {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub plugin_name: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub execution_count: i64,
    pub total_units: i64,
    pub unit_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response for plugin cost summary
#[derive(Debug, Serialize, ToSchema)]
pub struct PluginCostSummaryResponse {
    pub plugin_name: String,
    pub execution_count: i64,
    pub total_units: i64,
    pub unit_type: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Response for plugin costs list
#[derive(Debug, Serialize, ToSchema)]
pub struct PluginCostsResponse {
    pub costs: Vec<PluginCostSummaryResponse>,
    pub total_executions: i64,
}

/// Request to create or update plugin configuration
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePluginConfigRequest {
    /// Whether the plugin is enabled for this tenant
    pub enabled: bool,
    /// Plugin-specific configuration (API keys, settings, etc.)
    pub config: serde_json::Value,
}

/// Response for plugin configuration
#[derive(Debug, Serialize, ToSchema)]
pub struct PluginConfigResponse {
    pub id: Uuid,
    pub plugin_name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PluginConfig {
    /// Get the full decrypted configuration (merges public and encrypted fields)
    /// Requires an EncryptionService to decrypt sensitive fields
    pub fn get_full_config(
        &self,
        encryption_service: &crate::EncryptionService,
    ) -> Result<serde_json::Value, crate::AppError> {
        if !self.uses_encryption || self.encrypted_config.is_none() {
            return Ok(self.config.clone());
        }

        encryption_service.decrypt_and_merge_json(&self.config, self.encrypted_config.as_deref())
    }

    /// Redact sensitive fields for API responses (mask api_key, secrets, tokens, etc.)
    /// Returns a copy of the config with sensitive string values masked to show only first few characters
    pub fn redact_sensitive_fields(config: &serde_json::Value) -> serde_json::Value {
        if !config.is_object() {
            return config.clone();
        }

        let mut redacted = serde_json::Map::new();

        if let Some(obj) = config.as_object() {
            for (key, value) in obj {
                let key_lower = key.to_lowercase();
                let is_sensitive = key_lower.contains("api_key")
                    || key_lower.contains("secret")
                    || key_lower.contains("token")
                    || key_lower.contains("password")
                    || key_lower.contains("credential")
                    || key_lower.contains("private_key");

                if is_sensitive && value.is_string() {
                    if let Some(s) = value.as_str() {
                        // Mask the value - show first few chars + ***
                        let masked = if s.len() > 10 {
                            format!("{}***", &s[..7])
                        } else if s.len() > 4 {
                            format!("{}***", &s[..3])
                        } else {
                            "***".to_string()
                        };
                        redacted.insert(key.clone(), serde_json::Value::String(masked));
                    }
                } else {
                    redacted.insert(key.clone(), value.clone());
                }
            }
        }

        serde_json::Value::Object(redacted)
    }
}

impl From<PluginConfig> for PluginConfigResponse {
    fn from(config: PluginConfig) -> Self {
        Self {
            id: config.id,
            plugin_name: config.plugin_name,
            enabled: config.enabled,
            config: PluginConfig::redact_sensitive_fields(&config.config),
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

/// Request to execute a plugin
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecutePluginRequest {
    /// Media ID to process (e.g., audio file for transcription)
    pub media_id: Uuid,
}

/// Response for plugin execution
#[derive(Debug, Serialize, ToSchema)]
pub struct ExecutePluginResponse {
    /// Task ID for tracking the async execution
    pub task_id: Uuid,
    /// Initial status
    pub status: String,
}

/// Plugin information response
#[derive(Debug, Serialize, ToSchema)]
pub struct PluginInfoResponse {
    pub name: String,
    pub description: String,
    pub supported_media_types: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_config_to_response() {
        let config = PluginConfig {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            plugin_name: "test_plugin".to_string(),
            enabled: true,
            config: serde_json::json!({"api_key": "secret"}),
            encrypted_config: None,
            uses_encryption: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: PluginConfigResponse = config.clone().into();

        assert_eq!(response.id, config.id);
        assert_eq!(response.plugin_name, config.plugin_name);
        assert_eq!(response.enabled, config.enabled);
        // Response config is redacted (sensitive fields like api_key are masked)
        assert_eq!(
            response.config,
            PluginConfig::redact_sensitive_fields(&config.config)
        );
        assert_eq!(response.created_at, config.created_at);
        assert_eq!(response.updated_at, config.updated_at);
    }

    #[test]
    fn test_plugin_config_to_response_disabled() {
        let config = PluginConfig {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            plugin_name: "disabled_plugin".to_string(),
            enabled: false,
            config: serde_json::json!({}),
            encrypted_config: None,
            uses_encryption: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: PluginConfigResponse = config.into();

        assert!(!response.enabled);
    }

    #[test]
    fn test_redact_sensitive_fields() {
        // Test with API key
        let config = serde_json::json!({
            "api_key": "sk-ant-test-key-for-redaction-testing",
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 2048
        });

        let redacted = PluginConfig::redact_sensitive_fields(&config);

        // API key should be masked
        assert_eq!(redacted["api_key"].as_str().unwrap(), "sk-ant-***");

        // Non-sensitive fields should remain unchanged
        assert_eq!(
            redacted["model"].as_str().unwrap(),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(redacted["max_tokens"].as_i64().unwrap(), 2048);
    }

    #[test]
    fn test_redact_multiple_sensitive_fields() {
        let config = serde_json::json!({
            "api_key": "test-key-12345",
            "secret": "my-secret-value",
            "token": "bearer-token-xyz",
            "password": "pass123",
            "normal_field": "not-sensitive"
        });

        let redacted = PluginConfig::redact_sensitive_fields(&config);

        // All sensitive fields should be masked
        assert_eq!(redacted["api_key"].as_str().unwrap(), "test-ke***");
        assert_eq!(redacted["secret"].as_str().unwrap(), "my-secr***");
        assert_eq!(redacted["token"].as_str().unwrap(), "bearer-***");
        assert_eq!(redacted["password"].as_str().unwrap(), "pas***");

        // Normal field unchanged
        assert_eq!(redacted["normal_field"].as_str().unwrap(), "not-sensitive");
    }

    #[test]
    fn test_redact_short_sensitive_value() {
        let config = serde_json::json!({
            "api_key": "abc"
        });

        let redacted = PluginConfig::redact_sensitive_fields(&config);

        // Short values get minimal masking
        assert_eq!(redacted["api_key"].as_str().unwrap(), "***");
    }
}
