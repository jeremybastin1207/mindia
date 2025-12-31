//! Repository trait abstractions for plugin testing
//!
//! These traits define the minimal interface that plugins need from repositories,
//! allowing for easy mocking and testing without database dependencies.

use anyhow::Result;
use async_trait::async_trait;
use mindia_core::error::AppError;
use mindia_core::models::{Audio, FileGroup, Image, Media, MediaType, ProcessingStatus};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::media::file_group::FileGroupRepository;
use crate::db::media::media::MediaRepository;

/// Trait for media repository operations needed by plugins
#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait PluginMediaRepository: Send + Sync {
    /// Get an audio file by ID
    async fn get_audio(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Audio>>;

    /// Get an image by ID
    async fn get_image(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Image>>;

    /// Get any media by ID
    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Media>>;

    /// Get metadata for a media item
    async fn get_metadata(&self, tenant_id: Uuid, media_id: Uuid) -> Result<Option<JsonValue>>;

    /// Merge plugin metadata into a media item
    /// Returns the updated metadata after merging
    async fn merge_plugin_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        plugin_name: &str,
        metadata: JsonValue,
    ) -> Result<Option<JsonValue>>;

    /// Create a media entry for an already-uploaded file
    /// This is useful when plugins generate new media files (e.g., colorized videos)
    async fn create_media_entry(
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
    ) -> Result<Media>;
}

/// Trait for file group repository operations needed by plugins
#[async_trait]
pub trait PluginFileGroupRepository: Send + Sync {
    /// Create a file group from existing media IDs
    /// This is useful for grouping related files (e.g., original and processed versions)
    async fn create_group(&self, tenant_id: Uuid, media_ids: Vec<Uuid>) -> Result<FileGroup>;
}

// Implementations for concrete repository types

#[async_trait]
impl PluginMediaRepository for MediaRepository {
    async fn get_audio(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Audio>> {
        self.get_audio(tenant_id, id)
            .await
            .map_err(|e: AppError| anyhow::anyhow!(e))
    }

    async fn get_image(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Image>> {
        self.get_image(tenant_id, id)
            .await
            .map_err(|e: AppError| anyhow::anyhow!(e))
    }

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Media>> {
        self.get(tenant_id, id)
            .await
            .map_err(|e: AppError| anyhow::anyhow!(e))
    }

    async fn get_metadata(&self, tenant_id: Uuid, media_id: Uuid) -> Result<Option<JsonValue>> {
        self.get_metadata(tenant_id, media_id)
            .await
            .map_err(|e: AppError| anyhow::anyhow!(e))
    }

    async fn merge_plugin_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        plugin_name: &str,
        metadata: JsonValue,
    ) -> Result<Option<JsonValue>> {
        self.merge_plugin_metadata(tenant_id, media_id, plugin_name, metadata)
            .await
            .map_err(|e: AppError| anyhow::anyhow!(e))
    }

    async fn create_media_entry(
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
    ) -> Result<Media> {
        self.create_media(
            tenant_id,
            id,
            media_type,
            original_filename,
            storage_key,
            storage_url,
            content_type,
            file_size,
            width,
            height,
            duration,
            Some(ProcessingStatus::Completed),
            None, // hls_master_playlist
            None, // variants
            None, // bitrate
            None, // sample_rate
            None, // channels
            None, // page_count
            "permanent".to_string(),
            true,
            None,
            None,
            None,
        )
        .await
        .map_err(|e: AppError| anyhow::anyhow!(e))
    }
}

#[async_trait]
impl PluginFileGroupRepository for FileGroupRepository {
    async fn create_group(&self, tenant_id: Uuid, media_ids: Vec<Uuid>) -> Result<FileGroup> {
        self.create_group(tenant_id, media_ids)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
