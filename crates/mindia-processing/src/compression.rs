use anyhow::{anyhow, Result};
use bytes::Bytes;
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::io::Cursor;

/// Quality presets for image compression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityPreset {
    #[default]
    Normal, // Default quality, balanced size and quality
    Better,   // Higher quality, ≈125% file size
    Best,     // Near pristine quality, ≈170% file size
    Lighter,  // Smaller files, ≈80% file size
    Lightest, // Maximum compression, ≈50% file size
}

impl QualityPreset {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "normal" => Ok(QualityPreset::Normal),
            "better" => Ok(QualityPreset::Better),
            "best" => Ok(QualityPreset::Best),
            "lighter" => Ok(QualityPreset::Lighter),
            "lightest" => Ok(QualityPreset::Lightest),
            _ => Err(anyhow!("Invalid quality preset: {}", s)),
        }
    }

    /// Get quality value for JPEG (0-100)
    pub fn jpeg_quality(self) -> u8 {
        match self {
            QualityPreset::Normal => 75,
            QualityPreset::Better => 85,
            QualityPreset::Best => 95,
            QualityPreset::Lighter => 65,
            QualityPreset::Lightest => 50,
        }
    }

    /// Get quality value for WebP (0-100)
    pub fn webp_quality(self) -> f32 {
        match self {
            QualityPreset::Normal => 80.0,
            QualityPreset::Better => 90.0,
            QualityPreset::Best => 98.0,
            QualityPreset::Lighter => 70.0,
            QualityPreset::Lightest => 55.0,
        }
    }

    /// Get quality value for AVIF (0-100)
    pub fn avif_quality(self) -> u8 {
        match self {
            QualityPreset::Normal => 70,
            QualityPreset::Better => 80,
            QualityPreset::Best => 90,
            QualityPreset::Lighter => 60,
            QualityPreset::Lightest => 45,
        }
    }

    /// Get compression level for PNG (1-9, lossless)
    pub fn png_compression_level(self) -> u8 {
        6 // PNG is lossless, so we use a fixed good compression level
    }
}

/// Output format for compressed images
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Jpeg,
    Png,
    WebP,
    Avif,
    Auto, // Automatic format selection based on Accept header
}

impl OutputFormat {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "jpeg" | "jpg" => Ok(OutputFormat::Jpeg),
            "png" => Ok(OutputFormat::Png),
            "webp" => Ok(OutputFormat::WebP),
            "avif" => Ok(OutputFormat::Avif),
            "auto" => Ok(OutputFormat::Auto),
            _ => Err(anyhow!("Invalid format: {}", s)),
        }
    }

    pub fn to_mime_type(self) -> &'static str {
        match self {
            OutputFormat::Jpeg => "image/jpeg",
            OutputFormat::Png => "image/png",
            OutputFormat::WebP => "image/webp",
            OutputFormat::Avif => "image/avif",
            OutputFormat::Auto => "image/jpeg", // fallback
        }
    }

    pub fn to_image_format(self) -> ImageFormat {
        match self {
            OutputFormat::Jpeg => ImageFormat::Jpeg,
            OutputFormat::Png => ImageFormat::Png,
            OutputFormat::WebP => ImageFormat::WebP,
            OutputFormat::Avif => ImageFormat::Avif,
            OutputFormat::Auto => ImageFormat::Jpeg, // fallback
        }
    }
}

