//! Image upload processor: dimensions + EXIF sanitization.

use async_trait::async_trait;

use crate::image::{ImageDimensions, ImageProcessor};
use crate::upload::traits::UploadProcessor;

/// Image metadata from upload pipeline.
#[derive(Clone, Debug)]
pub struct UploadImageMetadata {
    pub dimensions: ImageDimensions,
}

/// Image upload processor.
pub struct ImageUploadProcessor {
    remove_exif: bool,
}

impl ImageUploadProcessor {
    pub fn new(remove_exif: bool) -> Self {
        Self { remove_exif }
    }
}

#[async_trait]
impl UploadProcessor for ImageUploadProcessor {
    type Metadata = UploadImageMetadata;

    async fn extract_metadata(&self, data: &[u8]) -> anyhow::Result<UploadImageMetadata> {
        let len = data.len();
        let data = data.to_vec();
        // Image decode is CPU-bound; run off the async pool to avoid blocking other tasks.
        let dims =
            tokio::task::spawn_blocking(move || ImageProcessor::validate_and_get_dimensions(&data))
                .await??;
        let (width, height) = dims.unwrap_or((0, 0));
        Ok(UploadImageMetadata {
            dimensions: ImageDimensions {
                width,
                height,
                format: "unknown".to_string(),
                size_bytes: Some(len as u64),
                exif_orientation: None,
                color_space: None,
            },
        })
    }

    async fn sanitize(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        if !self.remove_exif {
            return Ok(data);
        }
        let out = tokio::task::spawn_blocking(move || ImageProcessor::remove_exif(&data)).await??;
        Ok(out)
    }
}
