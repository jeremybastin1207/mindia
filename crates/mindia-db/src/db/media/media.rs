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

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.row_to_image(r).await?);
        }
        Ok(out)
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

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.row_to_video(r).await?);
        }
        Ok(out)
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

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.row_to_audio(r).await?);
        }
        Ok(out)
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

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.row_to_document(r).await?);
        }
        Ok(out)
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
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.row_to_image(r).await?);
        }
        Ok(out)
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
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.row_to_video(r).await?);
        }
        Ok(out)
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
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.row_to_document(r).await?);
        }
        Ok(out)
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
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.row_to_audio(r).await?);
        }
        Ok(out)
    }

    /// Delete image (alias for delete). Used by cleanup.
    pub async fn delete_image(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        self.delete(tenant_id, id).await
    }

    /// Delete video (alias for delete). Used by cleanup.
    pub async fn delete_video(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        self.delete(tenant_id, id).await
    }

    /// Delete document (alias for delete). Used by cleanup.
    pub async fn delete_document(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        self.delete(tenant_id, id).await
    }

    /// Delete audio (alias for delete). Used by cleanup.
    pub async fn delete_audio(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        self.delete(tenant_id, id).await
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

        // Get folder info if exists
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

        // Batch query: Get all URLs and folder info in single queries
        // Get URLs for all images
        let url_rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT m.id, s.url FROM media m JOIN storage_locations s ON m.storage_id = s.id WHERE m.tenant_id = $1 AND m.id = ANY($2)"
        )
        .bind(tenant_id)
        .bind(&image_ids)
        .fetch_all(&self.pool)
        .await?;

        let url_map: std::collections::HashMap<Uuid, String> = url_rows.into_iter().collect();

        // Get folder info for all images
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

        // Build responses
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

        // Batch query: Get all URLs and folder info
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

        // Generate HLS URL if available
        let hls_url = if let Some(ref playlist) = video.hls_master_playlist {
            // HLS playlist is stored relative to the main video
            Some(format!(
                "{}/{}",
                url.trim_end_matches(&video.filename),
                playlist
            ))
        } else {
            None
        };

        // Get folder info if exists
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

        // Get folder info if exists
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

        // Get folder info if exists
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
        // Validate folder exists and belongs to tenant if provided
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

        // Update media folder_id
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
        // First verify media exists and belongs to tenant
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

        // Get metadata (can be NULL even if media exists)
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

        // Return metadata (assumes it's already in nested structure: {"user": {...}, "plugins": {...}})
        // If NULL, return empty nested structure
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

        Ok(metadata.and_then(|m| {
            // Extract user namespace (assumes nested structure)
            m.get("user").cloned()
        }))
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

    /// Merge user metadata (only updates user namespace, preserves plugins)
    #[tracing::instrument(skip(self, new_metadata), fields(db.table = "media", db.operation = "update", db.record_id = %media_id))]
    pub async fn merge_user_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        new_metadata: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, AppError> {
        // Get current metadata with tenant verification
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

        let current_metadata = row
            .ok_or_else(|| {
                AppError::NotFound("Media not found or does not belong to tenant".to_string())
            })?
            .0
            .unwrap_or_else(|| serde_json::json!({"user": {}, "plugins": {}}));

        // Extract user and plugins namespaces (assumes nested structure)
        let mut user_obj = current_metadata
            .get("user")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);
        let plugins_obj = current_metadata
            .get("plugins")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);

        // Merge new user metadata into existing user namespace
        if let Some(new_obj) = new_metadata.as_object() {
            for (key, value) in new_obj {
                user_obj.insert(key.clone(), value.clone());
            }
        }
        // If not an object, ignore it (log warning in debug mode)
        // This prevents unreachable code and makes the behavior explicit

        // Reconstruct nested structure
        let merged_metadata = serde_json::json!({
            "user": serde_json::Value::Object(user_obj),
            "plugins": serde_json::Value::Object(plugins_obj),
        });

        // Update metadata in database with tenant isolation enforced
        let rows_affected = sqlx::query(
            r#"
            UPDATE media
            SET metadata = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(serde_json::to_value(&merged_metadata)?)
        .bind(media_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound(
                "Media not found or does not belong to tenant".to_string(),
            ));
        }

        Ok(Some(merged_metadata))
    }

    /// Set/update single user metadata key
    #[tracing::instrument(skip(self, value), fields(db.table = "media", db.operation = "update", db.record_id = %media_id, metadata.key = %key))]
    pub async fn merge_user_metadata_key(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        key: &str,
        value: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, AppError> {
        // Validate key
        validation::validate_metadata_key(key)?;
        validation::validate_metadata_value(&value)?;

        // Get current metadata with tenant verification
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

        let current_metadata = row
            .ok_or_else(|| {
                AppError::NotFound("Media not found or does not belong to tenant".to_string())
            })?
            .0
            .unwrap_or_else(|| serde_json::json!({"user": {}, "plugins": {}}));

        // Extract user and plugins namespaces (assumes nested structure)
        let mut user_obj = current_metadata
            .get("user")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);
        let plugins_obj = current_metadata
            .get("plugins")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);

        // Check key count limit
        if !user_obj.contains_key(key) && user_obj.len() >= validation::MAX_USER_METADATA_KEYS {
            return Err(AppError::MetadataKeyLimitExceeded(format!(
                "Cannot add key '{}': metadata already has {} keys (maximum allowed: {})",
                key,
                user_obj.len(),
                validation::MAX_USER_METADATA_KEYS
            )));
        }

        // Set the key in user namespace
        user_obj.insert(key.to_string(), value);

        // Reconstruct nested structure
        let merged_metadata = serde_json::json!({
            "user": serde_json::Value::Object(user_obj),
            "plugins": serde_json::Value::Object(plugins_obj),
        });

        // Update metadata in database with tenant isolation enforced
        let rows_affected = sqlx::query(
            r#"
            UPDATE media
            SET metadata = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(serde_json::to_value(&merged_metadata)?)
        .bind(media_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound(
                "Media not found or does not belong to tenant".to_string(),
            ));
        }

        Ok(Some(merged_metadata))
    }

    /// Delete single user metadata key
    #[tracing::instrument(skip(self), fields(db.table = "media", db.operation = "update", db.record_id = %media_id, metadata.key = %key))]
    pub async fn delete_user_metadata_key(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        key: &str,
    ) -> Result<Option<serde_json::Value>, AppError> {
        // Get current metadata with tenant verification
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

        let current_metadata = row
            .ok_or_else(|| {
                AppError::NotFound("Media not found or does not belong to tenant".to_string())
            })?
            .0
            .unwrap_or_else(|| serde_json::json!({"user": {}, "plugins": {}}));

        // Extract user and plugins namespaces (assumes nested structure)
        let mut user_obj = current_metadata
            .get("user")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);
        let plugins_obj = current_metadata
            .get("plugins")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);

        // Remove key from user namespace (if exists)
        if user_obj.remove(key).is_none() {
            // Key doesn't exist, return current metadata unchanged
            return Ok(Some(current_metadata));
        }

        // Reconstruct nested structure
        let merged_metadata = serde_json::json!({
            "user": serde_json::Value::Object(user_obj),
            "plugins": serde_json::Value::Object(plugins_obj),
        });

        // Update metadata in database with tenant isolation enforced
        let rows_affected = sqlx::query(
            r#"
            UPDATE media
            SET metadata = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(serde_json::to_value(&merged_metadata)?)
        .bind(media_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound(
                "Media not found or does not belong to tenant".to_string(),
            ));
        }

        Ok(Some(merged_metadata))
    }

    /// Replace entire user metadata namespace (preserves plugins)
    #[tracing::instrument(skip(self, new_metadata), fields(db.table = "media", db.operation = "update", db.record_id = %media_id))]
    pub async fn replace_user_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        new_metadata: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, AppError> {
        // Validate user metadata
        validation::validate_user_metadata(&new_metadata)?;

        // Get current metadata with tenant verification
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

        let current_metadata = row
            .ok_or_else(|| {
                AppError::NotFound("Media not found or does not belong to tenant".to_string())
            })?
            .0
            .unwrap_or_else(|| serde_json::json!({"user": {}, "plugins": {}}));

        // Extract plugins namespace (preserve it, assumes nested structure)
        let plugins_obj = current_metadata
            .get("plugins")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);

        // Ensure new_metadata is an object
        let user_obj = if let Some(obj) = new_metadata.as_object() {
            obj.clone()
        } else {
            serde_json::Map::new()
        };

        // Reconstruct nested structure with new user metadata
        let merged_metadata = serde_json::json!({
            "user": serde_json::Value::Object(user_obj),
            "plugins": serde_json::Value::Object(plugins_obj),
        });

        // Update metadata in database with tenant isolation enforced
        let rows_affected = sqlx::query(
            r#"
            UPDATE media
            SET metadata = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(serde_json::to_value(&merged_metadata)?)
        .bind(media_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound(
                "Media not found or does not belong to tenant".to_string(),
            ));
        }

        Ok(Some(merged_metadata))
    }

    /// Merge plugin metadata into plugins.{plugin_name} namespace
    #[tracing::instrument(skip(self, plugin_data), fields(db.table = "media", db.operation = "update", db.record_id = %media_id, plugin.name = %plugin_name))]
    pub async fn merge_plugin_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        plugin_name: &str,
        plugin_data: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, AppError> {
        // Get current metadata with tenant verification
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

        let current_metadata = row
            .ok_or_else(|| {
                AppError::NotFound("Media not found or does not belong to tenant".to_string())
            })?
            .0
            .unwrap_or_else(|| serde_json::json!({"user": {}, "plugins": {}}));

        // Extract user and plugins namespaces (assumes nested structure)
        let user_obj = current_metadata
            .get("user")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);
        let mut plugins_obj = current_metadata
            .get("plugins")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_else(serde_json::Map::new);

        // Get or create plugin namespace
        let mut plugin_obj = plugins_obj
            .entry(plugin_name.to_string())
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .unwrap()
            .clone();

        // Merge plugin data into plugin namespace
        if let Some(new_obj) = plugin_data.as_object() {
            for (key, value) in new_obj {
                plugin_obj.insert(key.clone(), value.clone());
            }
        } else {
            // If not an object, replace entirely
            if let Some(obj) = plugin_data.as_object() {
                plugin_obj = obj.clone();
            }
        }

        // Update plugins namespace
        plugins_obj.insert(
            plugin_name.to_string(),
            serde_json::Value::Object(plugin_obj),
        );

        // Reconstruct nested structure
        let merged_metadata = serde_json::json!({
            "user": serde_json::Value::Object(user_obj),
            "plugins": serde_json::Value::Object(plugins_obj),
        });

        // Update metadata in database with tenant isolation enforced
        let rows_affected = sqlx::query(
            r#"
            UPDATE media
            SET metadata = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(serde_json::to_value(&merged_metadata)?)
        .bind(media_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound(
                "Media not found or does not belong to tenant".to_string(),
            ));
        }

        Ok(Some(merged_metadata))
    }
}
