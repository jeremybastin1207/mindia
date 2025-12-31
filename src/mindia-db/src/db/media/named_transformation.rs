//! Repository for managing named transformations (transformation presets)

use mindia_core::{models::NamedTransformation, AppError};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

/// Repository for managing named transformations
#[derive(Clone)]
pub struct NamedTransformationRepository {
    pool: PgPool,
}

impl NamedTransformationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new named transformation
    #[tracing::instrument(skip(self), fields(db.table = "named_transformations", db.operation = "insert"))]
    pub async fn create(
        &self,
        tenant_id: Uuid,
        name: String,
        operations: String,
        description: Option<String>,
    ) -> Result<NamedTransformation, AppError> {
        // Check for duplicate name
        let duplicate_exists = sqlx::query_scalar::<Postgres, bool>(
            "SELECT EXISTS(SELECT 1 FROM named_transformations WHERE tenant_id = $1 AND name = $2)",
        )
        .bind(tenant_id)
        .bind(&name)
        .fetch_one(&self.pool)
        .await?;

        if duplicate_exists {
            return Err(anyhow::anyhow!("A preset with this name already exists").into());
        }

        // Insert the named transformation
        let named_transformation = sqlx::query_as::<Postgres, NamedTransformation>(
            r#"
            INSERT INTO named_transformations (tenant_id, name, operations, description)
            VALUES ($1, $2, $3, $4)
            RETURNING id, tenant_id, name, operations, description, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(&name)
        .bind(&operations)
        .bind(&description)
        .fetch_one(&self.pool)
        .await?;

        Ok(named_transformation)
    }

    /// Get a named transformation by name (tenant-scoped)
    #[tracing::instrument(skip(self), fields(db.table = "named_transformations", db.operation = "select"))]
    pub async fn get_by_name(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<Option<NamedTransformation>, AppError> {
        let named_transformation = sqlx::query_as::<Postgres, NamedTransformation>(
            r#"
            SELECT id, tenant_id, name, operations, description, created_at, updated_at
            FROM named_transformations
            WHERE tenant_id = $1 AND name = $2
            "#,
        )
        .bind(tenant_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(named_transformation)
    }

    /// Get a named transformation by ID (tenant-scoped)
    #[tracing::instrument(skip(self), fields(db.table = "named_transformations", db.operation = "select", db.record_id = %id))]
    pub async fn get_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<NamedTransformation>, AppError> {
        let named_transformation = sqlx::query_as::<Postgres, NamedTransformation>(
            r#"
            SELECT id, tenant_id, name, operations, description, created_at, updated_at
            FROM named_transformations
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(named_transformation)
    }

    /// List all named transformations for a tenant
    #[tracing::instrument(skip(self), fields(db.table = "named_transformations", db.operation = "select"))]
    pub async fn list(&self, tenant_id: Uuid) -> Result<Vec<NamedTransformation>, AppError> {
        let named_transformations = sqlx::query_as::<Postgres, NamedTransformation>(
            r#"
            SELECT id, tenant_id, name, operations, description, created_at, updated_at
            FROM named_transformations
            WHERE tenant_id = $1
            ORDER BY name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(named_transformations)
    }

    /// Update a named transformation
    #[tracing::instrument(skip(self), fields(db.table = "named_transformations", db.operation = "update"))]
    pub async fn update(
        &self,
        tenant_id: Uuid,
        name: &str,
        operations: Option<String>,
        description: Option<Option<String>>,
    ) -> Result<NamedTransformation, AppError> {
        // Check if preset exists
        let existing = self
            .get_by_name(tenant_id, name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Preset not found"))?;

        // Build dynamic update query
        let new_operations = operations.unwrap_or(existing.operations);
        let new_description = match description {
            Some(desc) => desc,           // Explicitly set (can be None to clear)
            None => existing.description, // Keep existing
        };

        let updated = sqlx::query_as::<Postgres, NamedTransformation>(
            r#"
            UPDATE named_transformations
            SET operations = $1, description = $2, updated_at = NOW()
            WHERE tenant_id = $3 AND name = $4
            RETURNING id, tenant_id, name, operations, description, created_at, updated_at
            "#,
        )
        .bind(&new_operations)
        .bind(&new_description)
        .bind(tenant_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(updated)
    }

    /// Delete a named transformation
    #[tracing::instrument(skip(self), fields(db.table = "named_transformations", db.operation = "delete"))]
    pub async fn delete(&self, tenant_id: Uuid, name: &str) -> Result<bool, AppError> {
        let rows_affected =
            sqlx::query("DELETE FROM named_transformations WHERE tenant_id = $1 AND name = $2")
                .bind(tenant_id)
                .bind(name)
                .execute(&self.pool)
                .await?
                .rows_affected();

        Ok(rows_affected > 0)
    }
}
