//! Image transformer - orchestrates all image transformations
//!
//! This module provides the main `ImageTransformer` that chains together
//! various transform operations (resize, orientation, filters, watermark, etc.)

use crate::compression::{FormatSelector, ImageCompressor, OutputFormat, QualityPreset};
use crate::image::filters::{FilterConfig, ImageFilters};
use crate::image::orientation::ImageOrientation;
use crate::image::resize::{ImageResize, ResizeDimensions, StretchMode};
use crate::image::smart_crop::{SmartCrop, SmartCropConfig};
use crate::image::watermark::{Watermark, WatermarkConfig};
use bytes::Bytes;
use image::GenericImageView;
use std::io::Cursor;

/// Main image transformer that orchestrates all transform operations
pub struct ImageTransformer;

impl ImageTransformer {
    /// Simple resize operation - entry point for basic resizing
    pub fn resize(
        data: &[u8],
        dimensions: ResizeDimensions,
        stretch_mode: StretchMode,
        format: image::ImageFormat,
    ) -> Result<Bytes, anyhow::Error> {
        let cursor = Cursor::new(data);
        let img = image::ImageReader::new(cursor)
            .with_guessed_format()?
            .decode()?;

        let resized = ImageResize::apply_resize(&img, dimensions, stretch_mode);

        let (width, height) = resized.dimensions();
        let estimated_size = (width * height * 3) as usize;
        let mut buffer = Vec::with_capacity(estimated_size);
        let mut cursor = Cursor::new(&mut buffer);
        resized.write_to(&mut cursor, format)?;

        Ok(Bytes::from(buffer))
    }

    /// Detect image format from content type
    pub fn detect_format(content_type: &str) -> image::ImageFormat {
        match content_type {
            "image/jpeg" | "image/jpg" => image::ImageFormat::Jpeg,
            "image/png" => image::ImageFormat::Png,
            "image/gif" => image::ImageFormat::Gif,
            "image/webp" => image::ImageFormat::WebP,
            _ => image::ImageFormat::Jpeg,
        }
    }

