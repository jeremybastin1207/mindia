//! Media lifecycle operations: deletion of storage artifacts and related data.
//!
//! Keeps handler logic thin and allows unit testing without HTTP.

use futures::stream::{self, StreamExt};
use mindia_core::models::MediaType;
use mindia_db::EmbeddingRepository;
use mindia_storage::Storage;
use std::sync::Arc;
use uuid::Uuid;

/// Service for media lifecycle operations (e.g. deleting artifacts before DB delete).
pub struct MediaLifecycleService;

impl MediaLifecycleService {
    /// Delete type-specific artifacts (HLS files, embeddings) before removing the media row.
    /// Best-effort: logs errors but does not fail the overall delete.
    pub async fn delete_media_artifacts(
        tenant_id: Uuid,
        media_id: Uuid,
        media_type: MediaType,
        storage: &Arc<dyn Storage>,
        embedding_repository: &EmbeddingRepository,
        hls_master_playlist: Option<&String>,
    ) {
        match media_type {
            MediaType::Video => {
                Self::delete_video_hls_files(storage, media_id, hls_master_playlist).await;
            }
            MediaType::Audio => {
                Self::delete_audio_embeddings(embedding_repository, tenant_id, media_id).await;
            }
            MediaType::Image | MediaType::Document => {}
        }
    }

    /// Delete video HLS files (master playlist, variant playlists, segments) from storage.
    async fn delete_video_hls_files(
        storage: &Arc<dyn Storage>,
        media_id: Uuid,
        hls_master_playlist: Option<&String>,
    ) {
        if let Some(master_playlist) = hls_master_playlist {
            if let Err(e) = storage.delete(master_playlist).await {
                tracing::error!(
                    error = %e,
                    storage_key = %master_playlist,
                    "Failed to delete master playlist from storage"
                );
            }

            let base_key = format!("uploads/{}", media_id);
            let variants = vec!["360p", "480p", "720p", "1080p"];

            for variant in variants {
                let variant_playlist = format!("{}/{}/index.m3u8", base_key, variant);
                if let Err(e) = storage.delete(&variant_playlist).await {
                    tracing::debug!(
                        error = %e,
                        storage_key = %variant_playlist,
                        "Variant playlist not found or already deleted"
                    );
                }

                let segment_keys: Vec<String> = (0..999)
                    .map(|i| format!("{}/{}/segment_{:03}.ts", base_key, variant, i))
                    .collect();
                stream::iter(segment_keys)
                    .map(|key| {
                        let s = storage.clone();
                        async move {
                            let _ = s.delete(&key).await;
                        }
                    })
                    .buffer_unordered(64)
                    .collect::<Vec<_>>()
                    .await;
            }
        }
    }

    /// Delete embeddings for an audio media item.
    async fn delete_audio_embeddings(
        embedding_repository: &EmbeddingRepository,
        tenant_id: Uuid,
        audio_id: Uuid,
    ) {
        if let Err(e) = embedding_repository
            .delete_embedding(tenant_id, audio_id)
            .await
        {
            tracing::warn!(
                error = %e,
                audio_id = %audio_id,
                tenant_id = %tenant_id,
                "Failed to delete audio embeddings"
            );
        }
    }
}
