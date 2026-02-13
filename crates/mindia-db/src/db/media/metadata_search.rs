//! Metadata search repository
//!
//! Provides efficient metadata search capabilities using PostgreSQL JSONB operators
//! and GIN indexes for fast queries on nested metadata structure.

use anyhow::{Context, Result};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use mindia_core::models::{EntityType, SearchResult};
use mindia_core::validation::validate_metadata_key;
#[cfg(feature = "semantic-search")]
use pgvector::Vector;

/// Maximum number of metadata filter conditions allowed per query
const MAX_METADATA_FILTERS: usize = 10;

/// Metadata filter conditions for search queries
#[derive(Debug, Clone)]
pub struct MetadataFilters {
    /// Exact match filters: metadata.user.key = value
    pub exact: Vec<(String, String)>,
    /// Range filters: metadata.user.key >= min_value AND <= max_value
    pub ranges: Vec<(String, Option<String>, Option<String>)>,
    /// Text contains filters: metadata.user.key ILIKE pattern
    pub text_contains: Vec<(String, String)>,
}

impl MetadataFilters {
    pub fn new() -> Self {
        Self {
            exact: Vec::new(),
            ranges: Vec::new(),
            text_contains: Vec::new(),
        }
    }

    /// Check if any filters are present
    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.ranges.is_empty() && self.text_contains.is_empty()
    }

    /// Get total filter count
    pub fn count(&self) -> usize {
        self.exact.len() + self.ranges.len() + self.text_contains.len()
    }

    /// Validate filter count doesn't exceed limit
    pub fn validate(&self) -> Result<()> {
        let count = self.count();
        if count > MAX_METADATA_FILTERS {
            return Err(anyhow::anyhow!(
                "Too many metadata filters: {} (maximum allowed: {})",
                count,
                MAX_METADATA_FILTERS
            ));
        }
        Ok(())
    }
}

impl Default for MetadataFilters {
    fn default() -> Self {
        Self::new()
    }
}

/// Repository for metadata search operations
#[derive(Clone)]
pub struct MetadataSearchRepository {
    pool: PgPool,
}