    /// Transform image with resizing and compression
    /// Returns (compressed_data, output_content_type)
    ///
    /// This is the main transformation pipeline that chains all transforms together:
    /// 1. EXIF auto-rotation (if enabled)
    /// 2. Manual rotation/flip
    /// 3. Smart crop (if requested)
    /// 4. Resize (if requested)
    /// 5. Watermark (if requested)
    /// 6. Filters (if requested)
    /// 7. Format selection and compression
    #[allow(clippy::too_many_arguments)]
    pub fn transform_with_compression(
        data: &[u8],
        resize_dims: Option<ResizeDimensions>,
        stretch_mode: StretchMode,
        requested_format: Option<OutputFormat>,
        quality: QualityPreset,
        accept_header: Option<&str>,
        original_content_type: &str,
        rotate_angle: Option<u16>,
        flip_horizontal: bool,
        flip_vertical: bool,
        autorotate: bool,
        smart_crop: Option<SmartCropConfig>,
        watermark: Option<WatermarkConfig>,
        watermark_data: Option<&[u8]>,
        filter_config: Option<FilterConfig>,
    ) -> Result<(Bytes, String), anyhow::Error> {
        let cursor = Cursor::new(data);
        let mut img = image::ImageReader::new(cursor)
            .with_guessed_format()?
            .decode()?;

        // Step 1: Apply EXIF auto-rotation if enabled (default)
        if autorotate {
            img = ImageOrientation::apply_exif_orientation(img, data);
        }

        // Step 2: Apply manual rotation if requested
        if let Some(angle) = rotate_angle {
            tracing::debug!(angle = angle, "Applying manual rotation");
            img = ImageOrientation::rotate_by_angle(img, angle);
        }

        // Step 3: Apply flip/mirror operations
        if flip_horizontal {
            tracing::debug!("Applying horizontal flip (mirror)");
            img = ImageOrientation::apply_flip_horizontal(img);
        }
        if flip_vertical {
            tracing::debug!("Applying vertical flip");
            img = ImageOrientation::apply_flip_vertical(img);
        }

        // Step 4: Apply smart crop if requested (before resize)
        if let Some(ref crop_config) = smart_crop {
            tracing::debug!(
                width = crop_config.width,
                height = crop_config.height,
                "Applying smart crop"
            );
            img = SmartCrop::crop(img, crop_config.width, crop_config.height)?;
        }

        // Step 5: Apply resize if requested
        if let Some(dims) = resize_dims {
            img = ImageResize::apply_resize(&img, dims, stretch_mode);
        }

        // Step 6: Apply watermark if requested (after resize, before compression)
        let has_watermark = watermark.is_some();
        if let Some(ref watermark_config) = watermark {
            if let Some(watermark_bytes) = watermark_data {
                tracing::debug!(watermark_id = %watermark_config.watermark_id, "Applying watermark");
                img = Watermark::apply(
                    img,
                    watermark_bytes,
                    watermark_config,
                    ImageResize::select_filter,
                )?;
            }
        }

        // Step 7: Apply filters if requested (after resize/watermark, before compression)
        if let Some(ref filter_cfg) = filter_config {
            tracing::debug!("Applying image filters");
            img = ImageFilters::apply(img, filter_cfg);
        }

        // Determine output format
        let output_format = if let Some(fmt) = requested_format {
            // If format is explicitly requested, use it (or select if auto)
            if fmt == OutputFormat::Auto {
                FormatSelector::select_format(accept_header, &img, fmt)
            } else {
                fmt
            }
        } else {
            // No format specified - preserve original or use auto
            // If any transformation was applied, we need to re-encode, so use auto
            if resize_dims.is_some()
                || smart_crop.is_some()
                || has_watermark
                || filter_config.is_some()
            {
                FormatSelector::select_format(accept_header, &img, OutputFormat::Auto)
            } else {
                // No transformation, detect from original content type
                match original_content_type {
                    "image/jpeg" | "image/jpg" => OutputFormat::Jpeg,
                    "image/png" => OutputFormat::Png,
                    "image/webp" => OutputFormat::WebP,
                    "image/avif" => OutputFormat::Avif,
                    _ => OutputFormat::Jpeg,
                }
            }
        };

        tracing::debug!(
            requested_format = ?requested_format,
            output_format = ?output_format,
            quality = ?quality,
            has_resize = resize_dims.is_some(),
            "Applying compression"
        );

        // Compress with selected format and quality (adaptive quality enabled)
        let (compressed_data, actual_format) = ImageCompressor::compress(
            &img,
            output_format,
            quality,
            true, // enable adaptive quality
        )?;

        let content_type = actual_format.to_mime_type().to_string();

        Ok((compressed_data, content_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::processor::ImageProcessor;
    use crate::WatermarkPosition;
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

    #[test]
    fn test_parse_dimensions() {
        let dims = ResizeDimensions::parse("320x240").unwrap();
        assert_eq!(dims.width, Some(320));
        assert_eq!(dims.height, Some(240));

        let dims = ResizeDimensions::parse("320x").unwrap();
        assert_eq!(dims.width, Some(320));
        assert_eq!(dims.height, None);

        let dims = ResizeDimensions::parse("x240").unwrap();
        assert_eq!(dims.width, None);
        assert_eq!(dims.height, Some(240));

        assert!(ResizeDimensions::parse("x").is_err());
        assert!(ResizeDimensions::parse("abc").is_err());
    }

    #[test]
    fn test_exif_orientation_transforms() {
        // Test all EXIF orientations (1-8)
        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(1);
        assert_eq!(rotate, None);
        assert!(!flip_h);
        assert!(!flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(2);
        assert_eq!(rotate, None);
        assert!(flip_h);
        assert!(!flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(3);
        assert_eq!(rotate, Some(180));
        assert!(!flip_h);
        assert!(!flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(4);
        assert_eq!(rotate, None);
        assert!(!flip_h);
        assert!(flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(5);
        assert_eq!(rotate, Some(270));
        assert!(flip_h);
        assert!(!flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(6);
        assert_eq!(rotate, Some(90));
        assert!(!flip_h);
        assert!(!flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(7);
        assert_eq!(rotate, Some(90));
        assert!(flip_h);
        assert!(!flip_v);

        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(8);
        assert_eq!(rotate, Some(270));
        assert!(!flip_h);
        assert!(!flip_v);

        // Test invalid orientation
        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(99);
        assert_eq!(rotate, None);
        assert!(!flip_h);
        assert!(!flip_v);
    }

    #[test]
    fn test_rotate_by_angle() {
        // Create a simple 2x2 test image
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])));

        // Test 90 degree rotation
        let rotated = ImageOrientation::rotate_by_angle(img.clone(), 90);
        assert_eq!(rotated.dimensions(), (2, 2)); // Dimensions for square image

        // Test 180 degree rotation
        let rotated = ImageOrientation::rotate_by_angle(img.clone(), 180);
        assert_eq!(rotated.dimensions(), (2, 2));

        // Test 270 degree rotation
        let rotated = ImageOrientation::rotate_by_angle(img.clone(), 270);
        assert_eq!(rotated.dimensions(), (2, 2));

        // Test invalid angle (should return original)
        let rotated = ImageOrientation::rotate_by_angle(img.clone(), 45);
        assert_eq!(rotated.dimensions(), img.dimensions());
    }

    #[test]
    fn test_flip_operations() {
        // Create a simple 2x3 test image (non-square to verify flip direction)
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 3, Rgba([0, 255, 0, 255])));

        // Test horizontal flip (mirror)
        let flipped = ImageOrientation::apply_flip_horizontal(img.clone());
        assert_eq!(flipped.dimensions(), (2, 3));

        // Test vertical flip
        let flipped = ImageOrientation::apply_flip_vertical(img.clone());
        assert_eq!(flipped.dimensions(), (2, 3));
    }

    #[test]
    fn test_rotation_dimension_changes() {
        // Create a non-square image to test dimension changes
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 2, Rgba([0, 0, 255, 255])));

        // Original dimensions
        assert_eq!(img.dimensions(), (4, 2));

        // 90 degree rotation should swap dimensions
        let rotated = ImageOrientation::rotate_by_angle(img.clone(), 90);
        assert_eq!(rotated.dimensions(), (2, 4)); // Width and height swapped

        // 180 degree rotation should keep dimensions
        let rotated = ImageOrientation::rotate_by_angle(img.clone(), 180);
        assert_eq!(rotated.dimensions(), (4, 2));

        // 270 degree rotation should swap dimensions
        let rotated = ImageOrientation::rotate_by_angle(img.clone(), 270);
        assert_eq!(rotated.dimensions(), (2, 4)); // Width and height swapped
    }

    #[test]
    fn test_watermark_position() {
        // Test watermark position enum
        let pos_tl = WatermarkPosition::TopLeft;
        let _pos_br = WatermarkPosition::BottomRight;
        let pos_custom = WatermarkPosition::Custom { x: 10, y: 20 };

        // Just verify they compile and can be matched
        match pos_tl {
            WatermarkPosition::TopLeft => {}
            _ => panic!(),
        }
        match pos_custom {
            WatermarkPosition::Custom { x, y } => {
                assert_eq!(x, 10);
                assert_eq!(y, 20);
            }
            _ => panic!(),
        }
    }
}
