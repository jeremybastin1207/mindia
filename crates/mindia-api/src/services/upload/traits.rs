//! Traits for media upload configuration and processing

use async_trait::async_trait;
use mindia_core::AppError;

/// Configuration for a media type
///
/// This trait provides media-type specific configuration values
/// like max file size, allowed extensions, and content types.
pub trait MediaUploadConfig: Send + Sync {
    /// Maximum allowed file size in bytes
    fn max_file_size(&self) -> usize;

    /// Allowed file extensions (without leading dot)
    fn allowed_extensions(&self) -> &[String];

    /// Allowed content types
    fn allowed_content_types(&self) -> &[String];

    /// Name of the media type (for logging/errors)
    fn media_type_name(&self) -> &'static str;
}

/// Media-specific processing (dimensions, duration, sanitization)
///
/// Each media type implements this trait to provide type-specific
/// processing like dimension extraction for images or metadata
/// extraction for audio/video.
#[async_trait]
pub trait MediaProcessor: Send + Sync {
    /// Metadata type extracted from the media file
    type Metadata: Send;

    /// Extract metadata from file data (dimensions, duration, etc.)
    ///
    /// This method should validate the file format and extract
    /// media-specific metadata like dimensions for images or
    /// duration for audio/video.
    async fn extract_metadata(&self, data: &[u8]) -> Result<Self::Metadata, AppError>;

    /// Sanitize file data (remove EXIF, normalize format, etc.)
    ///
    /// This method can optionally modify the file data, for example
    /// removing EXIF metadata from images or normalizing audio formats.
    async fn sanitize(&self, data: Vec<u8>) -> Result<Vec<u8>, AppError>;
}
