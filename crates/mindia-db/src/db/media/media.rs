use chrono::{DateTime, Utc};
use mindia_core::models::StorageLocation;
use mindia_core::models::{
    to_audio, to_document, to_image, to_video, Audio, AudioResponse, Document, DocumentResponse,
    Image, ImageResponse, Media, MediaRow, MediaType, ProcessingStatus, TypeMetadata, Video,
    VideoResponse,
};
use mindia_core::validation;
use mindia_core::AppError;
use mindia_storage::Storage;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use super::storage::StorageLocationRepository;

fn row_to_image_with_storage(row: MediaRow, loc: &StorageLocation) -> Image {
    to_image(&row.to_media_item(), loc, &row.type_metadata_parsed())
}

fn row_to_video_with_storage(row: MediaRow, loc: &StorageLocation) -> Video {
    to_video(&row.to_media_item(), loc, &row.type_metadata_parsed())
}

fn row_to_audio_with_storage(row: MediaRow, loc: &StorageLocation) -> Audio {
    to_audio(&row.to_media_item(), loc, &row.type_metadata_parsed())
}

fn row_to_document_with_storage(row: MediaRow, loc: &StorageLocation) -> Document {
    to_document(&row.to_media_item(), loc, &row.type_metadata_parsed())
}

/// Unified media repository
///
/// Manages all media types (images, videos, audios, documents) and coordinates
/// with storage backends. Domain models returned by this repository are clean
/// and free of storage implementation details.
#[derive(Clone)]
pub struct MediaRepository {
    pool: PgPool,
    storage: Arc<dyn Storage>,
    storage_locations: StorageLocationRepository,
}

impl MediaRepository {
    pub fn new(pool: PgPool, storage: Arc<dyn Storage>) -> Self {
        Self {
            pool: pool.clone(),
            storage,
            storage_locations: StorageLocationRepository::new(pool),
        }
    }

    /// Fetch storage location by id; required to build Image/Video/Audio/Document from MediaRow.
    async fn get_storage(
        &self,
        storage_id: Uuid,
    ) -> Result<Option<mindia_core::models::StorageLocation>, AppError> {
        self.storage_locations.get_by_id(storage_id).await
    }

    /// Fetch multiple storage locations by id in one query (avoids N+1 in list/get_expired methods).
    async fn get_storages_batch(
        &self,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, mindia_core::models::StorageLocation>, AppError> {
        self.storage_locations.get_by_ids(ids).await
    }

    fn rows_to_with_map<T, F>(
        rows: Vec<MediaRow>,
        storage_map: &HashMap<Uuid, StorageLocation>,
        f: F,
    ) -> Result<Vec<T>, AppError>
    where
        F: Fn(MediaRow, &StorageLocation) -> T,
    {
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let loc = storage_map.get(&r.storage_id).ok_or_else(|| {
                AppError::Internal(format!("Storage location {} not found", r.storage_id))
            })?;
            out.push(f(r, loc));
        }
        Ok(out)
    }

    fn rows_to_images_with_map(
        &self,
        rows: Vec<MediaRow>,
        storage_map: &HashMap<Uuid, StorageLocation>,
    ) -> Result<Vec<Image>, AppError> {
        Self::rows_to_with_map(rows, storage_map, row_to_image_with_storage)
    }

    fn rows_to_videos_with_map(
        &self,
        rows: Vec<MediaRow>,
        storage_map: &HashMap<Uuid, StorageLocation>,
    ) -> Result<Vec<Video>, AppError> {
        Self::rows_to_with_map(rows, storage_map, row_to_video_with_storage)
    }

    fn rows_to_audios_with_map(
        &self,
        rows: Vec<MediaRow>,
        storage_map: &HashMap<Uuid, StorageLocation>,
    ) -> Result<Vec<Audio>, AppError> {
        Self::rows_to_with_map(rows, storage_map, row_to_audio_with_storage)
    }

    fn rows_to_documents_with_map(
        &self,
        rows: Vec<MediaRow>,
        storage_map: &HashMap<Uuid, StorageLocation>,
    ) -> Result<Vec<Document>, AppError> {
        Self::rows_to_with_map(rows, storage_map, row_to_document_with_storage)
    }

    /// Convert MediaRow to Image (requires fetching storage location).
    async fn row_to_image(&self, row: MediaRow) -> Result<Image, AppError> {
        let storage = self.get_storage(row.storage_id).await?.ok_or_else(|| {
            AppError::Internal(format!("Storage location {} not found", row.storage_id))
        })?;
        let item = row.to_media_item();
        let meta = row.type_metadata_parsed();
        Ok(to_image(&item, &storage, &meta))
    }

    /// Convert MediaRow to Video.
    async fn row_to_video(&self, row: MediaRow) -> Result<Video, AppError> {
        let storage = self.get_storage(row.storage_id).await?.ok_or_else(|| {
            AppError::Internal(format!("Storage location {} not found", row.storage_id))
        })?;
        let item = row.to_media_item();
        let meta = row.type_metadata_parsed();
        Ok(to_video(&item, &storage, &meta))
    }

    /// Convert MediaRow to Audio.
    async fn row_to_audio(&self, row: MediaRow) -> Result<Audio, AppError> {
        let storage = self.get_storage(row.storage_id).await?.ok_or_else(|| {
            AppError::Internal(format!("Storage location {} not found", row.storage_id))
        })?;
        let item = row.to_media_item();
        let meta = row.type_metadata_parsed();
        Ok(to_audio(&item, &storage, &meta))
    }

    /// Convert MediaRow to Document.
    async fn row_to_document(&self, row: MediaRow) -> Result<Document, AppError> {
        let storage = self.get_storage(row.storage_id).await?.ok_or_else(|| {
            AppError::Internal(format!("Storage location {} not found", row.storage_id))
        })?;
        let item = row.to_media_item();
        let meta = row.type_metadata_parsed();
        Ok(to_document(&item, &storage, &meta))
    }

    /// Convert MediaRow to Media enum.
    async fn row_to_media(&self, row: MediaRow) -> Result<Media, AppError> {
        let storage = self.get_storage(row.storage_id).await?.ok_or_else(|| {
            AppError::Internal(format!("Storage location {} not found", row.storage_id))
        })?;
        let item = row.to_media_item();
        let meta = row.type_metadata_parsed();
        Ok(match row.media_type {
            MediaType::Image => Media::Image(to_image(&item, &storage, &meta)),
            MediaType::Video => Media::Video(to_video(&item, &storage, &meta)),
            MediaType::Audio => Media::Audio(to_audio(&item, &storage, &meta)),
            MediaType::Document => Media::Document(to_document(&item, &storage, &meta)),
        })
    }

