//! Video processor implementation

use async_trait::async_trait;
use mindia_core::AppError;

use super::traits::MediaProcessor;

/// Video metadata (empty for now - metadata extracted during transcoding)
#[allow(dead_code)]
pub struct VideoMetadata {
    // Video metadata is extracted during transcoding, not at upload time
}

/// Video processor for handling video-specific operations
#[allow(dead_code)]
pub struct VideoProcessorImpl;

impl VideoProcessorImpl {
    /// Create a new video processor
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MediaProcessor for VideoProcessorImpl {
    type Metadata = VideoMetadata;

    /// Extract video metadata
    ///
    /// For videos, metadata (duration, dimensions) is extracted during
    /// transcoding, not at upload time. This returns empty metadata.
    async fn extract_metadata(&self, _data: &[u8]) -> Result<Self::Metadata, AppError> {
        // Video metadata is extracted during transcoding
        Ok(VideoMetadata {})
    }

    /// Sanitize video (no-op for video files)
    ///
    /// Videos don't require sanitization like images do with EXIF removal.
    async fn sanitize(&self, data: Vec<u8>) -> Result<Vec<u8>, AppError> {
        Ok(data)
    }
}
