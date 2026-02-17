use super::TaskHandler;
use crate::state::AppState;
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use mindia_core::models::{EntityType, GenerateEmbeddingPayload, Task};
use serde_json::json;
use std::str;
use std::sync::Arc;

pub struct EmbeddingTaskHandler;

#[async_trait]
impl TaskHandler for EmbeddingTaskHandler {
    #[tracing::instrument(skip(self, task, state), fields(task.id = %task.id, entity.id = tracing::field::Empty, entity.type = tracing::field::Empty))]
    async fn process(&self, task: &Task, state: Arc<AppState>) -> Result<serde_json::Value> {
        // Parse payload
        let payload: GenerateEmbeddingPayload = serde_json::from_value(task.payload.clone())
            .context("Failed to parse generate embedding payload")?;

        tracing::Span::current().record("entity.id", payload.entity_id.to_string());
        tracing::Span::current().record("entity.type", &payload.entity_type);

        tracing::info!(
            entity_id = %payload.entity_id,
            entity_type = %payload.entity_type,
            "Processing embedding generation task"
        );

        // Check if semantic search is enabled
        let semantic_search = state
            .semantic_search
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Semantic search is not enabled"))?;

        // Download file using storage abstraction (works for both local and S3 backends)
        let file_data = Self::download_file(&payload.s3_url, &state).await?;

        // Parse entity type
        let entity_type: EntityType = payload.entity_type.parse().context("Invalid entity type")?;

        // Generate description based on entity type
        let description = match entity_type {
            EntityType::Image => {
                tracing::debug!("Describing image with vision model");
                semantic_search.describe_image(file_data, None).await?
            }
            EntityType::Video => {
                tracing::debug!("Extracting video frame and describing");
                // For videos, extract a middle frame using ffmpeg
                let frame_data = Self::extract_video_frame(&file_data, &state).await?;
                semantic_search
                    .describe_video_frame(frame_data, None)
                    .await?
            }
            EntityType::Document => {
                tracing::debug!("Extracting text from document");
                // Run text extraction in a blocking task so future PDF/heavy parsing does not block the runtime
                let data = file_data.clone();
                let text = tokio::task::spawn_blocking(move || Self::extract_document_text(&data))
                    .await
                    .context("spawn_blocking for document text extraction")??;
                semantic_search.summarize_document(&text).await?
            }
            EntityType::Audio => {
                tracing::debug!("Extracting audio metadata and creating description");
                // For audio, extract metadata and create a description
                Self::extract_audio_description(&file_data, &state).await?
            }
        };

        tracing::info!(
            entity_id = %payload.entity_id,
            description_len = description.len(),
            "Description generated, creating embedding"
        );

        // Generate embedding from description
        let embedding = semantic_search.generate_embedding(&description).await?;

        // Store in database with tenant isolation
        state
            .db
            .embedding_repository
            .insert_embedding(
                task.tenant_id, // Use tenant_id from task for tenant isolation
                payload.entity_id,
                entity_type,
                description.clone(),
                embedding,
                semantic_search.embedding_model_name().to_string(),
            )
            .await
            .context("Failed to store embedding in database")?;

        tracing::info!(
            entity_id = %payload.entity_id,
            entity_type = %entity_type,
            "Embedding generation completed successfully"
        );

        Ok(json!({
            "status": "success",
            "entity_id": payload.entity_id,
            "entity_type": entity_type,
            "description_length": description.len()
        }))
    }
}

impl EmbeddingTaskHandler {
    /// Extract storage key from a storage URL (works for both local and S3 URLs).
    fn extract_storage_key_from_url(url: &str) -> Result<String> {
        // Local: http://localhost:3000/media/uuid.jpg -> media/uuid.jpg
        if url.starts_with("http://localhost") || url.starts_with("http://127.0.0.1") {
            if let Some(idx) = url.find("/media/") {
                return Ok(url[idx + 1..].to_string());
            }
            return Err(anyhow::anyhow!("Invalid local URL format: {}", url));
        }
        // S3 or other https: https://bucket.s3.region.amazonaws.com/key -> key (path after host)
        if url.starts_with("https://") || url.starts_with("http://") {
            let after_scheme = url
                .find("://")
                .map(|i| i + 3)
                .ok_or_else(|| anyhow::anyhow!("Invalid URL format: {}", url))?;
            let path_start = url[after_scheme..]
                .find('/')
                .map(|i| after_scheme + i)
                .unwrap_or(url.len());
            let path = if path_start < url.len() {
                url[path_start..].trim_start_matches('/')
            } else {
                ""
            };
            if path.is_empty() {
                return Err(anyhow::anyhow!("Invalid storage URL: no path in {}", url));
            }
            return Ok(path.to_string());
        }
        Err(anyhow::anyhow!("Unsupported URL scheme: {}", url))
    }

    /// Download file using the storage abstraction (works for both local and S3 backends).
    async fn download_file(storage_url: &str, state: &Arc<AppState>) -> Result<Bytes> {
        tracing::debug!(storage_url = %storage_url, "Downloading file from storage");

        let key = Self::extract_storage_key_from_url(storage_url)?;
        tracing::debug!(storage_key = %key, "Downloading via storage abstraction");

        let data = state
            .media
            .storage
            .download(&key)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download file: {}", e))?;

        tracing::debug!(size_bytes = data.len(), "File downloaded successfully");
        Ok(Bytes::from(data))
    }

