use mindia_db::MediaRepository;
use mindia_db::TaskRepository;
use mindia_storage::Storage;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

#[derive(Clone)]
pub struct CleanupService {
    media_repository: Arc<MediaRepository>,
    storage: Arc<dyn Storage>,
    /// When set, old finished tasks (completed/failed/cancelled) are deleted during cleanup.
    task_repository: Option<Arc<TaskRepository>>,
    /// Retention in days for finished tasks. Used only when task_repository is set.
    task_retention_days: i32,
}

impl CleanupService {
    pub fn new(
        media_repository: Arc<MediaRepository>,
        storage: Arc<dyn Storage>,
        task_repository: Option<Arc<TaskRepository>>,
        task_retention_days: i32,
    ) -> Self {
        Self {
            media_repository,
            storage,
            task_repository,
            task_retention_days,
        }
    }

    /// Start the background cleanup task that runs every hour
    /// Returns a JoinHandle for graceful shutdown
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(3600)); // 1 hour

            loop {
                cleanup_interval.tick().await;

                tracing::info!("Starting scheduled cleanup of expired files");

                if let Err(e) = self.cleanup_expired_files().await {
                    tracing::error!(error = %e, "Cleanup task failed");
                } else {
                    tracing::info!("Cleanup task completed successfully");
                }
            }
        })
    }

    /// Cleanup all expired files from all media types
    #[tracing::instrument(skip(self), fields(cleanup.operation = "expire_all"))]
    async fn cleanup_expired_files(&self) -> Result<(), anyhow::Error> {
        let images = match self.cleanup_expired_images().await {
            Ok(count) => count,
            Err(e) => {
                tracing::error!(error = %e, "Failed to cleanup expired images");
                0
            }
        };

        #[cfg(feature = "video")]
        let videos = match self.cleanup_expired_videos().await {
            Ok(count) => count,
            Err(e) => {
                tracing::error!(error = %e, "Failed to cleanup expired videos");
                0
            }
        };
        #[cfg(not(feature = "video"))]
        let videos = 0usize;

        #[cfg(feature = "document")]
        let documents = match self.cleanup_expired_documents().await {
            Ok(count) => count,
            Err(e) => {
                tracing::error!(error = %e, "Failed to cleanup expired documents");
                0
            }
        };
        #[cfg(not(feature = "document"))]
        let documents = 0usize;

        #[cfg(feature = "audio")]
        let audios = match self.cleanup_expired_audios().await {
            Ok(count) => count,
            Err(e) => {
                tracing::error!(error = %e, "Failed to cleanup expired audios");
                0
            }
        };
        #[cfg(not(feature = "audio"))]
        let audios = 0usize;

        let tasks = if let Some(ref task_repo) = self.task_repository {
            match task_repo
                .delete_old_finished_tasks(self.task_retention_days)
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to cleanup old finished tasks");
                    0
                }
            }
        } else {
            0u64
        };

        let total_deleted = images + videos + documents + audios + (tasks as usize);

        tracing::info!(
            images,
            videos,
            documents,
            audios,
            tasks,
            total_deleted,
            "Cleanup completed"
        );

        Ok(())
    }

    /// Cleanup expired images
    #[tracing::instrument(skip(self), fields(cleanup.media_type = "images"))]
    async fn cleanup_expired_images(&self) -> Result<usize, anyhow::Error> {
        let expired_images = self.media_repository.get_expired_images().await?;
        let count = expired_images.len();

        for image in expired_images {
            tracing::info!(
                image_id = %image.id,
                s3_key = %image.storage_key(),
                expires_at = ?image.expires_at,
                "Deleting expired image"
            );

            match self.storage.delete(image.storage_key()).await {
                Ok(_) => {
                    tracing::debug!(storage_key = %image.storage_key(), "Successfully deleted from storage");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        storage_key = %image.storage_key(),
                        "Failed to delete file from storage, continuing with database deletion"
                    );
                }
            }

            match self
                .media_repository
                .delete(image.tenant_id, image.id)
                .await
            {
                Ok(_) => {
                    tracing::debug!(image_id = %image.id, "Successfully deleted from database");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        image_id = %image.id,
                        "Failed to delete from database"
                    );
                }
            }
        }

        Ok(count)
    }

    /// Cleanup expired videos
    #[cfg(feature = "video")]
    #[tracing::instrument(skip(self), fields(cleanup.media_type = "videos"))]
    async fn cleanup_expired_videos(&self) -> Result<usize, anyhow::Error> {
        let expired_videos: Vec<mindia_core::models::Video> =
            self.media_repository.get_expired_videos().await?;
        let count = expired_videos.len();

        for video in expired_videos {
            tracing::info!(
                video_id = %video.id,
                s3_key = %video.storage_key(),
                expires_at = ?video.expires_at,
                "Deleting expired video"
            );

            match self.storage.delete(video.storage_key()).await {
                Ok(_) => {
                    tracing::debug!(storage_key = %video.storage_key(), "Successfully deleted original video from storage");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        storage_key = %video.storage_key(),
                        "Failed to delete video from storage"
                    );
                }
            }

            // Delete HLS files if they exist
            if let Some(ref hls_master_playlist) = video.hls_master_playlist {
                let hls_folder: String = hls_master_playlist
                    .trim_end_matches("master.m3u8")
                    .to_string();
                tracing::debug!(
                    video_id = %video.id,
                    hls_folder = %hls_folder,
                    "Cleaning up HLS transcoded files"
                );

                let master_key = format!("{}master.m3u8", hls_folder);
                if let Err(e) = self.storage.delete(&master_key).await {
                    tracing::debug!(error = %e, key = %master_key, "Failed to delete master playlist, continuing");
                }

                if let Some(ref variants_json) = video.variants {
                    if let Some(variants_array) = variants_json.as_array() {
                        for variant in variants_array {
                            if let Some(playlist) = variant
                                .get("playlist")
                                .and_then(|p: &serde_json::Value| p.as_str())
                            {
                                let variant_key = format!("{}{}", hls_folder, playlist);
                                if let Err(e) = self.storage.delete(&variant_key).await {
                                    tracing::debug!(error = %e, key = %variant_key, "Failed to delete variant playlist, continuing");
                                }
                            }
                        }
                    }
                }
            }

            match self
                .media_repository
                .delete(video.tenant_id, video.id)
                .await
            {
                Ok(_) => {
                    tracing::debug!(video_id = %video.id, "Successfully deleted from database");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        video_id = %video.id,
                        "Failed to delete from database"
                    );
                }
            }
        }

        Ok(count)
    }

    /// Cleanup expired documents
    #[cfg(feature = "document")]
    #[tracing::instrument(skip(self), fields(cleanup.media_type = "documents"))]
    async fn cleanup_expired_documents(&self) -> Result<usize, anyhow::Error> {
        let expired_documents = self.media_repository.get_expired_documents().await?;
        let count = expired_documents.len();

        for document in expired_documents {
            tracing::info!(
                document_id = %document.id,
                s3_key = %document.storage_key(),
                expires_at = ?document.expires_at,
                "Deleting expired document"
            );

            match self.storage.delete(document.storage_key()).await {
                Ok(_) => {
                    tracing::debug!(storage_key = %document.storage_key(), "Successfully deleted from storage");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        storage_key = %document.storage_key(),
                        "Failed to delete file from storage, continuing with database deletion"
                    );
                }
            }

            match self
                .media_repository
                .delete(document.tenant_id, document.id)
                .await
            {
                Ok(_) => {
                    tracing::debug!(document_id = %document.id, "Successfully deleted from database");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        document_id = %document.id,
                        "Failed to delete from database"
                    );
                }
            }
        }

        Ok(count)
    }

    /// Cleanup expired audios
    #[cfg(feature = "audio")]
    #[tracing::instrument(skip(self), fields(cleanup.media_type = "audios"))]
    async fn cleanup_expired_audios(&self) -> Result<usize, anyhow::Error> {
        let expired_audios = self.media_repository.get_expired_audios().await?;
        let count = expired_audios.len();

        for audio in expired_audios {
            tracing::info!(
                audio_id = %audio.id,
                s3_key = %audio.storage_key(),
                expires_at = ?audio.expires_at,
                "Deleting expired audio"
            );

            match self.storage.delete(audio.storage_key()).await {
                Ok(_) => {
                    tracing::debug!(storage_key = %audio.storage_key(), "Successfully deleted from storage");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        storage_key = %audio.storage_key(),
                        "Failed to delete file from storage, continuing with database deletion"
                    );
                }
            }

            match self
                .media_repository
                .delete(audio.tenant_id, audio.id)
                .await
            {
                Ok(_) => {
                    tracing::debug!(audio_id = %audio.id, "Successfully deleted from database");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        audio_id = %audio.id,
                        "Failed to delete from database"
                    );
                }
            }
        }

        Ok(count)
    }
}