    // =============================================================================
    // IMAGE OPERATIONS
    // =============================================================================

    #[tracing::instrument(skip(self, data), fields(db.table = "media", db.operation = "insert", media_type = "image"))]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_image(
        &self,
        tenant_id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        data: Vec<u8>,
        width: Option<i32>,
        height: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
    ) -> Result<Image, AppError> {
        let id = Uuid::new_v4();
        let file_size = data.len() as i64;

        let (storage_key, storage_url) = self
            .storage
            .upload(tenant_id, &filename, &content_type, data)
            .await
            .map_err(|e| AppError::Internal(format!("Storage upload error: {}", e)))?;

        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Image { width, height }.to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Image)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_image(row).await
    }

    /// Create an image record using an already-uploaded object in storage.
    ///
    /// This is used by the HTTP API upload pipeline, which handles validation,
    /// virus scanning, and storage upload before calling into the repository.
    /// Unlike `create_image`, this method does **not** upload bytes to storage.
    #[tracing::instrument(
        skip(self),
        fields(
            db.table = "media",
            db.operation = "insert",
            media_type = "image"
        )
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_image_from_storage(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        file_size: i64,
        width: Option<i32>,
        height: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        storage_key: String,
        storage_url: String,
    ) -> Result<Image, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Image { width, height }.to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Image)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_image(row).await
    }

    /// Create image record within a transaction (storage_location + media insert). Caller must upload to storage first and cleanup on failure.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_image_from_storage_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: Uuid,
        id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        file_size: i64,
        width: Option<i32>,
        height: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        storage_key: String,
        storage_url: String,
    ) -> Result<Image, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create_tx(tx, backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Image { width, height }.to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Image)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&mut **tx)
        .await?;

        Ok(row_to_image_with_storage(row, &loc))
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", media_type = "image", db.record_id = %id))]
    pub async fn get_image(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Image>, AppError> {
        let row: Option<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE tenant_id = $1 AND id = $2 AND media_type = 'image'",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_image(r).await?)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", media_type = "image"))]
    pub async fn list_images(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<Image>, AppError> {
        let rows: Vec<MediaRow> = match folder_id {
            None => {
                sqlx::query_as::<Postgres, MediaRow>(
                    "SELECT * FROM media WHERE tenant_id = $1 AND media_type = 'image' AND folder_id IS NULL ORDER BY uploaded_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(tenant_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            Some(fid) => {
                sqlx::query_as::<Postgres, MediaRow>(
                    "SELECT * FROM media WHERE tenant_id = $1 AND media_type = 'image' AND folder_id = $2 ORDER BY uploaded_at DESC LIMIT $3 OFFSET $4"
                )
                .bind(tenant_id)
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let storage_ids: Vec<Uuid> = rows.iter().map(|r| r.storage_id).collect();
        let storage_map = self.get_storages_batch(&storage_ids).await?;
        self.rows_to_images_with_map(rows, &storage_map)
    }

    // =============================================================================
    // VIDEO OPERATIONS
    // =============================================================================

    #[tracing::instrument(skip(self, data), fields(db.table = "media", db.operation = "insert", media_type = "video"))]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_video(
        &self,
        tenant_id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        data: Vec<u8>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
    ) -> Result<Video, AppError> {
        let id = Uuid::new_v4();
        let file_size = data.len() as i64;

        let (storage_key, storage_url) = self
            .storage
            .upload(tenant_id, &filename, &content_type, data)
            .await
            .map_err(|e| AppError::Internal(format!("Storage upload error: {}", e)))?;

        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Video {
            width: None,
            height: None,
            duration: None,
            processing_status: Some(ProcessingStatus::Pending),
            hls_master_playlist: None,
            variants: None,
        }
        .to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Video)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_video(row).await
    }

    /// Create a video record using an already-uploaded object in storage.
    ///
    /// This is used by the HTTP API upload pipeline, which uploads the original
    /// bytes before calling into the repository. This method **does not** touch
    /// storage and only persists metadata into the unified `media` table.
    #[tracing::instrument(
        skip(self),
        fields(
            db.table = "media",
            db.operation = "insert",
            media_type = "video"
        )
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_video_from_storage(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        file_size: i64,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        storage_key: String,
        storage_url: String,
    ) -> Result<Video, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Video {
            width: None,
            height: None,
            duration: None,
            processing_status: Some(ProcessingStatus::Pending),
            hls_master_playlist: None,
            variants: None,
        }
        .to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Video)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_video(row).await
    }

    /// Create video record within a transaction. Caller must upload to storage first and cleanup on failure.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_video_from_storage_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: Uuid,
        id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        file_size: i64,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        storage_key: String,
        storage_url: String,
    ) -> Result<Video, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create_tx(tx, backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Video {
            width: None,
            height: None,
            duration: None,
            processing_status: Some(ProcessingStatus::Pending),
            hls_master_playlist: None,
            variants: None,
        }
        .to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Video)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&mut **tx)
        .await?;

        Ok(row_to_video_with_storage(row, &loc))
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", media_type = "video", db.record_id = %id))]
    pub async fn get_video(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Video>, AppError> {
        let row: Option<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE tenant_id = $1 AND id = $2 AND media_type = 'video'",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_video(r).await?)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", media_type = "video"))]
    pub async fn list_videos(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<Video>, AppError> {
        let rows: Vec<MediaRow> = match folder_id {
            None => {
                sqlx::query_as::<Postgres, MediaRow>(
                    "SELECT * FROM media WHERE tenant_id = $1 AND media_type = 'video' AND folder_id IS NULL ORDER BY uploaded_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(tenant_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            Some(fid) => {
                sqlx::query_as::<Postgres, MediaRow>(
                    "SELECT * FROM media WHERE tenant_id = $1 AND media_type = 'video' AND folder_id = $2 ORDER BY uploaded_at DESC LIMIT $3 OFFSET $4"
                )
                .bind(tenant_id)
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let storage_ids: Vec<Uuid> = rows.iter().map(|r| r.storage_id).collect();
        let storage_map = self.get_storages_batch(&storage_ids).await?;
        self.rows_to_videos_with_map(rows, &storage_map)
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "update", media_type = "video", db.record_id = %id))]
    #[allow(clippy::too_many_arguments)]
    pub async fn update_video_processing(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        status: ProcessingStatus,
        width: Option<i32>,
        height: Option<i32>,
        duration: Option<f64>,
        hls_master_playlist: Option<String>,
        variants: Option<sqlx::types::JsonValue>,
    ) -> Result<Video, AppError> {
        let type_metadata: sqlx::types::JsonValue = serde_json::json!({
            "processing_status": format!("{}", status),
            "width": width,
            "height": height,
            "duration": duration,
            "hls_master_playlist": hls_master_playlist,
            "variants": variants
        });
        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            UPDATE media
            SET type_metadata = type_metadata || $3::jsonb, updated_at = NOW()
            WHERE tenant_id = $1 AND id = $2 AND media_type = 'video'
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_video(row).await
    }

    // =============================================================================
    // AUDIO OPERATIONS
    // =============================================================================

    #[tracing::instrument(skip(self, data), fields(db.table = "media", db.operation = "insert", media_type = "audio"))]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_audio(
        &self,
        tenant_id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        data: Vec<u8>,
        duration: Option<f64>,
        bitrate: Option<i32>,
        sample_rate: Option<i32>,
        channels: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
    ) -> Result<Audio, AppError> {
        let id = Uuid::new_v4();
        let file_size = data.len() as i64;

        let (storage_key, storage_url) = self
            .storage
            .upload(tenant_id, &filename, &content_type, data)
            .await
            .map_err(|e| AppError::Internal(format!("Storage upload error: {}", e)))?;

        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Audio {
            duration,
            bitrate,
            sample_rate,
            channels,
        }
        .to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Audio)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_audio(row).await
    }

    /// Create an audio record using an already-uploaded object in storage.
    ///
    /// This is primarily intended for future audio upload handlers and keeps
    /// storage concerns encapsulated in higher layers.
    #[tracing::instrument(
        skip(self),
        fields(
            db.table = "media",
            db.operation = "insert",
            media_type = "audio"
        )
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_audio_from_storage(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        file_size: i64,
        duration: Option<f64>,
        bitrate: Option<i32>,
        sample_rate: Option<i32>,
        channels: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        storage_key: String,
        storage_url: String,
    ) -> Result<Audio, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Audio {
            duration,
            bitrate,
            sample_rate,
            channels,
        }
        .to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Audio)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_audio(row).await
    }

    /// Create audio record within a transaction. Caller must upload to storage first and cleanup on failure.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_audio_from_storage_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: Uuid,
        id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        file_size: i64,
        duration: Option<f64>,
        bitrate: Option<i32>,
        sample_rate: Option<i32>,
        channels: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        storage_key: String,
        storage_url: String,
    ) -> Result<Audio, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create_tx(tx, backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Audio {
            duration,
            bitrate,
            sample_rate,
            channels,
        }
        .to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Audio)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&mut **tx)
        .await?;

        Ok(row_to_audio_with_storage(row, &loc))
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", media_type = "audio", db.record_id = %id))]
    pub async fn get_audio(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Audio>, AppError> {
        let row: Option<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE tenant_id = $1 AND id = $2 AND media_type = 'audio'",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_audio(r).await?)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", media_type = "audio"))]
    pub async fn list_audios(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<Audio>, AppError> {
        let rows: Vec<MediaRow> = match folder_id {
            None => {
                sqlx::query_as::<Postgres, MediaRow>(
                    "SELECT * FROM media WHERE tenant_id = $1 AND media_type = 'audio' AND folder_id IS NULL ORDER BY uploaded_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(tenant_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            Some(fid) => {
                sqlx::query_as::<Postgres, MediaRow>(
                    "SELECT * FROM media WHERE tenant_id = $1 AND media_type = 'audio' AND folder_id = $2 ORDER BY uploaded_at DESC LIMIT $3 OFFSET $4"
                )
                .bind(tenant_id)
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let storage_ids: Vec<Uuid> = rows.iter().map(|r| r.storage_id).collect();
        let storage_map = self.get_storages_batch(&storage_ids).await?;
        self.rows_to_audios_with_map(rows, &storage_map)
    }

    // =============================================================================
    // DOCUMENT OPERATIONS
    // =============================================================================

    #[tracing::instrument(skip(self, data), fields(db.table = "media", db.operation = "insert", media_type = "document"))]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_document(
        &self,
        tenant_id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        data: Vec<u8>,
        page_count: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
    ) -> Result<Document, AppError> {
        let id = Uuid::new_v4();
        let file_size = data.len() as i64;

        let (storage_key, storage_url) = self
            .storage
            .upload(tenant_id, &filename, &content_type, data)
            .await
            .map_err(|e| AppError::Internal(format!("Storage upload error: {}", e)))?;

        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Document { page_count }.to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Document)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_document(row).await
    }

    /// Create a document record using an already-uploaded object in storage.
    ///
    /// This is used by HTTP handlers that delegate validation and storage to
    /// higher-level services and only need to persist metadata into `media`.
    #[tracing::instrument(
        skip(self),
        fields(
            db.table = "media",
            db.operation = "insert",
            media_type = "document"
        )
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_document_from_storage(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        file_size: i64,
        page_count: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        storage_key: String,
        storage_url: String,
    ) -> Result<Document, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Document { page_count }.to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Document)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_document(row).await
    }

    /// Create document record within a transaction. Caller must upload to storage first and cleanup on failure.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_document_from_storage_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: Uuid,
        id: Uuid,
        filename: String,
        original_filename: String,
        content_type: String,
        file_size: i64,
        page_count: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        storage_key: String,
        storage_url: String,
    ) -> Result<Document, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create_tx(tx, backend, None, storage_key, storage_url)
            .await?;
        let type_metadata = TypeMetadata::Document { page_count }.to_json_value();

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(MediaType::Document)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(&type_metadata)
        .fetch_one(&mut **tx)
        .await?;

        Ok(row_to_document_with_storage(row, &loc))
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", media_type = "document", db.record_id = %id))]
    pub async fn get_document(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Document>, AppError> {
        let row: Option<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE tenant_id = $1 AND id = $2 AND media_type = 'document'",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_document(r).await?)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", media_type = "document"))]
    pub async fn list_documents(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<Document>, AppError> {
        let rows: Vec<MediaRow> = match folder_id {
            None => {
                sqlx::query_as::<Postgres, MediaRow>(
                    "SELECT * FROM media WHERE tenant_id = $1 AND media_type = 'document' AND folder_id IS NULL ORDER BY uploaded_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(tenant_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            Some(fid) => {
                sqlx::query_as::<Postgres, MediaRow>(
                    "SELECT * FROM media WHERE tenant_id = $1 AND media_type = 'document' AND folder_id = $2 ORDER BY uploaded_at DESC LIMIT $3 OFFSET $4"
                )
                .bind(tenant_id)
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let storage_ids: Vec<Uuid> = rows.iter().map(|r| r.storage_id).collect();
        let storage_map = self.get_storages_batch(&storage_ids).await?;
        self.rows_to_documents_with_map(rows, &storage_map)
    }

    // =============================================================================
    // GENERIC OPERATIONS (work with any media type)
    // =============================================================================

    /// Create a media record directly (for presigned uploads where file already exists in storage)
    ///
    /// This method is used when the file has already been uploaded to storage (e.g., via presigned URL)
    /// and we just need to create the database record.
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "insert", media_type = ?media_type))]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_media(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        media_type: MediaType,
        original_filename: String,
        storage_key: String,
        storage_url: String,
        content_type: String,
        file_size: i64,
        width: Option<i32>,
        height: Option<i32>,
        duration: Option<f64>,
        processing_status: Option<ProcessingStatus>,
        hls_master_playlist: Option<String>,
        variants: Option<serde_json::Value>,
        bitrate: Option<i32>,
        sample_rate: Option<i32>,
        channels: Option<i32>,
        page_count: Option<i32>,
        store_behavior: String,
        store_permanently: bool,
        expires_at: Option<DateTime<Utc>>,
        folder_id: Option<Uuid>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Media, AppError> {
        let now = Utc::now();
        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, storage_key.clone(), storage_url)
            .await?;
        let filename = format!(
            "{}.{}",
            id,
            original_filename.split('.').next_back().unwrap_or("bin")
        );
        let type_metadata = match media_type {
            MediaType::Image => TypeMetadata::Image { width, height }.to_json_value(),
            MediaType::Video => TypeMetadata::Video {
                width,
                height,
                duration,
                processing_status,
                hls_master_playlist,
                variants,
            }
            .to_json_value(),
            MediaType::Audio => TypeMetadata::Audio {
                duration,
                bitrate,
                sample_rate,
                channels,
            }
            .to_json_value(),
            MediaType::Document => TypeMetadata::Document { page_count }.to_json_value(),
        };

        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, metadata, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(media_type)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(file_size)
        .bind(now)
        .bind(now)
        .bind(&store_behavior)
        .bind(store_permanently)
        .bind(expires_at)
        .bind(folder_id)
        .bind(metadata)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_media(row).await
    }

    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", db.record_id = %id))]
    pub async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Media>, AppError> {
        let row: Option<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_media(r).await?)),
            None => Ok(None),
        }
    }

    /// Delete media: remove from storage first, then DB.
    /// Order ensures we never have "DB says deleted but storage still has file".
    /// If storage delete fails we return error and leave DB unchanged; if DB delete fails
    /// after storage delete, orphaned storage can be cleaned up by a reconciliation job.
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "delete", db.record_id = %id))]
    pub async fn delete(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        let row = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let storage = self.get_storage(row.storage_id).await?.ok_or_else(|| {
                AppError::Internal(format!("Storage location {} not found", row.storage_id))
            })?;
            let storage_key = storage.key.clone();

            // Delete from storage first so we never have DB deleted while storage remains
            self.storage
                .delete(&storage_key)
                .await
                .map_err(|e| AppError::Internal(format!("Storage delete failed: {}", e)))?;

            let rows_affected = sqlx::query("DELETE FROM media WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(id)
                .execute(&self.pool)
                .await?
                .rows_affected();

            Ok(rows_affected > 0)
        } else {
            Ok(false)
        }
    }

    /// Duplicate media: copy file in storage and create a new media record with same metadata.
    /// New record gets store_permanently = true, expires_at = None, new id, and new storage key.
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "copy", db.record_id = %id))]
    pub async fn copy_media(&self, tenant_id: Uuid, id: Uuid) -> Result<Media, AppError> {
        let row = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or_else(|| AppError::NotFound("Media not found".to_string()))?;

        let storage = self.get_storage(row.storage_id).await?.ok_or_else(|| {
            AppError::Internal(format!("Storage location {} not found", row.storage_id))
        })?;

        let new_id = Uuid::new_v4();
        let ext = row.original_filename.rsplit('.').next().unwrap_or("bin");
        let new_filename = format!("{}.{}", new_id, ext);
        let new_key = format!("media/{}/{}", tenant_id, new_filename);

        let new_url = self
            .storage
            .copy(&storage.key, &new_key)
            .await
            .map_err(|e| AppError::Internal(format!("Storage copy failed: {}", e)))?;

        let backend = self.storage.backend_type();
        let loc = self
            .storage_locations
            .create(backend, None, new_key, new_url)
            .await?;

        let now = Utc::now();
        let new_row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            INSERT INTO media (
                id, tenant_id, storage_id, media_type,
                filename, original_filename, content_type, file_size,
                uploaded_at, updated_at,
                store_behavior, store_permanently, expires_at, folder_id, metadata, type_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, true, NULL, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(new_id)
        .bind(tenant_id)
        .bind(loc.id)
        .bind(row.media_type)
        .bind(&new_filename)
        .bind(&row.original_filename)
        .bind(&row.content_type)
        .bind(row.file_size)
        .bind(now)
        .bind(now)
        .bind(&row.store_behavior)
        .bind(row.folder_id)
        .bind(&row.metadata)
        .bind(&row.type_metadata)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_media(new_row).await
    }

    /// List expired images (store_permanently = false, expires_at <= now). Used by cleanup jobs.
    #[tracing::instrument(skip(self), fields(db.table = "media", media_type = "image"))]
    pub async fn get_expired_images(&self) -> Result<Vec<Image>, AppError> {
        let now = Utc::now();
        let rows: Vec<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE media_type = 'image' AND store_permanently = false AND expires_at IS NOT NULL AND expires_at <= $1 ORDER BY expires_at ASC"
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        let storage_ids: Vec<Uuid> = rows.iter().map(|r| r.storage_id).collect();
        let storage_map = self.get_storages_batch(&storage_ids).await?;
        self.rows_to_images_with_map(rows, &storage_map)
    }

    /// List expired videos. Used by cleanup jobs.
    #[tracing::instrument(skip(self), fields(db.table = "media", media_type = "video"))]
    pub async fn get_expired_videos(&self) -> Result<Vec<Video>, AppError> {
        let now = Utc::now();
        let rows: Vec<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE media_type = 'video' AND store_permanently = false AND expires_at IS NOT NULL AND expires_at <= $1 ORDER BY expires_at ASC"
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        let storage_ids: Vec<Uuid> = rows.iter().map(|r| r.storage_id).collect();
        let storage_map = self.get_storages_batch(&storage_ids).await?;
        self.rows_to_videos_with_map(rows, &storage_map)
    }

    /// List expired documents. Used by cleanup jobs.
    #[tracing::instrument(skip(self), fields(db.table = "media", media_type = "document"))]
    pub async fn get_expired_documents(&self) -> Result<Vec<Document>, AppError> {
        let now = Utc::now();
        let rows: Vec<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE media_type = 'document' AND store_permanently = false AND expires_at IS NOT NULL AND expires_at <= $1 ORDER BY expires_at ASC"
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        let storage_ids: Vec<Uuid> = rows.iter().map(|r| r.storage_id).collect();
        let storage_map = self.get_storages_batch(&storage_ids).await?;
        self.rows_to_documents_with_map(rows, &storage_map)
    }

    /// List expired audios. Used by cleanup jobs.
    #[tracing::instrument(skip(self), fields(db.table = "media", media_type = "audio"))]
    pub async fn get_expired_audios(&self) -> Result<Vec<Audio>, AppError> {
        let now = Utc::now();
        let rows: Vec<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE media_type = 'audio' AND store_permanently = false AND expires_at IS NOT NULL AND expires_at <= $1 ORDER BY expires_at ASC"
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        let storage_ids: Vec<Uuid> = rows.iter().map(|r| r.storage_id).collect();
        let storage_map = self.get_storages_batch(&storage_ids).await?;
        self.rows_to_audios_with_map(rows, &storage_map)
    }

    /// Get video by id only (no tenant filter). Used by job queue / orchestration.
    #[tracing::instrument(skip(self), fields(db.table = "media", media_type = "video", db.record_id = %id))]
    pub async fn get_video_by_id_unchecked(&self, id: Uuid) -> Result<Option<Video>, AppError> {
        let row: Option<MediaRow> = sqlx::query_as::<Postgres, MediaRow>(
            "SELECT * FROM media WHERE id = $1 AND media_type = 'video'",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(r) => Ok(Some(self.row_to_video(r).await?)),
            None => Ok(None),
        }
    }

    /// Update only video processing status (convenience for job handlers).
    pub async fn update_video_processing_status(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        status: ProcessingStatus,
    ) -> Result<Video, AppError> {
        self.update_video_processing(tenant_id, id, status, None, None, None, None, None)
            .await
    }

    /// Update video HLS variants and set processing_status to completed. Used by orchestration.
    #[tracing::instrument(skip(self), fields(db.table = "media", media_type = "video", db.record_id = %id))]
    pub async fn update_video_variants(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        hls_master_playlist: String,
        variants: sqlx::types::JsonValue,
    ) -> Result<Video, AppError> {
        let type_metadata: sqlx::types::JsonValue = serde_json::json!({
            "processing_status": "completed",
            "hls_master_playlist": hls_master_playlist,
            "variants": variants
        });
        let row: MediaRow = sqlx::query_as::<Postgres, MediaRow>(
            r#"
            UPDATE media
            SET type_metadata = type_metadata || $3::jsonb, updated_at = NOW()
            WHERE tenant_id = $1 AND id = $2 AND media_type = 'video'
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .bind(&type_metadata)
        .fetch_one(&self.pool)
        .await?;
        self.row_to_video(row).await
    }

    // =============================================================================
    // STORAGE OPERATIONS
    // =============================================================================

    /// Get the URL for a media file
    pub async fn get_url(&self, tenant_id: Uuid, media_id: Uuid) -> Result<String, AppError> {
        let row: (String,) = sqlx::query_as(
            "SELECT s.url FROM media m JOIN storage_locations s ON m.storage_id = s.id WHERE m.tenant_id = $1 AND m.id = $2"
        )
        .bind(tenant_id)
        .bind(media_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Get a presigned/temporary URL for direct access
    pub async fn get_presigned_url(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        expires_in: Duration,
    ) -> Result<String, AppError> {
        let row: (String,) = sqlx::query_as(
            "SELECT s.key FROM media m JOIN storage_locations s ON m.storage_id = s.id WHERE m.tenant_id = $1 AND m.id = $2"
        )
        .bind(tenant_id)
        .bind(media_id)
        .fetch_one(&self.pool)
        .await?;

        let presigned_url = self
            .storage
            .get_presigned_url(&row.0, expires_in)
            .await
            .map_err(|e| AppError::Internal(format!("Storage presigned URL error: {}", e)))?;
        Ok(presigned_url)
    }

    /// Download media content
    pub async fn download_content(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
    ) -> Result<Vec<u8>, AppError> {
        let row: (String,) = sqlx::query_as(
            "SELECT s.key FROM media m JOIN storage_locations s ON m.storage_id = s.id WHERE m.tenant_id = $1 AND m.id = $2"
        )
        .bind(tenant_id)
        .bind(media_id)
        .fetch_one(&self.pool)
        .await?;

        let data = self
            .storage
            .download(&row.0)
            .await
            .map_err(|e| AppError::Internal(format!("Storage download error: {}", e)))?;
        Ok(data)
    }

    // =============================================================================
    // HELPER METHODS FOR BUILDING RESPONSES
    // =============================================================================

    /// Build ImageResponse from Image (includes URL and folder info)
    pub async fn build_image_response(
        &self,
        tenant_id: Uuid,
        image: Image,
    ) -> Result<ImageResponse, AppError> {
        let url = self.get_url(tenant_id, image.id).await?;

        let (folder_id, folder_name): (Option<Uuid>, Option<String>) = sqlx::query_as(
            "SELECT m.folder_id, f.name FROM media m LEFT JOIN folders f ON m.folder_id = f.id WHERE m.tenant_id = $1 AND m.id = $2"
        )
        .bind(tenant_id)
        .bind(image.id)
        .fetch_optional(&self.pool)
        .await?
        .map(|(fid, fname): (Option<Uuid>, Option<String>)| (fid, fname))
        .unwrap_or((None, None));

        Ok(ImageResponse {
            id: image.id,
            filename: image.original_filename,
            url,
            content_type: image.content_type,
            file_size: image.file_size,
            width: image.width,
            height: image.height,
            uploaded_at: image.uploaded_at,
            store_behavior: image.store_behavior,
            store_permanently: image.store_permanently,
            expires_at: image.expires_at,
            folder_id,
            folder_name,
        })
    }

    /// Build ImageResponse for multiple images efficiently (batch query to eliminate N+1)
    pub async fn build_image_responses(
        &self,
        tenant_id: Uuid,
        images: Vec<Image>,
    ) -> Result<Vec<ImageResponse>, AppError> {
        if images.is_empty() {
            return Ok(vec![]);
        }

        let image_ids: Vec<Uuid> = images.iter().map(|img| img.id).collect();

        let url_rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT m.id, s.url FROM media m JOIN storage_locations s ON m.storage_id = s.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&image_ids)
        .fetch_all(&self.pool)
        .await?;

        let url_map: std::collections::HashMap<Uuid, String> = url_rows.into_iter().collect();

        let folder_rows: Vec<(Uuid, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT m.id, m.folder_id, f.name FROM media m LEFT JOIN folders f ON m.folder_id = f.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&image_ids)
        .fetch_all(&self.pool)
        .await?;

        let folder_map: std::collections::HashMap<Uuid, (Option<Uuid>, Option<String>)> =
            folder_rows
                .into_iter()
                .map(|(id, fid, fname)| (id, (fid, fname)))
                .collect();

        let mut responses = Vec::with_capacity(images.len());
        for image in images {
            let url = url_map
                .get(&image.id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("Storage URL not found for image {}", image.id))
                })?
                .clone();
            let (folder_id, folder_name) =
                folder_map.get(&image.id).cloned().unwrap_or((None, None));

            responses.push(ImageResponse {
                id: image.id,
                filename: image.original_filename,
                url,
                content_type: image.content_type,
                file_size: image.file_size,
                width: image.width,
                height: image.height,
                uploaded_at: image.uploaded_at,
                store_behavior: image.store_behavior,
                store_permanently: image.store_permanently,
                expires_at: image.expires_at,
                folder_id,
                folder_name,
            });
        }

        Ok(responses)
    }

    /// Build VideoResponse for multiple videos efficiently (batch query to eliminate N+1)
    pub async fn build_video_responses(
        &self,
        tenant_id: Uuid,
        videos: Vec<Video>,
    ) -> Result<Vec<VideoResponse>, AppError> {
        if videos.is_empty() {
            return Ok(vec![]);
        }

        let video_ids: Vec<Uuid> = videos.iter().map(|v| v.id).collect();

        let url_rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT m.id, s.url FROM media m JOIN storage_locations s ON m.storage_id = s.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&video_ids)
        .fetch_all(&self.pool)
        .await?;

        let url_map: std::collections::HashMap<Uuid, String> = url_rows.into_iter().collect();

        let folder_rows: Vec<(Uuid, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT m.id, m.folder_id, f.name FROM media m LEFT JOIN folders f ON m.folder_id = f.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&video_ids)
        .fetch_all(&self.pool)
        .await?;

        let folder_map: std::collections::HashMap<Uuid, (Option<Uuid>, Option<String>)> =
            folder_rows
                .into_iter()
                .map(|(id, fid, fname)| (id, (fid, fname)))
                .collect();

        let mut responses = Vec::with_capacity(videos.len());
        for video in videos {
            let url = url_map
                .get(&video.id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("Storage URL not found for video {}", video.id))
                })?
                .clone();

            let hls_url = if let Some(ref playlist) = video.hls_master_playlist {
                Some(format!(
                    "{}/{}",
                    url.trim_end_matches(&video.filename),
                    playlist
                ))
            } else {
                None
            };

            let (folder_id, folder_name) =
                folder_map.get(&video.id).cloned().unwrap_or((None, None));

            responses.push(VideoResponse {
                id: video.id,
                filename: video.original_filename,
                url,
                content_type: video.content_type,
                file_size: video.file_size,
                width: video.width,
                height: video.height,
                duration: video.duration,
                processing_status: video.processing_status,
                hls_url,
                variants: video.variants,
                uploaded_at: video.uploaded_at,
                store_behavior: video.store_behavior,
                store_permanently: video.store_permanently,
                expires_at: video.expires_at,
                folder_id,
                folder_name,
            });
        }

        Ok(responses)
    }

    /// Build VideoResponse from Video (includes URLs and folder info)
    pub async fn build_video_response(
        &self,
        tenant_id: Uuid,
        video: Video,
    ) -> Result<VideoResponse, AppError> {
        let url = self.get_url(tenant_id, video.id).await?;

        let hls_url = if let Some(ref playlist) = video.hls_master_playlist {
            Some(format!(
                "{}/{}",
                url.trim_end_matches(&video.filename),
                playlist
            ))
        } else {
            None
        };

        let (folder_id, folder_name): (Option<Uuid>, Option<String>) = sqlx::query_as(
            "SELECT m.folder_id, f.name FROM media m LEFT JOIN folders f ON m.folder_id = f.id WHERE m.tenant_id = $1 AND m.id = $2"
        )
        .bind(tenant_id)
        .bind(video.id)
        .fetch_optional(&self.pool)
        .await?
        .map(|(fid, fname): (Option<Uuid>, Option<String>)| (fid, fname))
        .unwrap_or((None, None));

        Ok(VideoResponse {
            id: video.id,
            filename: video.original_filename,
            url,
            content_type: video.content_type,
            file_size: video.file_size,
            width: video.width,
            height: video.height,
            duration: video.duration,
            processing_status: video.processing_status,
            hls_url,
            variants: video.variants,
            uploaded_at: video.uploaded_at,
            store_behavior: video.store_behavior,
            store_permanently: video.store_permanently,
            expires_at: video.expires_at,
            folder_id,
            folder_name,
        })
    }

    /// Build AudioResponse for multiple audios efficiently (batch query to eliminate N+1)
    pub async fn build_audio_responses(
        &self,
        tenant_id: Uuid,
        audios: Vec<Audio>,
    ) -> Result<Vec<AudioResponse>, AppError> {
        if audios.is_empty() {
            return Ok(vec![]);
        }

        let audio_ids: Vec<Uuid> = audios.iter().map(|a| a.id).collect();

        let url_rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT m.id, s.url FROM media m JOIN storage_locations s ON m.storage_id = s.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&audio_ids)
        .fetch_all(&self.pool)
        .await?;

        let url_map: std::collections::HashMap<Uuid, String> = url_rows.into_iter().collect();

        let folder_rows: Vec<(Uuid, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT m.id, m.folder_id, f.name FROM media m LEFT JOIN folders f ON m.folder_id = f.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&audio_ids)
        .fetch_all(&self.pool)
        .await?;

        let folder_map: std::collections::HashMap<Uuid, (Option<Uuid>, Option<String>)> =
            folder_rows
                .into_iter()
                .map(|(id, fid, fname)| (id, (fid, fname)))
                .collect();

        let mut responses = Vec::with_capacity(audios.len());
        for audio in audios {
            let url = url_map
                .get(&audio.id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("Storage URL not found for audio {}", audio.id))
                })?
                .clone();
            let (folder_id, folder_name) =
                folder_map.get(&audio.id).cloned().unwrap_or((None, None));

            responses.push(AudioResponse {
                id: audio.id,
                filename: audio.original_filename,
                url,
                content_type: audio.content_type,
                file_size: audio.file_size,
                duration: audio.duration,
                bitrate: audio.bitrate,
                sample_rate: audio.sample_rate,
                channels: audio.channels,
                uploaded_at: audio.uploaded_at,
                store_behavior: audio.store_behavior,
                store_permanently: audio.store_permanently,
                expires_at: audio.expires_at,
                folder_id,
                folder_name,
            });
        }

        Ok(responses)
    }

    /// Build AudioResponse from Audio (includes URL and folder info)
    pub async fn build_audio_response(
        &self,
        tenant_id: Uuid,
        audio: Audio,
    ) -> Result<AudioResponse, AppError> {
        let url = self.get_url(tenant_id, audio.id).await?;

        let (folder_id, folder_name): (Option<Uuid>, Option<String>) = sqlx::query_as(
            "SELECT m.folder_id, f.name FROM media m LEFT JOIN folders f ON m.folder_id = f.id WHERE m.tenant_id = $1 AND m.id = $2"
        )
        .bind(tenant_id)
        .bind(audio.id)
        .fetch_optional(&self.pool)
        .await?
        .map(|(fid, fname): (Option<Uuid>, Option<String>)| (fid, fname))
        .unwrap_or((None, None));

        Ok(AudioResponse {
            id: audio.id,
            filename: audio.original_filename,
            url,
            content_type: audio.content_type,
            file_size: audio.file_size,
            duration: audio.duration,
            bitrate: audio.bitrate,
            sample_rate: audio.sample_rate,
            channels: audio.channels,
            uploaded_at: audio.uploaded_at,
            store_behavior: audio.store_behavior,
            store_permanently: audio.store_permanently,
            expires_at: audio.expires_at,
            folder_id,
            folder_name,
        })
    }

    /// Build DocumentResponse for multiple documents efficiently (batch query to eliminate N+1)
    pub async fn build_document_responses(
        &self,
        tenant_id: Uuid,
        documents: Vec<Document>,
    ) -> Result<Vec<DocumentResponse>, AppError> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        let document_ids: Vec<Uuid> = documents.iter().map(|d| d.id).collect();

        let url_rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT m.id, s.url FROM media m JOIN storage_locations s ON m.storage_id = s.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&document_ids)
        .fetch_all(&self.pool)
        .await?;

        let url_map: std::collections::HashMap<Uuid, String> = url_rows.into_iter().collect();

        let folder_rows: Vec<(Uuid, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT m.id, m.folder_id, f.name FROM media m LEFT JOIN folders f ON m.folder_id = f.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&document_ids)
        .fetch_all(&self.pool)
        .await?;

        let folder_map: std::collections::HashMap<Uuid, (Option<Uuid>, Option<String>)> =
            folder_rows
                .into_iter()
                .map(|(id, fid, fname)| (id, (fid, fname)))
                .collect();

        let mut responses = Vec::with_capacity(documents.len());
        for document in documents {
            let url = url_map
                .get(&document.id)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "Storage URL not found for document {}",
                        document.id
                    ))
                })?
                .clone();
            let (folder_id, folder_name) = folder_map
                .get(&document.id)
                .cloned()
                .unwrap_or((None, None));

            responses.push(DocumentResponse {
                id: document.id,
                filename: document.original_filename,
                url,
                content_type: document.content_type,
                file_size: document.file_size,
                page_count: document.page_count,
                uploaded_at: document.uploaded_at,
                store_behavior: document.store_behavior,
                store_permanently: document.store_permanently,
                expires_at: document.expires_at,
                folder_id,
                folder_name,
            });
        }

        Ok(responses)
    }

    /// Build DocumentResponse from Document (includes URL and folder info)
    pub async fn build_document_response(
        &self,
        tenant_id: Uuid,
        document: Document,
    ) -> Result<DocumentResponse, AppError> {
        let url = self.get_url(tenant_id, document.id).await?;

        let (folder_id, folder_name): (Option<Uuid>, Option<String>) = sqlx::query_as(
            "SELECT m.folder_id, f.name FROM media m LEFT JOIN folders f ON m.folder_id = f.id WHERE m.tenant_id = $1 AND m.id = $2"
        )
        .bind(tenant_id)
        .bind(document.id)
        .fetch_optional(&self.pool)
        .await?
        .map(|(fid, fname): (Option<Uuid>, Option<String>)| (fid, fname))
        .unwrap_or((None, None));

        Ok(DocumentResponse {
            id: document.id,
            filename: document.original_filename,
            url,
            content_type: document.content_type,
            file_size: document.file_size,
            page_count: document.page_count,
            uploaded_at: document.uploaded_at,
            store_behavior: document.store_behavior,
            store_permanently: document.store_permanently,
            expires_at: document.expires_at,
            folder_id,
            folder_name,
        })
    }

    /// Move media to a different folder
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "update", db.record_id = %media_id))]
    pub async fn move_media_to_folder(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        folder_id: Option<Uuid>,
    ) -> Result<bool, AppError> {
        if let Some(fid) = folder_id {
            let folder_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND tenant_id = $2)",
            )
            .bind(fid)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

            if !folder_exists {
                return Err(AppError::NotFound(
                    "Folder not found or does not belong to tenant".to_string(),
                ));
            }
        }

        let rows_affected = sqlx::query(
            "UPDATE media SET folder_id = $1, updated_at = NOW() WHERE tenant_id = $2 AND id = $3",
        )
        .bind(folder_id)
        .bind(tenant_id)
        .bind(media_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Get metadata for a media item, ensuring tenant isolation
    ///
    /// This method safely retrieves metadata, ensuring that:
    /// - The media belongs to the specified tenant (tenant isolation)
    /// - Only authorized operations are performed
    /// - Returns metadata in nested structure: {"user": {...}, "plugins": {...}}
    ///
    /// NOTE: This method assumes metadata is already in nested structure.
    /// Flat metadata is not supported and must be migrated separately.
    ///
    /// Returns:
    /// - `Ok(Some(metadata))` if media exists and has metadata (in nested structure)
    /// - `Ok(Some({"user": {}, "plugins": {}}))` if media exists but has no metadata (empty nested structure)
    /// - `Err(_)` if media doesn't exist or doesn't belong to tenant
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", db.record_id = %media_id))]
    pub async fn get_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let media_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM media WHERE id = $1 AND tenant_id = $2)",
        )
        .bind(media_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        if !media_exists {
            return Err(AppError::NotFound(
                "Media not found or does not belong to tenant".to_string(),
            ));
        }

        let row: Option<(Option<serde_json::Value>,)> =
            sqlx::query_as::<Postgres, (Option<serde_json::Value>,)>(
                r#"
            SELECT metadata FROM media
            WHERE id = $1 AND tenant_id = $2
            "#,
            )
            .bind(media_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row
            .and_then(|(metadata,)| metadata)
            .or_else(|| Some(serde_json::json!({"user": {}, "plugins": {}}))))
    }

    /// Get user metadata namespace only
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", db.record_id = %media_id))]
    pub async fn get_user_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let metadata = self.get_metadata(tenant_id, media_id).await?;

        Ok(metadata.and_then(|m| m.get("user").cloned()))
    }

    /// Get single user metadata key value
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "select", db.record_id = %media_id, metadata.key = %key))]
    pub async fn get_user_metadata_key(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        key: &str,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let user_metadata = self.get_user_metadata(tenant_id, media_id).await?;

        Ok(user_metadata.and_then(|m| m.as_object()?.get(key).cloned()))
    }

    /// Merge user metadata (only updates user namespace, preserves plugins).
    /// Uses atomic PostgreSQL jsonb || to avoid read-modify-write races.
    #[tracing::instrument(skip(self, new_metadata), fields(db.table = "media", db.operation = "update", db.record_id = %media_id))]
    pub async fn merge_user_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        new_metadata: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let new_user_json = serde_json::to_value(new_metadata).map_err(AppError::from)?;
        let row: Option<(Option<serde_json::Value>,)> =
            sqlx::query_as::<Postgres, (Option<serde_json::Value>,)>(
                r#"
            UPDATE media
            SET metadata = jsonb_set(
                COALESCE(metadata, '{}'::jsonb),
                '{user}',
                (COALESCE(metadata->'user', '{}'::jsonb) || $1::jsonb)
            ),
            updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            RETURNING metadata
            "#,
            )
            .bind(new_user_json)
            .bind(media_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;

        let merged = row.and_then(|r| r.0).ok_or_else(|| {
            AppError::NotFound("Media not found or does not belong to tenant".to_string())
        })?;
        Ok(Some(merged))
    }

    /// Set/update single user metadata key.
    /// Uses atomic PostgreSQL jsonb_set to avoid read-modify-write races.
    #[tracing::instrument(skip(self, value), fields(db.table = "media", db.operation = "update", db.record_id = %media_id, metadata.key = %key))]
    pub async fn merge_user_metadata_key(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        key: &str,
        value: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, AppError> {
        validation::validate_metadata_key(key)?;
        validation::validate_metadata_value(&value)?;

        // Key count check (best-effort under concurrency; full enforcement would need CHECK constraint)
        let count_row: Option<(i64,)> = sqlx::query_as(
            r#"SELECT (SELECT count(*) FROM jsonb_object_keys(COALESCE(metadata->'user', '{}'::jsonb)))::bigint FROM media WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(media_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;
        let key_count = count_row.map(|r| r.0).unwrap_or(-1);
        let has_key_row: Option<(bool,)> = sqlx::query_as(
            r#"SELECT (metadata->'user' ? $1) FROM media WHERE id = $2 AND tenant_id = $3"#,
        )
        .bind(key)
        .bind(media_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;
        let has_key = has_key_row.map(|r| r.0).unwrap_or(false);
        if !has_key && key_count >= validation::MAX_USER_METADATA_KEYS as i64 {
            return Err(AppError::MetadataKeyLimitExceeded(format!(
                "Cannot add key '{}': metadata already has {} keys (maximum allowed: {})",
                key,
                key_count,
                validation::MAX_USER_METADATA_KEYS
            )));
        }

        let value_json = serde_json::to_value(value).map_err(AppError::from)?;
        let row: Option<(Option<serde_json::Value>,)> =
            sqlx::query_as::<Postgres, (Option<serde_json::Value>,)>(
                r#"
            UPDATE media
            SET metadata = jsonb_set(
                COALESCE(metadata, '{}'::jsonb),
                array['user', $1]::text[],
                $2::jsonb,
                true
            ),
            updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            RETURNING metadata
            "#,
            )
            .bind(key)
            .bind(value_json)
            .bind(media_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;

        let merged = row.and_then(|r| r.0).ok_or_else(|| {
            AppError::NotFound("Media not found or does not belong to tenant".to_string())
        })?;
        Ok(Some(merged))
    }

    /// Delete single user metadata key.
    /// Uses atomic PostgreSQL jsonb #- to avoid read-modify-write races.
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "update", db.record_id = %media_id, metadata.key = %key))]
    pub async fn delete_user_metadata_key(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        key: &str,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let row: Option<(Option<serde_json::Value>,)> =
            sqlx::query_as::<Postgres, (Option<serde_json::Value>,)>(
                r#"
            UPDATE media
            SET metadata = (COALESCE(metadata, '{}'::jsonb) #- array['user', $1]::text[]),
                updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            RETURNING metadata
            "#,
            )
            .bind(key)
            .bind(media_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;

        let merged = row.and_then(|r| r.0).ok_or_else(|| {
            AppError::NotFound("Media not found or does not belong to tenant".to_string())
        })?;
        Ok(Some(merged))
    }

    /// Replace entire user metadata namespace (preserves plugins).
    /// Uses atomic PostgreSQL jsonb_set to avoid read-modify-write races.
    #[tracing::instrument(skip(self, new_metadata), fields(db.table = "media", db.operation = "update", db.record_id = %media_id))]
    pub async fn replace_user_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        new_metadata: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, AppError> {
        validation::validate_user_metadata(&new_metadata)?;
        let user_json = serde_json::to_value(new_metadata).map_err(AppError::from)?;

        let row: Option<(Option<serde_json::Value>,)> =
            sqlx::query_as::<Postgres, (Option<serde_json::Value>,)>(
                r#"
            UPDATE media
            SET metadata = jsonb_set(COALESCE(metadata, '{}'::jsonb), '{user}', $1::jsonb, true),
                updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            RETURNING metadata
            "#,
            )
            .bind(user_json)
            .bind(media_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;

        let merged = row.and_then(|r| r.0).ok_or_else(|| {
            AppError::NotFound("Media not found or does not belong to tenant".to_string())
        })?;
        Ok(Some(merged))
    }

    /// Merge plugin metadata into plugins.{plugin_name} namespace.
    /// Uses atomic PostgreSQL jsonb_set and || to avoid read-modify-write races.
    #[tracing::instrument(skip(self, plugin_data), fields(db.table = "media", db.operation = "update", db.record_id = %media_id, plugin.name = %plugin_name))]
    pub async fn merge_plugin_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        plugin_name: &str,
        plugin_data: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let plugin_json = serde_json::to_value(plugin_data).map_err(AppError::from)?;
        let row: Option<(Option<serde_json::Value>,)> =
            sqlx::query_as::<Postgres, (Option<serde_json::Value>,)>(
                r#"
            UPDATE media
            SET metadata = jsonb_set(
                COALESCE(metadata, '{}'::jsonb),
                array['plugins', $1]::text[],
                (COALESCE(metadata #> array['plugins', $1]::text[], '{}'::jsonb) || $2::jsonb),
                true
            ),
            updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            RETURNING metadata
            "#,
            )
            .bind(plugin_name)
            .bind(plugin_json)
            .bind(media_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;

        let merged = row.and_then(|r| r.0).ok_or_else(|| {
            AppError::NotFound("Media not found or does not belong to tenant".to_string())
        })?;
        Ok(Some(merged))
    }
}
