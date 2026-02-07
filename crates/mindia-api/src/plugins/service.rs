//! Plugin service for orchestrating plugin execution

use anyhow::{Context, Result};
use std::sync::Arc;
use uuid::Uuid;

use mindia_core::models::{PluginExecutionPayload, Priority, TaskType};
use mindia_core::EncryptionService;
use mindia_db::{PluginConfigRepository, PluginExecutionRepository};
use mindia_worker::TaskQueue;

use crate::plugins::PluginRegistry;

/// Service for managing plugin execution
#[derive(Clone)]
pub struct PluginService {
    registry: Arc<PluginRegistry>,
    config_repo: PluginConfigRepository,
    execution_repo: PluginExecutionRepository,
    task_queue: TaskQueue,
    #[allow(dead_code)]
    encryption_service: EncryptionService,
}

impl PluginService {
    pub fn new_with_encryption(
        registry: Arc<PluginRegistry>,
        config_repo: PluginConfigRepository,
        execution_repo: PluginExecutionRepository,
        task_queue: TaskQueue,
        encryption_service: EncryptionService,
    ) -> Self {
        Self {
            registry,
            config_repo,
            execution_repo,
            task_queue,
            encryption_service,
        }
    }

    /// Execute a plugin on a media file (creates async task)
    #[tracing::instrument(skip(self), fields(tenant_id = %tenant_id, plugin_name = %plugin_name, media_id = %media_id))]
    pub async fn execute_plugin(
        &self,
        tenant_id: Uuid,
        plugin_name: &str,
        media_id: Uuid,
    ) -> Result<Uuid> {
        // Verify plugin exists
        self.registry
            .get(plugin_name)
            .await
            .context("Plugin not found")?;

        // Get plugin configuration
        let config = self
            .config_repo
            .get_config(tenant_id, plugin_name)
            .await
            .context("Failed to get plugin config")?;

        let plugin_config = config.context("Plugin not configured for tenant")?;

        if !plugin_config.enabled {
            return Err(anyhow::anyhow!("Plugin is not enabled for this tenant"));
        }

        // Create execution record
        let execution = self
            .execution_repo
            .create_execution(tenant_id, plugin_name, media_id, None)
            .await
            .context("Failed to create plugin execution record")?;

        // Submit task to queue
        let payload = PluginExecutionPayload {
            plugin_name: plugin_name.to_string(),
            media_id,
            tenant_id,
        };

        tracing::debug!(
            tenant_id = %tenant_id,
            plugin_name = %plugin_name,
            media_id = %media_id,
            execution_id = %execution.id,
            "Submitting plugin execution task to queue"
        );

        let task_id = self
            .task_queue
            .submit_task(
                tenant_id,
                TaskType::PluginExecution,
                serde_json::to_value(&payload)?,
                Priority::Normal,
                None,
                None,
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    tenant_id = %tenant_id,
                    plugin_name = %plugin_name,
                    media_id = %media_id,
                    execution_id = %execution.id,
                    "Failed to submit plugin execution task to queue"
                );

                // Clean up the orphaned execution record
                let execution_repo = self.execution_repo.clone();
                let execution_id = execution.id;
                tokio::spawn(async move {
                    if let Err(cleanup_err) = execution_repo.delete_execution(execution_id).await {
                        tracing::warn!(
                            execution_id = %execution_id,
                            error = %cleanup_err,
                            "Failed to clean up orphaned execution record after task creation failure"
                        );
                    } else {
                        tracing::info!(
                            execution_id = %execution_id,
                            "Cleaned up orphaned execution record after task creation failure"
                        );
                    }
                });

                anyhow::anyhow!("Failed to submit plugin execution task: {}", e)
            })?;

        // Update execution with task_id
        self.execution_repo
            .update_task_id(execution.id, task_id)
            .await
            .context("Failed to update execution with task_id")?;

        tracing::info!(
            task_id = %task_id,
            execution_id = %execution.id,
            "Plugin execution task submitted"
        );

        Ok(task_id)
    }

    /// Get plugin configuration for a tenant
    pub async fn get_plugin_config(
        &self,
        tenant_id: Uuid,
        plugin_name: &str,
    ) -> Result<Option<mindia_core::models::PluginConfig>> {
        self.config_repo
            .get_config(tenant_id, plugin_name)
            .await
            .context("Failed to get plugin config")
    }

    /// Update plugin configuration for a tenant
    pub async fn update_plugin_config(
        &self,
        tenant_id: Uuid,
        plugin_name: &str,
        enabled: bool,
        config: serde_json::Value,
    ) -> Result<mindia_core::models::PluginConfig> {
        // Validate plugin exists
        let plugin = self.registry.get(plugin_name).await?;

        // Validate configuration
        plugin.validate_config(&config)?;

        self.config_repo
            .create_or_update_config(tenant_id, plugin_name, enabled, config)
            .await
            .context("Failed to update plugin config")
    }

    /// List all plugin configurations for a tenant
    #[allow(dead_code)]
    pub async fn list_configs(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<mindia_core::models::PluginConfig>> {
        self.config_repo
            .list_configs(tenant_id)
            .await
            .context("Failed to list plugin configs")
    }

    /// List all available plugins
    pub async fn list_plugins(&self) -> Result<Vec<crate::plugins::PluginInfo>> {
        self.registry.list().await
    }
}

#[cfg(test)]
mod tests {
    // PluginService methods require database connections and async operations,
    // so they're better suited for integration tests. list_plugins() is tested in registry.rs.
    // For unit testing other methods, use trait abstractions with a mocking library.
}
