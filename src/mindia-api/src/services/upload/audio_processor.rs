//! Audio processor implementation

use async_trait::async_trait;
use mindia_core::AppError;
use mindia_services::AudioService;
use uuid::Uuid;

use super::traits::MediaProcessor;

/// Audio metadata extracted from file
#[allow(dead_code)]
pub struct AudioMetadata {
    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
}

/// Audio processor for handling audio-specific operations
#[allow(dead_code)]
pub struct AudioProcessorImpl {
    ffprobe_path: String,
}

impl AudioProcessorImpl {
    /// Create a new audio processor
    #[allow(dead_code)]
    pub fn new(ffprobe_path: String) -> Self {
        Self { ffprobe_path }
    }
}

#[async_trait]
impl MediaProcessor for AudioProcessorImpl {
    type Metadata = AudioMetadata;

    /// Extract audio metadata using ffprobe
    ///
    /// This requires writing to a temporary file since ffprobe needs a file path.
    async fn extract_metadata(&self, data: &[u8]) -> Result<Self::Metadata, AppError> {
        let audio_id = Uuid::new_v4();
        let unique_filename = format!("{}-audio", audio_id);
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join(&unique_filename);

        // Write temporary file for metadata extraction
        tokio::fs::write(&temp_file_path, data)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write temp file: {}", e)))?;

        // Extract metadata using ffprobe
        let audio_service = AudioService::new(self.ffprobe_path.clone());
        let metadata = if let Some(path_str) = temp_file_path.to_str() {
            audio_service
                .extract_audio_metadata_from_path(path_str)
                .await
                .ok() // Don't fail upload if metadata extraction fails
        } else {
            tracing::warn!(
                audio_id = %audio_id,
                temp_path = ?temp_file_path,
                "Invalid temp file path, skipping metadata extraction"
            );
            None
        };

        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_file_path).await;

        if let Some(meta) = metadata {
            Ok(AudioMetadata {
                duration: meta.duration,
                bitrate: meta.bitrate,
                sample_rate: meta.sample_rate,
                channels: meta.channels,
            })
        } else {
            tracing::warn!(
                audio_id = %audio_id,
                "Failed to extract audio metadata, continuing without it"
            );
            // Return empty metadata instead of failing
            Ok(AudioMetadata {
                duration: None,
                bitrate: None,
                sample_rate: None,
                channels: None,
            })
        }
    }

    /// Sanitize audio (no-op for audio files)
    ///
    /// Audio files don't require sanitization like images do with EXIF removal.
    async fn sanitize(&self, data: Vec<u8>) -> Result<Vec<u8>, AppError> {
        Ok(data)
    }
}
