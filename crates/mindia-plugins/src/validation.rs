//! Validation utilities for plugin operations
//!
//! This module provides common validation functions to ensure plugins
//! operate safely and within resource constraints.

use anyhow::Result;

/// Default maximum size for media files (100 MB)
pub const DEFAULT_MAX_MEDIA_SIZE: usize = 100 * 1024 * 1024;

/// Maximum size for audio files (500 MB)
pub const MAX_AUDIO_SIZE: usize = 500 * 1024 * 1024;

/// Maximum size for image files (50 MB)
pub const MAX_IMAGE_SIZE: usize = 50 * 1024 * 1024;

/// Maximum size for video files (2 GB)
pub const MAX_VIDEO_SIZE: usize = 2 * 1024 * 1024 * 1024;

/// Validate that data size is within acceptable limits
///
/// # Arguments
/// * `data` - The data to validate
/// * `max_size` - Maximum allowed size in bytes
/// * `media_type` - Type of media for error messages (e.g., "audio", "image")
///
/// # Returns
/// Ok(()) if size is within limits, otherwise an error
///
/// # Example
/// ```
/// # use mindia_plugins::validation::validate_size;
/// let data = vec![0u8; 1000];
/// assert!(validate_size(&data, 2000, "test").is_ok());
/// assert!(validate_size(&data, 500, "test").is_err());
/// ```
pub fn validate_size(data: &[u8], max_size: usize, media_type: &str) -> Result<()> {
    let size = data.len();

    if size > max_size {
        return Err(anyhow::anyhow!(
            "{} file size ({} bytes) exceeds maximum allowed size ({} bytes). \
             Consider uploading a smaller file or increasing the size limit.",
            media_type,
            size,
            max_size
        ));
    }

    tracing::debug!(
        media_type = media_type,
        size = size,
        max_size = max_size,
        "Media size validation passed"
    );

    Ok(())
}

/// Validate audio file size
pub fn validate_audio_size(data: &[u8]) -> Result<()> {
    validate_size(data, MAX_AUDIO_SIZE, "Audio")
}

/// Validate image file size
pub fn validate_image_size(data: &[u8]) -> Result<()> {
    validate_size(data, MAX_IMAGE_SIZE, "Image")
}

/// Validate video file size
pub fn validate_video_size(data: &[u8]) -> Result<()> {
    validate_size(data, MAX_VIDEO_SIZE, "Video")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_size_within_limit() {
        let data = vec![0u8; 1000];
        assert!(validate_size(&data, 2000, "test").is_ok());
    }

    #[test]
    fn test_validate_size_exceeds_limit() {
        let data = vec![0u8; 1000];
        let result = validate_size(&data, 500, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[test]
    fn test_validate_size_exact_limit() {
        let data = vec![0u8; 1000];
        assert!(validate_size(&data, 1000, "test").is_ok());
    }

    #[test]
    fn test_validate_audio_size() {
        let small_audio = vec![0u8; 1024];
        assert!(validate_audio_size(&small_audio).is_ok());

        let large_audio = vec![0u8; MAX_AUDIO_SIZE + 1];
        assert!(validate_audio_size(&large_audio).is_err());
    }

    #[test]
    fn test_validate_image_size() {
        let small_image = vec![0u8; 1024];
        assert!(validate_image_size(&small_image).is_ok());

        let large_image = vec![0u8; MAX_IMAGE_SIZE + 1];
        assert!(validate_image_size(&large_image).is_err());
    }

    #[test]
    fn test_validate_video_size() {
        let small_video = vec![0u8; 1024];
        assert!(validate_video_size(&small_video).is_ok());

        // Note: Can't easily test the upper bound due to memory constraints
    }
}