/// Image complexity analyzer for adaptive quality
pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    /// Analyze image complexity (0-100 score)
    /// Higher score = more complex = needs higher quality
    pub fn analyze(img: &DynamicImage) -> u32 {
        let (width, height) = img.dimensions();
        let pixel_count = (width * height) as usize;

        // Sample pixels for performance (max 10000 samples)
        let sample_rate = (pixel_count / 10000).max(1);

        let mut edge_strength_sum = 0u64;
        let mut color_variations = std::collections::HashSet::new();
        let mut sampled_pixels = 0;

        let rgba = img.to_rgba8();

        for y in (0..height).step_by(sample_rate) {
            for x in (0..width).step_by(sample_rate) {
                let pixel = rgba.get_pixel(x, y);

                // Track color diversity (quantized to reduce memory)
                let color_key = (pixel[0] / 32, pixel[1] / 32, pixel[2] / 32);
                color_variations.insert(color_key);

                // Calculate edge strength using simple gradient
                if x > 0 && y > 0 {
                    let prev_x = rgba.get_pixel(x - 1, y);
                    let prev_y = rgba.get_pixel(x, y - 1);

                    let dx = (pixel[0] as i32 - prev_x[0] as i32).abs()
                        + (pixel[1] as i32 - prev_x[1] as i32).abs()
                        + (pixel[2] as i32 - prev_x[2] as i32).abs();

                    let dy = (pixel[0] as i32 - prev_y[0] as i32).abs()
                        + (pixel[1] as i32 - prev_y[1] as i32).abs()
                        + (pixel[2] as i32 - prev_y[2] as i32).abs();

                    edge_strength_sum += (dx + dy) as u64;
                }

                sampled_pixels += 1;
            }
        }

        // Calculate complexity factors
        let avg_edge_strength = if sampled_pixels > 0 {
            (edge_strength_sum / sampled_pixels as u64) as u32
        } else {
            0
        };

        let color_diversity = color_variations.len() as u32;

        // Normalize edge strength (typical range 0-50, normalize to 0-50)
        let edge_score = avg_edge_strength.min(50);

        // Normalize color diversity (typical range 0-2000, normalize to 0-50)
        let color_score = (color_diversity * 50 / 2000).min(50);

        // Combined complexity score (0-100)
        edge_score + color_score
    }

    /// Adjust quality based on complexity
    /// Returns quality adjustment (-20 to +20)
    pub fn adaptive_quality_adjustment(complexity: u32) -> i32 {
        match complexity {
            0..=20 => -20,  // Very simple image, reduce quality significantly
            21..=40 => -10, // Simple image, reduce quality moderately
            41..=60 => 0,   // Medium complexity, use base quality
            61..=80 => 10,  // Complex image, increase quality moderately
            81..=100 => 20, // Very complex image, increase quality significantly
            _ => 0,
        }
    }
}

/// Format selector based on Accept header and image properties
pub struct FormatSelector;

impl FormatSelector {
    /// Select optimal format based on Accept header and image properties
    pub fn select_format(
        accept_header: Option<&str>,
        img: &DynamicImage,
        requested_format: OutputFormat,
    ) -> OutputFormat {
        // If explicit format requested, use it
        if requested_format != OutputFormat::Auto {
            return requested_format;
        }

        let accept = accept_header.unwrap_or("");
        let (width, height) = img.dimensions();
        let megapixels = (width * height) as f32 / 1_000_000.0;

        // Check if image has meaningful alpha channel
        let has_alpha = Self::has_meaningful_alpha(img);

        // AVIF: Best compression, but only for smaller images and if supported
        if accept.contains("image/avif") && megapixels < 2.0 {
            return OutputFormat::Avif;
        }

        // WebP: Great compression with wide support
        if accept.contains("image/webp") {
            return OutputFormat::WebP;
        }

        // PNG: For images with transparency
        if has_alpha {
            return OutputFormat::Png;
        }

        // JPEG: Default fallback for photos
        OutputFormat::Jpeg
    }

