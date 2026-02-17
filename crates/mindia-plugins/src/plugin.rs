//! Plugin system core infrastructure
//!
//! This module provides the abstraction layer for plugins, keeping plugin
//! implementations separate from the core system.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use mindia_db::{PluginFileGroupRepository, PluginMediaRepository};
use mindia_storage::Storage;

/// Context provided to plugins during execution
///
/// Plugins should only use the provided repositories and services, which
/// ensure tenant isolation and authorized operations. Direct database access
/// is not provided to maintain security and data integrity.
///
/// # Security Note
///
/// The `config` field may contain sensitive information such as API keys.
/// Plugins MUST NOT log the config value or any sensitive fields within it.
/// When logging configuration, redact API keys and other credentials.
#[derive(Clone)]
pub struct PluginContext {
    /// Tenant ID for the plugin execution
    pub tenant_id: Uuid,
    /// Media ID of the file being processed (e.g., audio file for transcription)
    pub media_id: Uuid,
    /// Storage service for file operations
    pub storage: Arc<dyn Storage>,
    /// Media repository for database operations (with tenant isolation)
    pub media_repo: Arc<dyn PluginMediaRepository>,
    /// File group repository for associating files
    pub file_group_repo: Arc<dyn PluginFileGroupRepository>,
    /// Plugin-specific configuration (from plugin_configs table)
    ///
    /// WARNING: May contain sensitive data (API keys, credentials).
    /// Never log this field directly.
    pub config: serde_json::Value,
}

/// Usage data extracted from cloud provider API responses during plugin execution.
/// Used for cost tracking and analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUsage {
    /// Usage unit type (tokens, seconds, images, features, etc.)
    pub unit_type: String,
    /// Input units consumed (e.g., input_tokens, audio_seconds)
    pub input_units: Option<i64>,
    /// Output units consumed (e.g., output_tokens)
    pub output_units: Option<i64>,
    /// Total units consumed
    pub total_units: i64,
    /// Raw usage data from provider (for provider-specific details)
    pub raw_usage: Option<serde_json::Value>,
}

/// Result returned by plugin execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    /// Plugin execution status
    pub status: PluginExecutionStatus,
    /// Output data from plugin (e.g., transcript JSON)
    pub data: serde_json::Value,
    /// Optional error message if execution failed
    pub error: Option<String>,
    /// Optional metadata about the execution
    pub metadata: Option<serde_json::Value>,
    /// Optional usage data from provider API (for cost tracking)
    pub usage: Option<PluginUsage>,
}

/// Plugin execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginExecutionStatus {
    /// Plugin execution completed successfully
    Success,
    /// Plugin execution failed
    Failed,
}

/// Trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync + Debug {
    /// Get the plugin name/identifier
    fn name(&self) -> &str;

    /// Execute the plugin with the given context
    async fn execute(&self, context: PluginContext) -> Result<PluginResult>;

    /// Validate plugin configuration
    fn validate_config(&self, config: &serde_json::Value) -> Result<()>;
}

/// Plugin information for listing available plugins
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    /// Plugin name/identifier
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Supported media types (e.g., ["audio"])
    pub supported_media_types: Vec<String>,
}
