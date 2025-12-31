//! Plugin database repository

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use mindia_core::models::{PluginConfig, PluginExecution, PluginExecutionStatus};
use mindia_core::EncryptionService;

#[derive(Clone)]
pub struct PluginConfigRepository {
    pool: PgPool,
    encryption_service: Option<EncryptionService>,
}

impl PluginConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            encryption_service: None,
        }
    }

    /// Create a new repository with encryption support
    pub fn new_with_encryption(pool: PgPool, encryption_service: EncryptionService) -> Self {
        Self {
            pool,
            encryption_service: Some(encryption_service),
        }
    }

    /// Get plugin configuration for a tenant
    #[tracing::instrument(skip(self), fields(db.table = "plugin_configs", db.operation = "select"))]
    pub async fn get_config(
        &self,
        tenant_id: Uuid,
        plugin_name: &str,
    ) -> Result<Option<PluginConfig>> {
        let config = sqlx::query_as::<Postgres, PluginConfig>(
            r#"
            SELECT id, tenant_id, plugin_name, enabled, config, encrypted_config, uses_encryption, created_at, updated_at
            FROM plugin_configs
            WHERE tenant_id = $1 AND plugin_name = $2
            "#,
        )
        .bind(tenant_id)
        .bind(plugin_name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch plugin config")?;

        Ok(config)
    }

    /// Create or update plugin configuration
    #[tracing::instrument(skip(self, config), fields(db.table = "plugin_configs", db.operation = "upsert"))]
    pub async fn create_or_update_config(
        &self,
        tenant_id: Uuid,
        plugin_name: &str,
        enabled: bool,
        config: serde_json::Value,
    ) -> Result<PluginConfig> {
        let now = Utc::now();

        // Encrypt sensitive fields if encryption service is available
        let (public_config, encrypted_config, uses_encryption) = if let Some(ref enc_service) =
            self.encryption_service
        {
            let (public, encrypted) = enc_service
                .encrypt_sensitive_json(&config)
                .map_err(|e| anyhow::anyhow!("Failed to encrypt sensitive config fields: {}", e))?;

            let uses_enc = encrypted.is_some();
            (public, encrypted, uses_enc)
        } else {
            // No encryption service, store everything in public config
            tracing::warn!(
                tenant_id = %tenant_id,
                plugin_name = %plugin_name,
                "Storing plugin config without encryption - ENCRYPTION_KEY not configured"
            );
            (config.clone(), None, false)
        };

        let plugin_config = sqlx::query_as::<Postgres, PluginConfig>(
            r#"
            INSERT INTO plugin_configs (tenant_id, plugin_name, enabled, config, encrypted_config, uses_encryption, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (tenant_id, plugin_name)
            DO UPDATE SET
                enabled = EXCLUDED.enabled,
                config = EXCLUDED.config,
                encrypted_config = EXCLUDED.encrypted_config,
                uses_encryption = EXCLUDED.uses_encryption,
                updated_at = EXCLUDED.updated_at
            RETURNING id, tenant_id, plugin_name, enabled, config, encrypted_config, uses_encryption, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(plugin_name)
        .bind(enabled)
        .bind(&public_config)
        .bind(&encrypted_config)
        .bind(uses_encryption)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create or update plugin config")?;

        Ok(plugin_config)
    }

    /// List all plugin configurations for a tenant
    #[tracing::instrument(skip(self), fields(db.table = "plugin_configs", db.operation = "select"))]
    pub async fn list_configs(&self, tenant_id: Uuid) -> Result<Vec<PluginConfig>> {
        let configs = sqlx::query_as::<Postgres, PluginConfig>(
            r#"
            SELECT id, tenant_id, plugin_name, enabled, config, encrypted_config, uses_encryption, created_at, updated_at
            FROM plugin_configs
            WHERE tenant_id = $1
            ORDER BY plugin_name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list plugin configs")?;

        Ok(configs)
    }

    /// Delete plugin configuration
    #[tracing::instrument(skip(self), fields(db.table = "plugin_configs", db.operation = "delete"))]
    pub async fn delete_config(&self, tenant_id: Uuid, plugin_name: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM plugin_configs
            WHERE tenant_id = $1 AND plugin_name = $2
            "#,
        )
        .bind(tenant_id)
        .bind(plugin_name)
        .execute(&self.pool)
        .await
        .context("Failed to delete plugin config")?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct PluginExecutionRepository {
    pool: PgPool,
}

impl PluginExecutionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new plugin execution record
    #[tracing::instrument(skip(self), fields(db.table = "plugin_executions", db.operation = "insert"))]
    pub async fn create_execution(
        &self,
        tenant_id: Uuid,
        plugin_name: &str,
        media_id: Uuid,
        task_id: Option<Uuid>,
    ) -> Result<PluginExecution> {
        let now = Utc::now();

        let execution = sqlx::query_as::<Postgres, PluginExecution>(
            r#"
            INSERT INTO plugin_executions (tenant_id, plugin_name, media_id, task_id, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'pending', $5, $6)
            RETURNING id, tenant_id, plugin_name, media_id, task_id, status, result, created_at, updated_at,
                usage_unit_type, usage_input_units, usage_output_units, usage_total_units, usage_raw
            "#,
        )
        .bind(tenant_id)
        .bind(plugin_name)
        .bind(media_id)
        .bind(task_id)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create plugin execution")?;

        Ok(execution)
    }

    /// Update plugin execution status
    #[tracing::instrument(skip(self), fields(db.table = "plugin_executions", db.operation = "update"))]
    pub async fn update_execution_status(
        &self,
        execution_id: Uuid,
        status: PluginExecutionStatus,
        result: Option<serde_json::Value>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE plugin_executions
            SET status = $1, result = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(status)
        .bind(&result)
        .bind(execution_id)
        .execute(&self.pool)
        .await
        .context("Failed to update plugin execution status")?;

        Ok(())
    }

    /// Update plugin execution status with usage data
    #[tracing::instrument(skip(self, usage_raw), fields(db.table = "plugin_executions", db.operation = "update"))]
    #[allow(clippy::too_many_arguments)]
    pub async fn update_execution_with_usage(
        &self,
        execution_id: Uuid,
        status: PluginExecutionStatus,
        result: Option<serde_json::Value>,
        usage_unit_type: Option<&str>,
        usage_input_units: Option<i64>,
        usage_output_units: Option<i64>,
        usage_total_units: Option<i64>,
        usage_raw: Option<&serde_json::Value>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE plugin_executions
            SET status = $1, result = $2, usage_unit_type = $3, usage_input_units = $4,
                usage_output_units = $5, usage_total_units = $6, usage_raw = $7, updated_at = NOW()
            WHERE id = $8
            "#,
        )
        .bind(status)
        .bind(&result)
        .bind(usage_unit_type)
        .bind(usage_input_units)
        .bind(usage_output_units)
        .bind(usage_total_units)
        .bind(usage_raw)
        .bind(execution_id)
        .execute(&self.pool)
        .await
        .context("Failed to update plugin execution with usage")?;

        Ok(())
    }

    /// Get plugin execution by task ID
    #[tracing::instrument(skip(self), fields(db.table = "plugin_executions", db.operation = "select"))]
    pub async fn get_execution_by_task_id(&self, task_id: Uuid) -> Result<Option<PluginExecution>> {
        let execution = sqlx::query_as::<Postgres, PluginExecution>(
            r#"
            SELECT id, tenant_id, plugin_name, media_id, task_id, status, result, created_at, updated_at,
                usage_unit_type, usage_input_units, usage_output_units, usage_total_units, usage_raw
            FROM plugin_executions
            WHERE task_id = $1
            "#,
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch plugin execution by task_id")?;

        Ok(execution)
    }

    /// Update execution task_id
    #[tracing::instrument(skip(self), fields(db.table = "plugin_executions", db.operation = "update"))]
    pub async fn update_task_id(&self, execution_id: Uuid, task_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE plugin_executions
            SET task_id = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(task_id)
        .bind(execution_id)
        .execute(&self.pool)
        .await
        .context("Failed to update execution task_id")?;

        Ok(())
    }

    /// Delete a plugin execution record
    #[tracing::instrument(skip(self), fields(db.table = "plugin_executions", db.operation = "delete"))]
    pub async fn delete_execution(&self, execution_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM plugin_executions
            WHERE id = $1
            "#,
        )
        .bind(execution_id)
        .execute(&self.pool)
        .await
        .context("Failed to delete plugin execution")?;

        Ok(())
    }

    /// Get aggregated usage summary for a tenant, optionally filtered by plugin and date range.
    /// Returns (plugin_name, execution_count, total_units, unit_type) per row.
    #[tracing::instrument(skip(self), fields(db.table = "plugin_executions", db.operation = "select"))]
    pub async fn get_usage_summary(
        &self,
        tenant_id: Uuid,
        plugin_name: Option<&str>,
        period_start: Option<chrono::DateTime<Utc>>,
        period_end: Option<chrono::DateTime<Utc>>,
    ) -> Result<Vec<(String, i64, i64, String)>> {
        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            plugin_name: String,
            execution_count: i64,
            total_units: i64,
            unit_type: String,
        }

        let rows = sqlx::query_as::<Postgres, SummaryRow>(
            r#"
            SELECT plugin_name, COUNT(*)::bigint as execution_count,
                   COALESCE(SUM(usage_total_units), 0)::bigint as total_units,
                   COALESCE(MAX(usage_unit_type), 'unknown') as unit_type
            FROM plugin_executions
            WHERE tenant_id = $1 AND status = 'completed' AND usage_total_units IS NOT NULL
              AND ($2::text IS NULL OR plugin_name = $2)
              AND ($3::timestamptz IS NULL OR created_at >= $3)
              AND ($4::timestamptz IS NULL OR created_at < $4)
            GROUP BY plugin_name
            ORDER BY total_units DESC
            "#,
        )
        .bind(tenant_id)
        .bind(plugin_name)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch plugin usage summary")?;

        Ok(rows
            .into_iter()
            .map(|r| (r.plugin_name, r.execution_count, r.total_units, r.unit_type))
            .collect())
    }
}
