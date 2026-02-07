//! Video transcoding orchestration: download → transcode → upload HLS → update DB.

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

use mindia_core::models::Video;
use mindia_db::MediaRepository;
use mindia_infra::CapacityChecker;

use super::service::FFmpegService;
use super::video_storage::VideoStorage;

/// Config for video orchestration (FFmpeg, HLS, capacity monitoring).
#[derive(Clone)]
pub struct VideoOrchestratorConfig {
    pub ffmpeg_path: String,
    pub hls_segment_duration: u64,
    pub hls_variants: Vec<String>,
    pub capacity_monitor_interval_secs: u64,
}

/// Orchestrates full video processing: fetch from storage, transcode to HLS, upload, update DB.
pub struct VideoOrchestrator {
    media_repo: MediaRepository,
    storage: Arc<dyn VideoStorage>,
    capacity_checker: Arc<CapacityChecker>,
    config: VideoOrchestratorConfig,
}

impl VideoOrchestrator {
    pub fn new(
        media_repo: MediaRepository,
        storage: Arc<dyn VideoStorage>,
        capacity_checker: Arc<CapacityChecker>,
        config: VideoOrchestratorConfig,
    ) -> Self {
        Self {
            media_repo,
            storage,
            capacity_checker,
            config,
        }
    }

    /// Run the full pipeline for a video: download → transcode → upload HLS → update DB.
    pub async fn process_video(&self, video_id: Uuid) -> Result<()> {
        tracing::info!(video_id = %video_id, "Starting video processing");

        let video: Video = self
            .media_repo
            .get_video_by_id_unchecked(video_id)
            .await?
            .context("Video not found")?;

        let estimated_input_size = 100 * 1024 * 1024u64;
        let estimated_transcode_space = self
            .capacity_checker
            .estimate_video_transcode_space(estimated_input_size);

        let temp_dir_path = std::env::temp_dir();
        self.capacity_checker
            .check_disk_space_async(&temp_dir_path, estimated_transcode_space)
            .await
            .context("Insufficient disk space for video processing")?;
        self.capacity_checker
            .check_memory_async(estimated_transcode_space)
            .await
            .context("Insufficient memory for video processing")?;
        self.capacity_checker
            .check_cpu_usage_async()
            .await
            .context("CPU usage too high for video processing")?;

        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let temp_path = temp_dir.path();

        tracing::info!(video_id = %video_id, s3_key = %video.storage_key(), "Downloading video from storage");
        let video_data = self
            .storage
            .get_file(
                video.storage_bucket().unwrap_or_default(),
                video.storage_key(),
            )
            .await
            .context("Failed to download video from storage")?;

        let actual_input_size = video_data.len() as u64;
        let actual_transcode_space = self
            .capacity_checker
            .estimate_video_transcode_space(actual_input_size);
        self.capacity_checker
            .check_disk_space_async(temp_path, actual_transcode_space)
            .await
            .context("Insufficient disk space after downloading video")?;

        let input_path = temp_path.join("input.mp4");
        tokio::fs::write(&input_path, video_data)
            .await
            .context("Failed to write video to temp file")?;

        let ffmpeg = FFmpegService::new(
            self.config.ffmpeg_path.clone(),
            self.config.hls_segment_duration,
        )
        .context("Failed to initialize FFmpeg service: invalid ffmpeg_path")?;

        tracing::info!(video_id = %video_id, "Probing video metadata");
        let metadata = ffmpeg
            .probe_video(&input_path)
            .await
            .context("Failed to probe video")?;

        tracing::info!(
            video_id = %video_id,
            duration = metadata.duration,
            resolution = %format!("{}x{}", metadata.width, metadata.height),
            "Video metadata extracted"
        );

        let output_dir = temp_path.join("hls");
        tokio::fs::create_dir_all(&output_dir)
            .await
            .context("Failed to create HLS output directory")?;

        tracing::info!(video_id = %video_id, "Generating HLS variants");
        let check_interval = Duration::from_secs(self.config.capacity_monitor_interval_secs);
        let variants = self
            .capacity_checker
            .monitor_during_operation(
                async {
                    ffmpeg
                        .generate_all_variants(
                            &input_path,
                            &output_dir,
                            metadata.width,
                            metadata.height,
                            &self.config.hls_variants,
                        )
                        .await
                },
                check_interval,
            )
            .await
            .context("Failed to generate HLS variants")?;

        tracing::info!(
            video_id = %video_id,
            variant_count = variants.len(),
            "HLS variants generated"
        );

        let master_playlist = ffmpeg
            .create_master_playlist(&variants)
            .context("Failed to create master playlist")?;

        let master_playlist_path = output_dir.join("master.m3u8");
        tokio::fs::write(&master_playlist_path, master_playlist)
            .await
            .context("Failed to write master playlist")?;

        let base_s3_key = format!("uploads/{}", video_id);

        tracing::info!(video_id = %video_id, "Uploading HLS files to storage");

        let master_s3_key = format!("{}/master.m3u8", base_s3_key);
        let master_content = tokio::fs::read(&master_playlist_path).await?;
        self.storage
            .upload_file(
                video.storage_bucket().unwrap_or_default(),
                &master_s3_key,
                master_content,
                "application/vnd.apple.mpegurl",
            )
            .await
            .context("Failed to upload master playlist")?;

        for variant in &variants {
            let variant_dir = output_dir.join(&variant.name);

            let variant_playlist_path = variant_dir.join("index.m3u8");
            let variant_playlist_content = tokio::fs::read(&variant_playlist_path).await?;
            let variant_playlist_s3_key = format!("{}/{}/index.m3u8", base_s3_key, variant.name);
            self.storage
                .upload_file(
                    video.storage_bucket().unwrap_or_default(),
                    &variant_playlist_s3_key,
                    variant_playlist_content,
                    "application/vnd.apple.mpegurl",
                )
                .await
                .context(format!("Failed to upload {} playlist", variant.name))?;

            let mut entries = tokio::fs::read_dir(&variant_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("ts") {
                    let segment_content = tokio::fs::read(&path).await?;
                    let segment_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(String::from)
                        .ok_or_else(|| {
                            anyhow::anyhow!("Invalid segment path: missing file name")
                        })?;
                    let segment_s3_key =
                        format!("{}/{}/{}", base_s3_key, variant.name, segment_name);

                    self.storage
                        .upload_file(
                            video.storage_bucket().unwrap_or_default(),
                            &segment_s3_key,
                            segment_content,
                            "video/mp2t",
                        )
                        .await
                        .context(format!("Failed to upload segment {}", segment_name))?;
                }
            }

            tracing::info!(video_id = %video_id, variant = %variant.name, "Variant uploaded");
        }

        let variants_json =
            serde_json::to_value(&variants).context("Failed to serialize variants")?;
        let hls_master_playlist = format!("{}/master.m3u8", base_s3_key);

        self.media_repo
            .update_video_variants(
                video.tenant_id,
                video_id,
                hls_master_playlist,
                variants_json,
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to update video with HLS information")?;

        tracing::info!(video_id = %video_id, "Video processing completed successfully");
        Ok(())
    }
}