    /// Check if image has meaningful alpha channel (not fully opaque)
    fn has_meaningful_alpha(img: &DynamicImage) -> bool {
        // Only check if image format supports alpha
        match img {
            DynamicImage::ImageRgba8(_)
            | DynamicImage::ImageRgba16(_)
            | DynamicImage::ImageRgba32F(_) => {
                let rgba = img.to_rgba8();
                let (width, height) = img.dimensions();

                // Sample alpha channel (every 10th pixel for performance)
                for y in (0..height).step_by(10) {
                    for x in (0..width).step_by(10) {
                        let pixel = rgba.get_pixel(x, y);
                        if pixel[3] < 255 {
                            return true; // Found non-opaque pixel
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}

/// Main compression service
pub struct ImageCompressor;

impl ImageCompressor {
    /// Compress image with specified format and quality
    pub fn compress(
        img: &DynamicImage,
        format: OutputFormat,
        quality: QualityPreset,
        use_adaptive: bool,
    ) -> Result<(Bytes, OutputFormat)> {
        // Calculate adaptive quality if enabled
        let effective_quality = if use_adaptive {
            let complexity = ComplexityAnalyzer::analyze(img);
            let adjustment = ComplexityAnalyzer::adaptive_quality_adjustment(complexity);

            tracing::debug!(
                complexity = complexity,
                adjustment = adjustment,
                base_quality = ?quality,
                "Adaptive quality analysis"
            );

            quality // We'll apply adjustment in format-specific encoding
        } else {
            quality
        };

        let actual_format = format;

        let compressed_data = match actual_format {
            OutputFormat::Jpeg => Self::compress_jpeg(img, effective_quality, use_adaptive)?,
            OutputFormat::Png => Self::compress_png(img)?,
            OutputFormat::WebP => Self::compress_webp(img, effective_quality, use_adaptive)?,
            OutputFormat::Avif => Self::compress_avif(img, effective_quality, use_adaptive)?,
            OutputFormat::Auto => Self::compress_jpeg(img, effective_quality, use_adaptive)?, // fallback
        };

        Ok((compressed_data, actual_format))
    }

    /// Compress to JPEG using mozjpeg
    fn compress_jpeg(
        img: &DynamicImage,
        quality: QualityPreset,
        use_adaptive: bool,
    ) -> Result<Bytes> {
        let rgb_img = img.to_rgb8();
        let (width, height) = rgb_img.dimensions();

        let mut base_quality = quality.jpeg_quality();

        // Apply adaptive adjustment
        if use_adaptive {
            let complexity = ComplexityAnalyzer::analyze(img);
            let adjustment = ComplexityAnalyzer::adaptive_quality_adjustment(complexity);
            base_quality = (base_quality as i32 + adjustment).clamp(10, 100) as u8;
        }

        let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        comp.set_size(width as usize, height as usize);
        comp.set_quality(base_quality as f32);
        comp.set_progressive_mode();
        comp.set_optimize_coding(true);

        let mut comp = comp.start_compress(Vec::new())?;
        comp.write_scanlines(&rgb_img)?;
        let jpeg_data = comp.finish()?;

        Ok(Bytes::from(jpeg_data))
    }

    /// Compress to PNG
    fn compress_png(img: &DynamicImage) -> Result<Bytes> {
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        img.write_to(&mut cursor, ImageFormat::Png)?;

        Ok(Bytes::from(buffer))
    }

    /// Compress to WebP
    fn compress_webp(
        img: &DynamicImage,
        quality: QualityPreset,
        use_adaptive: bool,
    ) -> Result<Bytes> {
        let (width, height) = img.dimensions();

        let mut base_quality = quality.webp_quality();

        // Apply adaptive adjustment
        if use_adaptive {
            let complexity = ComplexityAnalyzer::analyze(img);
            let adjustment = ComplexityAnalyzer::adaptive_quality_adjustment(complexity);
            base_quality = (base_quality + adjustment as f32).clamp(10.0, 100.0);
        }

        // Convert to RGBA for WebP encoding
        let rgba_img = img.to_rgba8();

        let encoder = webp::Encoder::from_rgba(&rgba_img, width, height);
        let webp_data = encoder.encode(base_quality);

        Ok(Bytes::copy_from_slice(&webp_data))
    }

    /// Compress to AVIF
    fn compress_avif(
        img: &DynamicImage,
        quality: QualityPreset,
        use_adaptive: bool,
    ) -> Result<Bytes> {
        let (width, height) = img.dimensions();

        let mut base_quality = quality.avif_quality();

        // Apply adaptive adjustment
        if use_adaptive {
            let complexity = ComplexityAnalyzer::analyze(img);
            let adjustment = ComplexityAnalyzer::adaptive_quality_adjustment(complexity);
            base_quality = (base_quality as i32 + adjustment).clamp(10, 100) as u8;
        }

        // Convert to RGB for AVIF encoding
        let rgb_img = img.to_rgb8();
        let raw_pixels = rgb_img.as_raw();

        // Create RGB buffer
        let rgb_data: Vec<rgb::RGB8> = raw_pixels
            .chunks_exact(3)
            .map(|chunk| rgb::RGB8::new(chunk[0], chunk[1], chunk[2]))
            .collect();

        let img_buf = ravif::Img::new(rgb_data.as_slice(), width as usize, height as usize);

        let encoder = ravif::Encoder::new()
            .with_quality(base_quality as f32)
            .with_speed(6); // Balance between speed and compression

        let avif_data = encoder.encode_rgb(img_buf)?;

        Ok(Bytes::copy_from_slice(&avif_data.avif_file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_preset_parse() {
        assert_eq!(
            QualityPreset::parse("normal").unwrap(),
            QualityPreset::Normal
        );
        assert_eq!(
            QualityPreset::parse("better").unwrap(),
            QualityPreset::Better
        );
        assert_eq!(QualityPreset::parse("BEST").unwrap(), QualityPreset::Best);
        assert_eq!(
            QualityPreset::parse("lighter").unwrap(),
            QualityPreset::Lighter
        );
        assert_eq!(
            QualityPreset::parse("lightest").unwrap(),
            QualityPreset::Lightest
        );
        assert!(QualityPreset::parse("invalid").is_err());
    }

    #[test]
    fn test_output_format_parse() {
        assert_eq!(OutputFormat::parse("jpeg").unwrap(), OutputFormat::Jpeg);
        assert_eq!(OutputFormat::parse("png").unwrap(), OutputFormat::Png);
        assert_eq!(OutputFormat::parse("webp").unwrap(), OutputFormat::WebP);
        assert_eq!(OutputFormat::parse("avif").unwrap(), OutputFormat::Avif);
        assert_eq!(OutputFormat::parse("auto").unwrap(), OutputFormat::Auto);
        assert!(OutputFormat::parse("invalid").is_err());
    }

    #[test]
    fn test_jpeg_quality_values() {
        assert_eq!(QualityPreset::Normal.jpeg_quality(), 75);
        assert_eq!(QualityPreset::Better.jpeg_quality(), 85);
        assert_eq!(QualityPreset::Best.jpeg_quality(), 95);
        assert_eq!(QualityPreset::Lighter.jpeg_quality(), 65);
        assert_eq!(QualityPreset::Lightest.jpeg_quality(), 50);
    }

    #[test]
    fn test_webp_quality_values() {
        assert_eq!(QualityPreset::Normal.webp_quality(), 80.0);
        assert_eq!(QualityPreset::Better.webp_quality(), 90.0);
        assert_eq!(QualityPreset::Best.webp_quality(), 98.0);
        assert_eq!(QualityPreset::Lighter.webp_quality(), 70.0);
        assert_eq!(QualityPreset::Lightest.webp_quality(), 55.0);
    }

    #[test]
    fn test_avif_quality_values() {
        assert_eq!(QualityPreset::Normal.avif_quality(), 70);
        assert_eq!(QualityPreset::Better.avif_quality(), 80);
        assert_eq!(QualityPreset::Best.avif_quality(), 90);
        assert_eq!(QualityPreset::Lighter.avif_quality(), 60);
        assert_eq!(QualityPreset::Lightest.avif_quality(), 45);
    }

    #[test]
    fn test_png_compression_level() {
        // PNG compression level should be fixed
        assert_eq!(QualityPreset::Normal.png_compression_level(), 6);
        assert_eq!(QualityPreset::Best.png_compression_level(), 6);
        assert_eq!(QualityPreset::Lightest.png_compression_level(), 6);
    }

    #[test]
    fn test_output_format_to_mime_type() {
        assert_eq!(OutputFormat::Jpeg.to_mime_type(), "image/jpeg");
        assert_eq!(OutputFormat::Png.to_mime_type(), "image/png");
        assert_eq!(OutputFormat::WebP.to_mime_type(), "image/webp");
        assert_eq!(OutputFormat::Avif.to_mime_type(), "image/avif");
        assert_eq!(OutputFormat::Auto.to_mime_type(), "image/jpeg");
    }

    #[test]
    fn test_output_format_to_image_format() {
        assert_eq!(OutputFormat::Jpeg.to_image_format(), ImageFormat::Jpeg);
        assert_eq!(OutputFormat::Png.to_image_format(), ImageFormat::Png);
        assert_eq!(OutputFormat::WebP.to_image_format(), ImageFormat::WebP);
        assert_eq!(OutputFormat::Avif.to_image_format(), ImageFormat::Avif);
        assert_eq!(OutputFormat::Auto.to_image_format(), ImageFormat::Jpeg);
    }

    #[test]
    fn test_complexity_analyzer() {
        use image::{DynamicImage, Rgba, RgbaImage};

        // Create a simple solid color image (low complexity)
        let simple_img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255])));
        let complexity = ComplexityAnalyzer::analyze(&simple_img);
        assert!(complexity <= 100); // Should be within valid range

        // Create a more complex image (checkerboard pattern)
        let mut complex_img = RgbaImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                let color = if (x + y) % 20 < 10 {
                    Rgba([0, 0, 0, 255])
                } else {
                    Rgba([255, 255, 255, 255])
                };
                complex_img.put_pixel(x, y, color);
            }
        }
        let complex_dynamic = DynamicImage::ImageRgba8(complex_img);
        let complexity2 = ComplexityAnalyzer::analyze(&complex_dynamic);
        assert!(complexity2 <= 100);
        // More complex image should generally have higher complexity
        assert!(complexity2 >= complexity);
    }

    #[test]
    fn test_adaptive_quality_adjustment() {
        // Test all complexity ranges
        assert_eq!(ComplexityAnalyzer::adaptive_quality_adjustment(10), -20);
        assert_eq!(ComplexityAnalyzer::adaptive_quality_adjustment(30), -10);
        assert_eq!(ComplexityAnalyzer::adaptive_quality_adjustment(50), 0);
        assert_eq!(ComplexityAnalyzer::adaptive_quality_adjustment(70), 10);
        assert_eq!(ComplexityAnalyzer::adaptive_quality_adjustment(90), 20);
    }

    #[test]
    fn test_format_selector_has_alpha() {
        use image::{DynamicImage, Rgba, RgbaImage};

        // Image with transparency
        let mut img = RgbaImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 128])); // Semi-transparent
            }
        }
        let dynamic_img = DynamicImage::ImageRgba8(img);

        let has_alpha = FormatSelector::has_meaningful_alpha(&dynamic_img);
        assert!(has_alpha);
    }

    #[test]
    fn test_format_selector_no_alpha() {
        use image::{DynamicImage, Rgba, RgbaImage};

        // Image without transparency
        let img = RgbaImage::from_pixel(100, 100, Rgba([255, 0, 0, 255])); // Fully opaque
        let dynamic_img = DynamicImage::ImageRgba8(img);

        let has_alpha = FormatSelector::has_meaningful_alpha(&dynamic_img);
        assert!(!has_alpha);
    }

    #[test]
    fn test_format_selector_select_format_auto() {
        use image::{DynamicImage, Rgba, RgbaImage};

        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 0, 0, 255])));

        // With WebP in Accept header
        let format =
            FormatSelector::select_format(Some("image/webp, image/jpeg"), &img, OutputFormat::Auto);
        assert_eq!(format, OutputFormat::WebP);

        // With AVIF in Accept header (small image)
        let format =
            FormatSelector::select_format(Some("image/avif, image/webp"), &img, OutputFormat::Auto);
        assert_eq!(format, OutputFormat::Avif);

        // No Accept header, no alpha -> JPEG
        let format = FormatSelector::select_format(None, &img, OutputFormat::Auto);
        assert_eq!(format, OutputFormat::Jpeg);
    }

    #[test]
    fn test_format_selector_explicit_format() {
        use image::{DynamicImage, Rgba, RgbaImage};

        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 0, 0, 255])));

        // Explicit format should be used regardless of Accept header
        let format = FormatSelector::select_format(Some("image/webp"), &img, OutputFormat::Png);
        assert_eq!(format, OutputFormat::Png);
    }

    #[test]
    fn test_image_compressor_compress() {
        use image::{DynamicImage, Rgba, RgbaImage};

        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 0, 0, 255])));

        // Test JPEG compression
        let (data, format) = ImageCompressor::compress(
            &img,
            OutputFormat::Jpeg,
            QualityPreset::Normal,
            false, // no adaptive
        )
        .unwrap();

        assert!(!data.is_empty());
        assert_eq!(format, OutputFormat::Jpeg);

        // Test PNG compression
        let (data, format) =
            ImageCompressor::compress(&img, OutputFormat::Png, QualityPreset::Normal, false)
                .unwrap();

        assert!(!data.is_empty());
        assert_eq!(format, OutputFormat::Png);
    }
}
