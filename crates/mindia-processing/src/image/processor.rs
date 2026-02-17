//! Image processor - metadata extraction and validation

use crate::metadata::ImageMetadata;
use crate::traits::MediaProcessor;
use async_trait::async_trait;
use image::GenericImageView;
use image::ImageReader;
use img_parts::{jpeg::Jpeg, png::Png, ImageEXIF};
use std::io::Cursor;

pub struct ImageProcessor;

#[async_trait]
impl MediaProcessor for ImageProcessor {
    type Metadata = ImageMetadata;

    async fn extract_metadata(&self, data: &[u8]) -> Result<Self::Metadata, anyhow::Error> {
        let cursor = Cursor::new(data);
        let reader = ImageReader::new(cursor).with_guessed_format()?;
        let format = reader
            .format()
            .map(|f| format!("{:?}", f))
            .unwrap_or_else(|| "unknown".to_string());
        let img = reader.decode()?;

        let (width, height) = img.dimensions();

        // Get EXIF orientation
        let exif_orientation = Self::read_exif_orientation(data);

        Ok(ImageMetadata {
            width,
            height,
            format,
            size_bytes: Some(data.len() as u64),
            exif_orientation: if exif_orientation != 1 {
                Some(exif_orientation)
            } else {
                None
            },
            color_space: None, // Could be extracted from EXIF if needed
        })
    }

    fn validate(&self, data: &[u8]) -> Result<(), anyhow::Error> {
        let cursor = Cursor::new(data);
        let reader = ImageReader::new(cursor).with_guessed_format()?;
        reader.decode()?;
        Ok(())
    }

    fn get_dimensions(&self, data: &[u8]) -> Option<(u32, u32)> {
        let cursor = Cursor::new(data);
        let reader = ImageReader::new(cursor).with_guessed_format().ok()?;
        let img = reader.decode().ok()?;
        Some(img.dimensions())
    }
}

impl ImageProcessor {
    /// Remove EXIF metadata from image
    pub fn remove_exif(data: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        // Try to parse as JPEG first
        if let Ok(mut jpeg) = Jpeg::from_bytes(data.to_vec().into()) {
            jpeg.set_exif(None);
            return Ok(jpeg.encoder().bytes().to_vec());
        }

        // Try to parse as PNG
        if let Ok(mut png) = Png::from_bytes(data.to_vec().into()) {
            png.set_exif(None);
            return Ok(png.encoder().bytes().to_vec());
        }

        // If neither JPEG nor PNG, return original data
        Ok(data.to_vec())
    }

    /// Read EXIF orientation tag from image data.
    ///
    /// Returns orientation value (1–8) or 1 (normal) if not found. **Callers must not rely on
    /// orientation from this crate for correct display** until a real implementation is added:
    /// this is a stub that always returns 1. EXIF parsing (e.g. via `kamadak-exif`) is optional
    /// and not yet wired here; use `get_orientation_transforms` only when orientation is provided
    /// by another source.
    pub fn read_exif_orientation(_data: &[u8]) -> u8 {
        1
    }

    /// Validate image and get dimensions
    /// Returns dimensions if image is valid, error otherwise
    pub fn validate_and_get_dimensions(data: &[u8]) -> Result<Option<(u32, u32)>, anyhow::Error> {
        let processor = ImageProcessor;
        processor.validate(data)?;
        Ok(processor.get_dimensions(data))
    }

    /// Get rotation and flip operations needed for a given EXIF orientation
    /// Returns (rotate_angle, flip_horizontal, flip_vertical)
    pub fn get_orientation_transforms(orientation: u8) -> (Option<u16>, bool, bool) {
        match orientation {
            1 => (None, false, false),      // Normal
            2 => (None, true, false),       // Mirror horizontal
            3 => (Some(180), false, false), // Rotate 180
            4 => (None, false, true),       // Mirror vertical
            5 => (Some(270), true, false),  // Mirror horizontal + Rotate 270 CW
            6 => (Some(90), false, false),  // Rotate 90 CW
            7 => (Some(90), true, false),   // Mirror horizontal + Rotate 90 CW
            8 => (Some(270), false, false), // Rotate 270 CW
            _ => (None, false, false),      // Invalid, treat as normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    fn create_test_image() -> Vec<u8> {
        let img = RgbaImage::from_pixel(100, 100, Rgba([255, 0, 0, 255]));
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        img.write_to(&mut cursor, ImageFormat::Png).unwrap();
        buffer
    }

    #[tokio::test]
    async fn test_extract_metadata() {
        let processor = ImageProcessor;
        let image_data = create_test_image();

        let metadata = processor.extract_metadata(&image_data).await.unwrap();

        assert_eq!(metadata.width, 100);
        assert_eq!(metadata.height, 100);
        assert_eq!(metadata.format, "Png");
        assert!(metadata.size_bytes.is_some());
        assert_eq!(metadata.size_bytes.unwrap(), image_data.len() as u64);
    }

    #[tokio::test]
    async fn test_extract_metadata_invalid_image() {
        let processor = ImageProcessor;
        let invalid_data = b"not an image";

        let result = processor.extract_metadata(invalid_data).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_image() {
        let processor = ImageProcessor;
        let image_data = create_test_image();

        let result = processor.validate(&image_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_invalid_image() {
        let processor = ImageProcessor;
        let invalid_data = b"not an image";

        let result = processor.validate(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_dimensions() {
        let processor = ImageProcessor;
        let image_data = create_test_image();

        let dimensions = processor.get_dimensions(&image_data);
        assert_eq!(dimensions, Some((100, 100)));
    }

    #[test]
    fn test_get_dimensions_invalid() {
        let processor = ImageProcessor;
        let invalid_data = b"not an image";

        let dimensions = processor.get_dimensions(invalid_data);
        assert_eq!(dimensions, None);
    }

    #[test]
    fn test_read_exif_orientation_no_exif() {
        // Image without EXIF should return 1 (normal)
        let image_data = create_test_image();
        let orientation = ImageProcessor::read_exif_orientation(&image_data);
        assert_eq!(orientation, 1);
    }

    #[test]
    fn test_get_orientation_transforms_all_values() {
        // Test all valid orientations (1-8)
        for orientation in 1..=8 {
            let (rotate, _flip_h, _flip_v) =
                ImageProcessor::get_orientation_transforms(orientation);

            // Verify the transforms are valid
            if let Some(angle) = rotate {
                assert!([90, 180, 270].contains(&angle));
            }
        }
    }

    #[test]
    fn test_get_orientation_transforms_invalid() {
        // Invalid orientations should return normal (no transforms)
        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(0);
        assert_eq!(rotate, None);
        assert!(!flip_h);
        assert!(!flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(9);
        assert_eq!(rotate, None);
        assert!(!flip_h);
        assert!(!flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(255);
        assert_eq!(rotate, None);
        assert!(!flip_h);
        assert!(!flip_v);
    }

    #[test]
    fn test_remove_exif_png() {
        // PNG without EXIF should return original data
        let image_data = create_test_image();
        let result = ImageProcessor::remove_exif(&image_data).unwrap();
        // For PNG without EXIF, should return original
        assert!(!result.is_empty());
    }

    #[test]
    fn test_remove_exif_invalid_format() {
        // Invalid format should return original data
        let invalid_data = b"not an image";
        let result = ImageProcessor::remove_exif(invalid_data).unwrap();
        assert_eq!(result, invalid_data.to_vec());
    }
}