impl MetadataSearchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Search by metadata filters only (no semantic search)
    ///
    /// Uses PostgreSQL JSONB operators for efficient queries:
    /// - @> operator for exact matches (uses GIN index efficiently)
    /// - ->> operator for range and text searches (less efficient but flexible)
    #[tracing::instrument(skip(self, filters), fields(db.table = "media", db.operation = "search", tenant_id = %tenant_id, filter_count = filters.count()))]
    pub async fn search_by_metadata(
        &self,
        tenant_id: Uuid,
        filters: &MetadataFilters,
        entity_type: Option<EntityType>,
        folder_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SearchResult>> {
        filters.validate()?;

        if !filters.exact.is_empty()
            && filters.ranges.is_empty()
            && filters.text_contains.is_empty()
        {
            let mut exact_obj = serde_json::Map::new();
            for (key, value) in &filters.exact {
                exact_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
            let metadata_filter_json = serde_json::Value::Object(exact_obj);

            let rows = if let Some(et) = entity_type {
                sqlx::query(
                    r#"
                    SELECT 
                        m.id as entity_id,
                        m.media_type::text as entity_type,
                        COALESCE(e.description, '') as description,
                        1.0 as similarity,
                        m.original_filename as filename,
                        COALESCE(sl.url, '') as url,
                        m.content_type,
                        m.file_size
                    FROM media m
                    LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND m.media_type::text = e.entity_type::text::text AND m.tenant_id = e.tenant_id
                    WHERE m.tenant_id = $1 
                      AND m.media_type::text = $2
                      AND m.metadata->'user' @> $3::jsonb
                    ORDER BY m.uploaded_at DESC
                    LIMIT $4 OFFSET $5
                    "#,
                )
                .bind(tenant_id)
                .bind(et.to_string())
                .bind(metadata_filter_json)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            } else {
                sqlx::query(
                    r#"
                    SELECT 
                        m.id as entity_id,
                        m.media_type::text as entity_type,
                        COALESCE(e.description, '') as description,
                        1.0 as similarity,
                        m.original_filename as filename,
                        COALESCE(sl.url, '') as url,
                        m.content_type,
                        m.file_size
                    FROM media m
                    LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND m.media_type::text = e.entity_type::text AND m.tenant_id = e.tenant_id
                    WHERE m.tenant_id = $1 
                      AND m.metadata->'user' @> $2::jsonb
                    ORDER BY m.uploaded_at DESC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(tenant_id)
                .bind(metadata_filter_json)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            };

            let rows = rows.context("Failed to execute metadata search query")?;
            return self.rows_to_search_results(rows);
        }

        if !filters.ranges.is_empty() || !filters.text_contains.is_empty() {
            return self
                .search_by_metadata_complex(
                    tenant_id,
                    filters,
                    entity_type,
                    folder_id,
                    limit,
                    offset,
                )
                .await;
        }

        Ok(Vec::new())
    }

    /// Search with combined semantic similarity and metadata filters
    ///
    /// Combines vector similarity search with metadata filtering for efficient queries
    #[cfg(feature = "semantic-search")]
    #[tracing::instrument(skip(self, query_embedding, filters), fields(db.table = "media", db.operation = "search", tenant_id = %tenant_id, has_embedding = query_embedding.is_some(), filter_count = filters.as_ref().map(|f| f.count()).unwrap_or(0)))]
    #[allow(clippy::too_many_arguments)]
    pub async fn search_with_metadata_filters(
        &self,
        tenant_id: Uuid,
        query_embedding: Option<Vec<f32>>,
        filters: &Option<MetadataFilters>,
        entity_type: Option<EntityType>,
        folder_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SearchResult>> {
        // If no embedding provided, use metadata-only search
        if query_embedding.is_none() {
            if let Some(ref f) = filters {
                if !f.is_empty() {
                    return self
                        .search_by_metadata(tenant_id, f, entity_type, folder_id, limit, 0)
                        .await;
                }
            }
            return Ok(Vec::new());
        }

        // Validate metadata filters if present
        if let Some(ref f) = filters {
            f.validate()?;
        }

        let vector = query_embedding
            .map(Vector::from)
            .context("Query embedding is required for semantic search")?;

        // Handle exact match filters with semantic search (most efficient case)
        if let Some(ref filters) = filters {
            if !filters.exact.is_empty()
                && filters.ranges.is_empty()
                && filters.text_contains.is_empty()
            {
                // Build JSON object for exact matches
                let mut exact_obj = serde_json::Map::new();
                for (key, value) in &filters.exact {
                    exact_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
                }
                let metadata_filter_json = serde_json::Value::Object(exact_obj);

                let rows = match (entity_type, folder_id) {
                    (Some(et), Some(fid)) => {
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
                            JOIN media m ON e.entity_id = m.id AND e.tenant_id = m.tenant_id
                            LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                            WHERE e.tenant_id = $2 
                              AND e.entity_type::text = $3
                              AND m.metadata->'user' @> $4::jsonb
                              AND m.folder_id = $5
                            ORDER BY e.embedding <=> $1
                            LIMIT $6 OFFSET $7
                            "#,
                        )
                        .bind(vector)
                        .bind(tenant_id)
                        .bind(et.to_string())
                        .bind(metadata_filter_json)
                        .bind(fid)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(&self.pool)
                        .await
                    }
                    (Some(et), None) => {
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
                            JOIN media m ON e.entity_id = m.id AND e.tenant_id = m.tenant_id
                            LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                            WHERE e.tenant_id = $2 
                              AND e.entity_type::text = $3
                              AND m.metadata->'user' @> $4::jsonb
                            ORDER BY e.embedding <=> $1
                            LIMIT $5 OFFSET $6
                            "#,
                        )
                        .bind(vector)
                        .bind(tenant_id)
                        .bind(et.to_string())
                        .bind(metadata_filter_json)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(&self.pool)
                        .await
                    }
                    (None, Some(fid)) => {
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
                            JOIN media m ON e.entity_id = m.id AND e.tenant_id = m.tenant_id
                            LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                            WHERE e.tenant_id = $2 
                              AND m.metadata->'user' @> $3::jsonb
                              AND m.folder_id = $4
                            ORDER BY e.embedding <=> $1
                            LIMIT $5 OFFSET $6
                            "#,
                        )
                        .bind(vector)
                        .bind(tenant_id)
                        .bind(metadata_filter_json)
                        .bind(fid)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(&self.pool)
                        .await
                    }
                    (None, None) => {
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
                            JOIN media m ON e.entity_id = m.id AND e.tenant_id = m.tenant_id
                            LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                            WHERE e.tenant_id = $2 
                              AND m.metadata->'user' @> $3::jsonb
                            ORDER BY e.embedding <=> $1
                            LIMIT $4 OFFSET $5
                            "#,
                        )
                        .bind(vector)
                        .bind(tenant_id)
                        .bind(metadata_filter_json)
                        .bind(limit)
                        .bind(offset)
                        .fetch_all(&self.pool)
                        .await
                    }
                };

                let rows = rows.context("Failed to execute combined search query")?;
                return self.rows_to_search_results(rows);
            }
        }

        // If no metadata filters, this should fall through to regular semantic search
        // For now, return empty - this case should be handled by the caller using EmbeddingRepository
        Ok(Vec::new())
    }

    #[cfg(not(feature = "semantic-search"))]
    pub async fn search_with_metadata_filters(
        &self,
        _tenant_id: Uuid,
        _query_embedding: Option<Vec<f32>>,
        _filters: &Option<MetadataFilters>,
        _entity_type: Option<EntityType>,
        _folder_id: Option<Uuid>,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<SearchResult>> {
        Err(anyhow::anyhow!("Semantic search feature is not enabled"))
    }

    /// Search by metadata with complex filters (ranges, text contains, or mixed)
    ///
    /// Builds SQL queries dynamically while maintaining safety through:
    /// 1. Proper JSONB key escaping (keys come from user input, so we escape them)
    /// 2. Parameterized values (all filter values are bound as parameters)
    /// 3. Validation of keys to prevent injection via key names
    async fn search_by_metadata_complex(
        &self,
        tenant_id: Uuid,
        filters: &MetadataFilters,
        entity_type: Option<EntityType>,
        folder_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SearchResult>> {
        // Validate metadata keys using the same validation as when setting metadata
        // This ensures consistency and prevents SQL injection via edge cases
        // Keys are validated, then escaped for SQL safety when interpolated into JSONB paths
        let validate_key = |key: &str| -> Result<String> {
            // Use the same validation function as metadata setting
            // This ensures keys match the same pattern: ^[a-zA-Z0-9_\-\.:]+$
            // and are checked for reserved prefixes
            validate_metadata_key(key)
                .with_context(|| format!("Invalid metadata key: '{}'", key))?;
            Ok(key.to_string())
        };

        // Build WHERE clause conditions
        let mut where_parts = vec!["m.tenant_id = $1".to_string()];
        let mut param_index = 2;

        // Add entity type filter if provided
        if let Some(ref _et) = entity_type {
            where_parts.push(format!("m.media_type::text = ${}", param_index));
            param_index += 1;
        }

        // Add folder_id filter if provided
        if folder_id.is_some() {
            where_parts.push(format!("m.folder_id = ${}", param_index));
            param_index += 1;
        }

        // Add exact match filters using @> operator (most efficient)
        if !filters.exact.is_empty() {
            let mut exact_obj = serde_json::Map::new();
            for (key, value) in &filters.exact {
                let validated_key = validate_key(key)?;
                exact_obj.insert(validated_key, serde_json::Value::String(value.clone()));
            }
            let metadata_filter_json = serde_json::Value::Object(exact_obj);
            where_parts.push(format!("m.metadata->'user' @> ${}::jsonb", param_index));
            param_index += 1;

            // Build query with exact filters
            let query_str = format!(
                r#"
                    SELECT 
                        m.id as entity_id,
                        m.media_type::text as entity_type,
                        COALESCE(e.description, '') as description,
                        1.0 as similarity,
                        m.original_filename as filename,
                        COALESCE(sl.url, '') as url,
                        m.content_type,
                        m.file_size
                    FROM media m
                    LEFT JOIN storage_locations sl ON m.storage_id = sl.id
                    LEFT JOIN embeddings e ON m.id = e.entity_id AND m.media_type::text = e.entity_type::text::text AND m.tenant_id = e.tenant_id
                    WHERE {}
                    ORDER BY m.uploaded_at DESC
                    LIMIT ${} OFFSET ${}
                    "#,
                where_parts.join(" AND "),
                param_index,
                param_index + 1
            );

            let mut query = sqlx::query(&query_str);
            query = query.bind(tenant_id);
            if let Some(et) = entity_type {
                query = query.bind(et.to_string());
            }
            if let Some(fid) = folder_id {
                query = query.bind(fid);
            }
            query = query.bind(metadata_filter_json);
            query = query.bind(limit).bind(offset);

            let rows = query
                .fetch_all(&self.pool)
                .await
                .context("Failed to execute complex metadata search query")?;
            return self.rows_to_search_results(rows);
        }

        // For ranges and text_contains, we need to build conditions manually
        // Since JSONB key paths can't be easily parameterized, we validate keys strictly
        // and parameterize only the values

        let mut query_parts = vec![
            "SELECT m.id as entity_id,".to_string(),
            "m.media_type::text as entity_type,".to_string(),
            "COALESCE(e.description, '') as description,".to_string(),
            "1.0 as similarity,".to_string(),
            "m.original_filename as filename,".to_string(),
            "COALESCE(sl.url, '') as url,".to_string(),
            "m.content_type,".to_string(),
            "m.file_size".to_string(),
            "FROM media m".to_string(),
            "LEFT JOIN storage_locations sl ON m.storage_id = sl.id".to_string(),
            "LEFT JOIN embeddings e ON m.id = e.entity_id AND m.media_type::text = e.entity_type::text::text AND m.tenant_id = e.tenant_id".to_string(),
            format!("WHERE m.tenant_id = ${}", param_index),
        ];
        param_index += 1;

        if let Some(ref _et) = entity_type {
            query_parts.push(format!("AND m.media_type::text = ${}", param_index));
            param_index += 1;
        }

        // Add folder_id filter if provided
        if folder_id.is_some() {
            query_parts.push(format!("AND m.folder_id = ${}", param_index));
            param_index += 1;
        }

        // Add range filter conditions using jsonb_each so keys are parameterized (no SQL interpolation).
        for (key, min_val, max_val) in &filters.ranges {
            validate_key(key)
                .with_context(|| format!("Invalid metadata key in range filter: '{}'", key))?;
            query_parts.push(format!(
                "AND EXISTS (SELECT 1 FROM jsonb_each(m.metadata->'user') AS j(key, value) WHERE j.key = ${} AND (${}::text IS NULL OR j.value::text >= ${}) AND (${}::text IS NULL OR j.value::text <= ${}))",
                param_index,
                param_index + 1,
                param_index + 1,
                param_index + 2,
                param_index + 2
            ));
            param_index += 3;
        }

        // Add text contains filter conditions using jsonb_each so keys are parameterized.
        for (key, _) in &filters.text_contains {
            validate_key(key).with_context(|| {
                format!("Invalid metadata key in text_contains filter: '{}'", key)
            })?;
            query_parts.push(format!(
                "AND EXISTS (SELECT 1 FROM jsonb_each(m.metadata->'user') AS j(key, value) WHERE j.key = ${} AND j.value::text ILIKE ${})",
                param_index,
                param_index + 1
            ));
            param_index += 2;
        }

        query_parts.push("ORDER BY m.uploaded_at DESC".to_string());
        query_parts.push(format!(
            "LIMIT ${} OFFSET ${}",
            param_index,
            param_index + 1
        ));

        let query_str = query_parts.join(" ");

        let mut query = sqlx::query(&query_str);
        query = query.bind(tenant_id);
        if let Some(ref et) = entity_type {
            query = query.bind(et.to_string());
        }
        if let Some(fid) = folder_id {
            query = query.bind(fid);
        }
        for (key, min_val, max_val) in &filters.ranges {
            let validated_key = validate_key(key)
                .with_context(|| format!("Invalid metadata key in range filter: '{}'", key))?;
            query = query
                .bind(validated_key)
                .bind(min_val.clone())
                .bind(max_val.clone());
        }
        for (key, pattern) in &filters.text_contains {
            let validated_key = validate_key(key).with_context(|| {
                format!("Invalid metadata key in text_contains filter: '{}'", key)
            })?;
            let escaped_pattern = pattern
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            query = query
                .bind(validated_key)
                .bind(format!("%{}%", escaped_pattern));
        }
        query = query.bind(limit).bind(offset);

        let rows = query
            .fetch_all(&self.pool)
            .await
            .context("Failed to execute complex metadata search query")?;

        self.rows_to_search_results(rows)
    }

    /// Convert database rows to SearchResult vector
    fn rows_to_search_results(
        &self,
        rows: Vec<sqlx::postgres::PgRow>,
    ) -> Result<Vec<SearchResult>> {
        let results: Result<Vec<SearchResult>> = rows
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

                Ok(SearchResult {
                    id: row.get("entity_id"),
                    entity_type,
                    filename: row.get("filename"),
                    url: row.get("url"),
                    description: row.get("description"),
                    similarity_score: row.get::<f64, _>("similarity") as f32,
                    content_type: row.get("content_type"),
                    file_size: row.get("file_size"),
                })
            })
            .collect();

        results
    }
}
