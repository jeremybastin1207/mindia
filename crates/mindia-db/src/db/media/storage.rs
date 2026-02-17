//! Storage location repository: CRUD for storage_locations table.

use mindia_core::models::StorageLocation;
use mindia_core::AppError;
use mindia_core::StorageBackend;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use uuid::Uuid;

/// Row type for storage_locations table (for FromRow).
#[derive(Debug, sqlx::FromRow)]
pub struct StorageLocationRow {
    pub id: Uuid,
    pub backend: StorageBackend,
    pub bucket: Option<String>,
    pub key: String,
    pub url: String,
}

impl StorageLocationRow {
    pub fn to_storage_location(self) -> StorageLocation {
        StorageLocation {
            id: self.id,
            backend: self.backend,
            bucket: self.bucket,
            key: self.key,
            url: self.url,
        }
    }
}

/// Repository for storage_locations table.
#[derive(Clone)]
pub struct StorageLocationRepository {
    pool: PgPool,
}

impl StorageLocationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new storage location and return it.
    #[tracing::instrument(skip(self), fields(db.table = "storage_locations"))]
    pub async fn create(
        &self,
        backend: StorageBackend,
        bucket: Option<String>,
        key: String,
        url: String,
    ) -> Result<StorageLocation, AppError> {
        let row: StorageLocationRow = sqlx::query_as::<Postgres, StorageLocationRow>(
            r#"
            INSERT INTO storage_locations (backend, bucket, key, url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, backend, bucket, key, url
            "#,
        )
        .bind(backend)
        .bind(&bucket)
        .bind(&key)
        .bind(&url)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.to_storage_location())
    }

    /// Insert a new storage location within a transaction.
    #[tracing::instrument(skip(self, tx), fields(db.table = "storage_locations"))]
    pub async fn create_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        backend: StorageBackend,
        bucket: Option<String>,
        key: String,
        url: String,
    ) -> Result<StorageLocation, AppError> {
        let row: StorageLocationRow = sqlx::query_as::<Postgres, StorageLocationRow>(
            r#"
            INSERT INTO storage_locations (backend, bucket, key, url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, backend, bucket, key, url
            "#,
        )
        .bind(backend)
        .bind(&bucket)
        .bind(&key)
        .bind(&url)
        .fetch_one(&mut **tx)
        .await?;
        Ok(row.to_storage_location())
    }

    /// Fetch a storage location by id.
    #[tracing::instrument(skip(self), fields(db.table = "storage_locations", db.record_id = %id))]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<StorageLocation>, AppError> {
        let row: Option<StorageLocationRow> = sqlx::query_as::<Postgres, StorageLocationRow>(
            "SELECT id, backend, bucket, key, url FROM storage_locations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.to_storage_location()))
    }

    /// Fetch multiple storage locations by ids in one query (avoids N+1 when mapping rows to domain types).
    #[tracing::instrument(skip(self, ids), fields(db.table = "storage_locations", count = ids.len()))]
    pub async fn get_by_ids(
        &self,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, StorageLocation>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<StorageLocationRow> = sqlx::query_as::<Postgres, StorageLocationRow>(
            "SELECT id, backend, bucket, key, url FROM storage_locations WHERE id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.id, r.to_storage_location()))
            .collect())
    }
}
