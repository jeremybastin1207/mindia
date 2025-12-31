use mindia_core::models::{Tenant, TenantStatus};
use mindia_core::AppError;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new tenant. Storage is configured separately via tenant_storage_config.
    pub async fn create_tenant(&self, name: &str) -> Result<Tenant, AppError> {
        let tenant = sqlx::query_as::<_, Tenant>(
            r#"
            INSERT INTO tenants (name, status)
            VALUES ($1, 'active')
            RETURNING id, name, status, created_at, updated_at
            "#,
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create tenant: {}", e);
            AppError::Internal("Failed to create tenant".to_string())
        })?;

        tracing::info!("Created new tenant: {} ({})", tenant.name, tenant.id);
        Ok(tenant)
    }

    /// Get tenant by ID
    pub async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<Option<Tenant>, AppError> {
        let tenant = sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, status, created_at, updated_at
            FROM tenants
            WHERE id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch tenant by ID: {}", e);
            AppError::Internal("Failed to fetch tenant".to_string())
        })?;

        Ok(tenant)
    }

    /// List all active tenants
    pub async fn list_active_tenants(&self) -> Result<Vec<Tenant>, AppError> {
        let tenants = sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, status, created_at, updated_at
            FROM tenants
            WHERE status = 'active'
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list active tenants: {}", e);
            AppError::Internal("Failed to list tenants".to_string())
        })?;

        Ok(tenants)
    }

    /// Update tenant status
    pub async fn update_tenant_status(
        &self,
        tenant_id: Uuid,
        status: TenantStatus,
    ) -> Result<Tenant, AppError> {
        let status_str = match status {
            TenantStatus::Active => "active",
            TenantStatus::Suspended => "suspended",
            TenantStatus::Deleted => "deleted",
        };

        let tenant = sqlx::query_as::<_, Tenant>(
            r#"
            UPDATE tenants
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, s3_bucket, s3_region, status, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(status_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update tenant status: {}", e);
            if let sqlx::Error::RowNotFound = e {
                AppError::NotFound("Tenant not found".to_string())
            } else {
                AppError::Internal("Failed to update tenant status".to_string())
            }
        })?;

        tracing::info!("Updated tenant {} status to {:?}", tenant_id, status);
        Ok(tenant)
    }

    /// Update tenant name
    pub async fn update_tenant_name(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<Tenant, AppError> {
        let tenant = sqlx::query_as::<_, Tenant>(
            r#"
            UPDATE tenants
            SET name = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, status, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update tenant name: {}", e);
            if let sqlx::Error::RowNotFound = e {
                AppError::NotFound("Tenant not found".to_string())
            } else {
                AppError::Internal("Failed to update tenant name".to_string())
            }
        })?;

        tracing::info!("Updated tenant {} name to {}", tenant_id, name);
        Ok(tenant)
    }

    /// Delete tenant (soft delete by setting status to deleted)
    pub async fn delete_tenant(&self, tenant_id: Uuid) -> Result<(), AppError> {
        self.update_tenant_status(tenant_id, TenantStatus::Deleted)
            .await?;
        Ok(())
    }
}
