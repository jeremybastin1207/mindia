use chrono::Utc;
use mindia_core::{
    models::{Embedding, EntityType, SearchResult},
    AppError,
};
#[cfg(feature = "semantic-search")]
use pgvector::Vector;
use sqlx::{PgPool, Postgres, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct EmbeddingRepository {
    pool: PgPool,
}

impl EmbeddingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[tracing::instrument(skip(self, embedding_vec), fields(db.table = "embeddings", db.operation = "insert", tenant_id = %tenant_id))]
    pub async fn insert_embedding(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
        entity_type: EntityType,
        description: String,
        embedding_vec: Vec<f32>,
        model_name: String,
    ) -> Result<Embedding, AppError> {
        let now = Utc::now();
        let vector = Vector::from(embedding_vec);

        let embedding = sqlx::query_as::<Postgres, Embedding>(
            r#"
            INSERT INTO embeddings (
                tenant_id, entity_id, entity_type, description, embedding, model_name, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (entity_id, entity_type) 
            DO UPDATE SET 
                tenant_id = EXCLUDED.tenant_id,
                description = EXCLUDED.description,
                embedding = EXCLUDED.embedding,
                model_name = EXCLUDED.model_name,
                updated_at = EXCLUDED.updated_at
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .bind(entity_type)
        .bind(&description)
        .bind(vector)
        .bind(&model_name)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(embedding)
    }

    #[tracing::instrument(skip(self), fields(db.table = "embeddings", db.operation = "select", db.record_id = %entity_id, tenant_id = %tenant_id))]
    pub async fn get_embedding(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Option<Embedding>, AppError> {
        let embedding = sqlx::query_as::<Postgres, Embedding>(
            "SELECT * FROM embeddings WHERE tenant_id = $1 AND entity_id = $2",
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(embedding)
    }

    #[tracing::instrument(skip(self, query_embedding), fields(db.table = "embeddings", db.operation = "vector_search", tenant_id = %tenant_id, db.limit = %limit))]
    pub async fn search_similar(
        &self,
        tenant_id: Uuid,
        query_embedding: Vec<f32>,
        entity_type: Option<EntityType>,
        folder_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SearchResult>, AppError> {
        let vector = Vector::from(query_embedding);

        let results = match (entity_type, folder_id) {
            (Some(et), Some(fid)) => {
                // Search with both entity type and folder filter
                sqlx::query(
                    r#"
                    SELECT 
                        e.entity_id,
                        e.entity_type::text,
                        e.description,
                        1 - (e.embedding <=> $1) as similarity,
                        COALESCE(m.original_filename, '') as filename,
                        COALESCE(sl.url, '') as url,
                        COALESCE(m.content_type, '') as content_type,
                        COALESCE(m.file_size, 0) as file_size
                    FROM embeddings e
                    JOIN media m ON e.entity_id = m.id AND e.tenant_id = m.tenant_id AND e.entity_type::text = m.media_type::text
                    LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                    WHERE e.tenant_id = $2 AND e.entity_type = $3 AND m.folder_id = $4
                    ORDER BY e.embedding <=> $1
                    LIMIT $5 OFFSET $6
                    "#,
                )
                .bind(vector)
                .bind(tenant_id)
                .bind(et)
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(et), None) => {
                // Search with entity type filter only
                sqlx::query(
                    r#"
                    SELECT 
                        e.entity_id,
                        e.entity_type::text,
                        e.description,
                        1 - (e.embedding <=> $1) as similarity,
                        COALESCE(m.original_filename, '') as filename,
                        COALESCE(sl.url, '') as url,
                        COALESCE(m.content_type, '') as content_type,
                        COALESCE(m.file_size, 0) as file_size
                    FROM embeddings e
                    JOIN media m ON e.entity_id = m.id AND e.tenant_id = m.tenant_id AND e.entity_type::text = m.media_type::text
                    LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                    WHERE e.tenant_id = $2 AND e.entity_type = $3
                    ORDER BY e.embedding <=> $1
                    LIMIT $4 OFFSET $5
                    "#,
                )
                .bind(vector)
                .bind(tenant_id)
                .bind(et)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(fid)) => {
                // Search with folder filter only
                sqlx::query(
                    r#"
                    SELECT 
                        e.entity_id,
                        e.entity_type::text,
                        e.description,
                        1 - (e.embedding <=> $1) as similarity,
                        COALESCE(m.original_filename, '') as filename,
                        COALESCE(sl.url, '') as url,
                        COALESCE(m.content_type, '') as content_type,
                        COALESCE(m.file_size, 0) as file_size
                    FROM embeddings e
                    JOIN media m ON e.entity_id = m.id AND e.tenant_id = m.tenant_id AND e.entity_type::text = m.media_type::text
                    LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                    WHERE e.tenant_id = $2 AND m.folder_id = $3
                    ORDER BY e.embedding <=> $1
                    LIMIT $4 OFFSET $5
                    "#,
                )
                .bind(vector)
                .bind(tenant_id)
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                // Search across all entity types and folders
                sqlx::query(
                    r#"
                    SELECT 
                        e.entity_id,
                        e.entity_type::text,
                        e.description,
                        1 - (e.embedding <=> $1) as similarity,
                        COALESCE(m.original_filename, '') as filename,
                        COALESCE(sl.url, '') as url,
                        COALESCE(m.content_type, '') as content_type,
                        COALESCE(m.file_size, 0) as file_size
                    FROM embeddings e
                    JOIN media m ON e.entity_id = m.id AND e.tenant_id = m.tenant_id AND e.entity_type::text = m.media_type::text
                    LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                    WHERE e.tenant_id = $2
                    ORDER BY e.embedding <=> $1
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(vector)
                .bind(tenant_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let search_results = results
            .into_iter()
            .map(|row| {
                let entity_type_str: String = row.get("entity_type");
                let entity_type = match entity_type_str.as_str() {
                    "image" => EntityType::Image,
                    "video" => EntityType::Video,
                    "document" => EntityType::Document,
                    "audio" => EntityType::Audio,
                    _ => EntityType::Image, // Default fallback
                };

                SearchResult {
                    id: row.get("entity_id"),
                    entity_type,
                    filename: row.get("filename"),
                    url: row.get("url"),
                    description: row.get("description"),
                    similarity_score: row.get::<f64, _>("similarity") as f32,
                    content_type: row.get("content_type"),
                    file_size: row.get("file_size"),
                }
            })
            .collect();

        Ok(search_results)
    }

    #[tracing::instrument(skip(self), fields(db.table = "embeddings", db.operation = "delete", db.record_id = %entity_id, tenant_id = %tenant_id))]
    pub async fn delete_embedding(
        &self,
        tenant_id: Uuid,
        entity_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM embeddings WHERE tenant_id = $1 AND entity_id = $2")
            .bind(tenant_id)
            .bind(entity_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    #[tracing::instrument(skip(self), fields(db.table = "embeddings", db.operation = "count"))]
    pub async fn count_embeddings(&self) -> Result<i64, AppError> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM embeddings")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Get entities that don't have embeddings yet (for batch processing)
    ///
    /// # Security Notice
    ///
    /// - If `tenant_id` is `Some(id)`, only entities for that tenant are returned (tenant-isolated)
    /// - If `tenant_id` is `None`, entities from ALL tenants are returned (system-level only)
    ///
    /// **WARNING:** Passing `None` bypasses tenant isolation. This should ONLY be used by:
    /// - System-level batch processing tools (e.g., CLI tools)
    /// - Background workers with proper authorization
    /// - Never from user-facing API handlers
    ///
    /// For user-facing operations, always pass `Some(tenant_id)` to ensure tenant isolation.
    #[tracing::instrument(skip(self), fields(db.operation = "select_missing_embeddings", entity_type = %entity_type, tenant_id = ?tenant_id))]
    pub async fn get_entities_without_embeddings(
        &self,
        tenant_id: Option<Uuid>,
        entity_type: EntityType,
        limit: i64,
    ) -> Result<Vec<(Uuid, Uuid, String)>, AppError> {
        let results = match (tenant_id, entity_type) {
            (Some(tid), EntityType::Image) => {
                sqlx::query(
                    r#"
                    SELECT m.id, m.tenant_id, sl.url AS s3_url
                    FROM media m
                    JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND e.entity_type = 'image'
                    WHERE m.media_type = 'image' AND m.tenant_id = $1 AND e.id IS NULL
                    LIMIT $2
                    "#,
                )
                .bind(tid)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, EntityType::Image) => {
                sqlx::query(
                    r#"
                    SELECT m.id, m.tenant_id, sl.url AS s3_url
                    FROM media m
                    JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND e.entity_type = 'image'
                    WHERE m.media_type = 'image' AND e.id IS NULL
                    LIMIT $1
                    "#,
                )
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(tid), EntityType::Video) => {
                sqlx::query(
                    r#"
                    SELECT m.id, m.tenant_id, sl.url AS s3_url
                    FROM media m
                    JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND e.entity_type = 'video'
                    WHERE m.media_type = 'video' AND m.tenant_id = $1 AND e.id IS NULL
                    LIMIT $2
                    "#,
                )
                .bind(tid)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, EntityType::Video) => {
                sqlx::query(
                    r#"
                    SELECT m.id, m.tenant_id, sl.url AS s3_url
                    FROM media m
                    JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND e.entity_type = 'video'
                    WHERE m.media_type = 'video' AND e.id IS NULL
                    LIMIT $1
                    "#,
                )
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(tid), EntityType::Document) => {
                sqlx::query(
                    r#"
                    SELECT m.id, m.tenant_id, sl.url AS s3_url
                    FROM media m
                    JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND e.entity_type = 'document'
                    WHERE m.media_type = 'document' AND m.tenant_id = $1 AND e.id IS NULL
                    LIMIT $2
                    "#,
                )
                .bind(tid)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, EntityType::Document) => {
                sqlx::query(
                    r#"
                    SELECT m.id, m.tenant_id, sl.url AS s3_url
                    FROM media m
                    JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND e.entity_type = 'document'
                    WHERE m.media_type = 'document' AND e.id IS NULL
                    LIMIT $1
                    "#,
                )
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(tid), EntityType::Audio) => {
                sqlx::query(
                    r#"
                    SELECT m.id, m.tenant_id, sl.url AS s3_url
                    FROM media m
                    JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND e.entity_type = 'audio'
                    WHERE m.media_type = 'audio' AND m.tenant_id = $1 AND e.id IS NULL
                    LIMIT $2
                    "#,
                )
                .bind(tid)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, EntityType::Audio) => {
                sqlx::query(
                    r#"
                    SELECT m.id, m.tenant_id, sl.url AS s3_url
                    FROM media m
                    JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND e.entity_type = 'audio'
                    WHERE m.media_type = 'audio' AND e.id IS NULL
                    LIMIT $1
                    "#,
                )
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let entities = results
            .into_iter()
            .map(|row| (row.get("id"), row.get("tenant_id"), row.get("s3_url")))
            .collect();

        Ok(entities)
    }
}
