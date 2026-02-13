//! Image processor implementation

use async_trait::async_trait;
use mindia_core::AppError;
use mindia_processing::image::ImageDimensions;
use mindia_services::ImageProcessor;

use super::traits::MediaProcessor;

/// Image metadata extracted from file
#[derive(Clone)]
pub struct ImageMetadata {
    pub dimensions: ImageDimensions,
}

/// Image processor for handling image-specific operations
pub struct ImageProcessorImpl {
    remove_exif: bool,
}

impl ImageProcessorImpl {
    /// Create a new image processor
    pub fn new(remove_exif: bool) -> Self {
        Self { remove_exif }
    }
}

#[async_trait]
impl MediaProcessor for ImageProcessorImpl {
    type Metadata = ImageMetadata;

    /// Extract image dimensions
    async fn extract_metadata(&self, data: &[u8]) -> Result<Self::Metadata, AppError> {
        let data_clone = data.to_vec();
        let dims_opt = tokio::task::spawn_blocking(move || {
            ImageProcessor::validate_and_get_dimensions(&data_clone)
        })
        .await
        .map_err(|e| AppError::Internal(format!("Failed to process image: {}", e)))?
        .map_err(|e: anyhow::Error| AppError::InvalidInput(format!("Invalid image file: {}", e)))?;

        let (width, height) = dims_opt.unwrap_or((0, 0));
        let dimensions = ImageDimensions {
            width,
            height,
            format: "unknown".to_string(),
            size_bytes: Some(data.len() as u64),
            exif_orientation: None,
            color_space: None,
        };

        Ok(ImageMetadata { dimensions })
    }

    /// Sanitize image (remove EXIF if configured)
    async fn sanitize(&self, data: Vec<u8>) -> Result<Vec<u8>, AppError> {
        if !self.remove_exif {
            return Ok(data);
        }

        let data_clone = data.clone();
        let sanitized =
            tokio::task::spawn_blocking(move || ImageProcessor::remove_exif(&data_clone))
                .await
                .map_err(|e| AppError::Internal(format!("Failed to process image: {}", e)))?
                .map_err(|e: anyhow::Error| {
                    AppError::Internal(format!("Failed to remove EXIF data: {}", e))
                })?;

        tracing::debug!("EXIF metadata removed from image");
        Ok(sanitized)
    }
}
