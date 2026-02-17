use anyhow::{Context, Result};
use chrono::Utc;
use mindia_core::models::{FileGroup, FileGroupFileItem};
use mindia_storage::Storage;
use sqlx::{PgPool, Postgres};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct FileGroupRepository {
    pool: PgPool,
}

impl FileGroupRepository {
    pub fn new(pool: PgPool, _storage: Arc<dyn Storage>) -> Self {
        Self { pool }
    }

    /// Create a file group from existing media UUIDs
    /// Validates that all files exist and belong to the tenant
    /// Enforces maximum of 1000 files per group
    #[tracing::instrument(skip(self, media_ids), fields(db.table = "file_groups", db.operation = "insert"))]
    pub async fn create_group(&self, tenant_id: Uuid, media_ids: Vec<Uuid>) -> Result<FileGroup> {
        if media_ids.is_empty() {
            return Err(anyhow::anyhow!("Cannot create group with no files"));
        }
        if media_ids.len() > 1000 {
            return Err(anyhow::anyhow!(
                "Groups cannot contain more than 1000 files"
            ));
        }

        let placeholders: Vec<String> = (1..=media_ids.len()).map(|i| format!("${}", i)).collect();
        let query = format!(
            "SELECT id FROM media WHERE tenant_id = ${} AND id IN ({})",
            media_ids.len() + 1,
            placeholders.join(", ")
        );

        let mut query_builder = sqlx::query_scalar::<Postgres, Uuid>(&query);
        for id in &media_ids {
            query_builder = query_builder.bind(id);
        }
        query_builder = query_builder.bind(tenant_id);

        let existing_ids: Vec<Uuid> = query_builder.fetch_all(&self.pool).await?;

        if existing_ids.len() != media_ids.len() {
            return Err(anyhow::anyhow!(
                "Some files not found or do not belong to tenant"
            ));
        }

        let mut seen = std::collections::HashSet::new();
        for id in &media_ids {
            if !seen.insert(id) {
                return Err(anyhow::anyhow!("Duplicate file IDs in request"));
            }
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction")?;

        let group_id = Uuid::new_v4();
        let created_at = Utc::now();

        sqlx::query("INSERT INTO file_groups (id, tenant_id, created_at) VALUES ($1, $2, $3)")
            .bind(group_id)
            .bind(tenant_id)
            .bind(created_at)
            .execute(&mut *tx)
            .await
            .context("Failed to create file group")?;

        for (index, media_id) in media_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO file_group_items (group_id, media_id, index, created_at) VALUES ($1, $2, $3, $4)"
            )
            .bind(group_id)
            .bind(media_id)
            .bind(index as i32)
            .bind(created_at)
            .execute(&mut *tx)
            .await
            .context("Failed to insert file group item")?;
        }

        tx.commit().await.context("Failed to commit transaction")?;

        Ok(FileGroup {
            id: group_id,
            tenant_id,
            created_at,
        })
    }

    /// Get group info (metadata only)
    #[tracing::instrument(skip(self), fields(db.table = "file_groups", db.operation = "select", db.record_id = %group_id))]
    pub async fn get_group_info(
        &self,
        tenant_id: Uuid,
        group_id: Uuid,
    ) -> Result<Option<(FileGroup, i64)>> {
        let row = sqlx::query_as::<Postgres, (Uuid, Uuid, chrono::DateTime<Utc>, i64)>(
            r#"
            SELECT 
                fg.id,
                fg.tenant_id,
                fg.created_at,
                COUNT(fgi.media_id) as file_count
            FROM file_groups fg
            LEFT JOIN file_group_items fgi ON fg.id = fgi.group_id
            WHERE fg.id = $1 AND fg.tenant_id = $2
            GROUP BY fg.id, fg.tenant_id, fg.created_at
            "#,
        )
        .bind(group_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, tenant_id, created_at, file_count)| {
            (
                FileGroup {
                    id,
                    tenant_id,
                    created_at,
                },
                file_count,
            )
        }))
    }

    /// Get all files in a group with metadata
    #[tracing::instrument(skip(self), fields(db.table = "file_groups", db.operation = "select", db.record_id = %group_id))]
    pub async fn get_group_files(
        &self,
        tenant_id: Uuid,
        group_id: Uuid,
    ) -> Result<Vec<FileGroupFileItem>> {
        let group_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM file_groups WHERE id = $1 AND tenant_id = $2")
                .bind(group_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        if group_exists.is_none() {
            return Err(anyhow::anyhow!("File group not found"));
        }

        let rows = sqlx::query_as::<Postgres, (Uuid, String, String, String, i64, String)>(
            r#"
            SELECT 
                m.id,
                COALESCE(sl.url, '') as url,
                m.original_filename as filename,
                m.content_type,
                m.file_size,
                m.media_type::text as media_type
            FROM file_group_items fgi
            JOIN media m ON fgi.media_id = m.id
            LEFT JOIN storage_locations sl ON m.storage_id = sl.id
            WHERE fgi.group_id = $1 AND m.tenant_id = $2
            ORDER BY fgi.index ASC
            "#,
        )
        .bind(group_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, url, filename, content_type, file_size, media_type)| FileGroupFileItem {
                    id,
                    url,
                    filename,
                    content_type,
                    file_size,
                    media_type,
                },
            )
            .collect())
    }

    /// Get specific file by index in group
    #[tracing::instrument(skip(self), fields(db.table = "file_groups", db.operation = "select", db.record_id = %group_id))]
    pub async fn get_file_by_index(
        &self,
        tenant_id: Uuid,
        group_id: Uuid,
        index: i32,
    ) -> Result<Option<(Uuid, String)>> {
        let group_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM file_groups WHERE id = $1 AND tenant_id = $2")
                .bind(group_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        if group_exists.is_none() {
            return Err(anyhow::anyhow!("File group not found"));
        }

        let row = sqlx::query_as::<Postgres, (Uuid, String)>(
            r#"
            SELECT 
                m.id,
                COALESCE(sl.url, '') as url
            FROM file_group_items fgi
            JOIN media m ON fgi.media_id = m.id
            LEFT JOIN storage_locations sl ON m.storage_id = sl.id
            WHERE fgi.group_id = $1 AND fgi.index = $2 AND m.tenant_id = $3
            "#,
        )
        .bind(group_id)
        .bind(index)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Delete a file group (files remain intact)
    #[tracing::instrument(skip(self), fields(db.table = "file_groups", db.operation = "delete", db.record_id = %group_id))]
    pub async fn delete_group(&self, tenant_id: Uuid, group_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM file_groups WHERE id = $1 AND tenant_id = $2")
            .bind(group_id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get media IDs and storage keys for archive generation
    pub async fn get_group_media_for_archive(
        &self,
        tenant_id: Uuid,
        group_id: Uuid,
    ) -> Result<Vec<(Uuid, String, String)>> {
        let group_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM file_groups WHERE id = $1 AND tenant_id = $2")
                .bind(group_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        if group_exists.is_none() {
            return Err(anyhow::anyhow!("File group not found"));
        }

        let rows = sqlx::query_as::<Postgres, (Uuid, String, String)>(
            r#"
            SELECT 
                m.id,
                m.storage_key,
                m.original_filename
            FROM file_group_items fgi
            JOIN media m ON fgi.media_id = m.id
            WHERE fgi.group_id = $1 AND m.tenant_id = $2
            ORDER BY fgi.index ASC
            "#,
        )
        .bind(group_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