    async fn extract_video_frame(video_data: &Bytes, state: &Arc<AppState>) -> Result<Bytes> {
        use mindia_services::FFmpegService;
        use tempfile::NamedTempFile;
        use tokio::fs;

        tracing::debug!("Extracting frame from video");

        // Check capacity before creating temp files
        let estimated_video_space = state
            .capacity_checker
            .estimate_temp_file_space(video_data.len() as u64);
        let estimated_frame_space = 10 * 1024 * 1024; // Estimate 10MB for frame
        let temp_dir = std::env::temp_dir();
        state
            .capacity_checker
            .check_disk_space_async(&temp_dir, estimated_video_space + estimated_frame_space)
            .await
            .context("Insufficient disk space for video frame extraction")?;

        // Write video to temp file
        let mut temp_video = NamedTempFile::new()?;
        std::io::Write::write_all(&mut temp_video, video_data)?;
        let video_path = temp_video.path().to_path_buf();

        // Create temp file for frame
        let frame_file = NamedTempFile::new()?;
        let frame_path = frame_file.path().to_path_buf();

        // Extract frame at 1 second (or middle of video)
        let ffmpeg = FFmpegService::new(
            state.media.ffmpeg_path.clone(),
            state.media.hls_segment_duration,
        )
        .context("Failed to initialize FFmpeg service: invalid ffmpeg_path")?;
        ffmpeg
            .extract_frame(&video_path, &frame_path, 1.0)
            .await
            .context("Failed to extract video frame")?;

        // Read frame data
        let frame_data = fs::read(&frame_path)
            .await
            .context("Failed to read extracted frame")?;

        tracing::debug!(frame_size = frame_data.len(), "Video frame extracted");

        Ok(Bytes::from(frame_data))
    }

    fn extract_document_text(document_data: &Bytes) -> Result<String> {
        tracing::debug!("Extracting text from document");

        // Plain text: return as-is
        if let Ok(text) = str::from_utf8(document_data) {
            tracing::debug!(text_len = text.len(), "Text extracted successfully");
            return Ok(text.to_string());
        }

        // PDF: use pdf-extract when document feature is enabled
        if document_data.len() >= 5 && &document_data[0..5] == b"%PDF-" {
            #[cfg(feature = "document")]
            {
                return extract_pdf_text(document_data);
            }
            #[cfg(not(feature = "document"))]
            {
                tracing::warn!("PDF extraction requires document feature");
                return Ok("Document content (PDF extraction not available)".to_string());
            }
        }

        tracing::warn!("Unsupported binary document format");
        Ok("Document content (unsupported binary format)".to_string())
    }

    async fn extract_audio_description(
        audio_data: &Bytes,
        state: &Arc<AppState>,
    ) -> Result<String> {
        use mindia_services::AudioService;
        use tempfile::NamedTempFile;

        tracing::debug!("Extracting audio metadata for description");

        // Check capacity before creating temp file
        let estimated_space = state
            .capacity_checker
            .estimate_temp_file_space(audio_data.len() as u64);
        let temp_dir = std::env::temp_dir();
        state
            .capacity_checker
            .check_disk_space_async(&temp_dir, estimated_space)
            .await
            .context("Insufficient disk space for audio metadata extraction")?;

        // Write audio to temp file for metadata extraction
        let mut temp_audio = NamedTempFile::new()?;
        std::io::Write::write_all(&mut temp_audio, audio_data)?;
        let audio_path = temp_audio.path().to_path_buf();

        // Extract metadata using AudioService
        let ffprobe_path = state.config.ffmpeg_path().replace("ffmpeg", "ffprobe");
        let audio_service = AudioService::new(ffprobe_path);

        // Extract metadata if path is valid UTF-8 (temp file paths should always be valid)
        let metadata = if let Some(path_str) = audio_path.to_str() {
            audio_service
                .extract_audio_metadata_from_path(path_str)
                .await
                .ok()
        } else {
            tracing::warn!("Invalid audio file path: non-UTF8 characters in path");
            None
        };

        // Clean up temp file
        let _ = tokio::fs::remove_file(&audio_path).await;

        // Create description from metadata
        let description = if let Some(meta) = metadata {
            let mut parts = Vec::new();

            if let Some(duration) = meta.duration {
                let minutes = (duration / 60.0) as u32;
                let seconds = (duration % 60.0) as u32;
                parts.push(format!("{}:{} duration", minutes, seconds));
            }

            if let Some(bitrate) = meta.bitrate {
                parts.push(format!("{} kbps bitrate", bitrate));
            }

            if let Some(sample_rate) = meta.sample_rate {
                parts.push(format!("{} Hz sample rate", sample_rate));
            }

            if let Some(channels) = meta.channels {
                let channel_text = match channels {
                    1 => "mono".to_string(),
                    2 => "stereo".to_string(),
                    n => format!("{} channels", n),
                };
                parts.push(channel_text);
            }

            if parts.is_empty() {
                "Audio file".to_string()
            } else {
                format!("Audio file: {}", parts.join(", "))
            }
        } else {
            tracing::warn!("Could not extract audio metadata, using placeholder");
            "Audio file".to_string()
        };

        tracing::debug!(description = %description, "Audio description created");
        Ok(description)
    }
}

#[cfg(feature = "document")]
fn extract_pdf_text(document_data: &Bytes) -> Result<String> {
    use std::io::Write;

    let mut temp = tempfile::NamedTempFile::new().context("Failed to create temp file for PDF")?;
    temp.write_all(document_data)
        .context("Failed to write PDF to temp file")?;
    temp.flush().context("Failed to flush temp file")?;
    let path = temp.path();

    match pdf_extract::extract_text(path) {
        Ok(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                tracing::warn!("PDF text extraction returned empty");
                Ok("Document (no extractable text)".to_string())
            } else {
                tracing::debug!(text_len = trimmed.len(), "PDF text extracted");
                Ok(trimmed.to_string())
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "PDF text extraction failed");
            Ok(format!("Document (extraction failed: {})", e))
        }
    }
}
