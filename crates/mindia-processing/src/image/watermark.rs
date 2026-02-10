use image::{imageops, DynamicImage, ImageReader};
use std::io::Cursor;
use uuid::Uuid;

/// Watermark configuration
#[derive(Debug, Clone)]
pub struct WatermarkConfig {
    pub watermark_id: Uuid,
    pub position: WatermarkPosition,
    pub size: WatermarkSize,
    pub opacity: f32,
}

/// Watermark position
#[derive(Debug, Clone)]
pub enum WatermarkPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    Custom { x: u32, y: u32 },
}

/// Watermark size
#[derive(Debug, Clone)]
pub enum WatermarkSize {
    Absolute { width: u32, height: u32 },
    Relative { percent: f32 },
}

pub struct Watermark;

impl Watermark {
    /// Apply watermark to image
    pub fn apply(
        img: DynamicImage,
        watermark_data: &[u8],
        config: &WatermarkConfig,
        select_filter: fn(u32, u32, u32, u32) -> image::imageops::FilterType,
    ) -> Result<DynamicImage, anyhow::Error> {
        // Load watermark image
        let cursor = Cursor::new(watermark_data);
        let reader = ImageReader::new(cursor).with_guessed_format()?;
        let mut watermark_img = reader.decode()?.to_rgba8();

        use image::GenericImageView;
        let (img_width, img_height) = img.dimensions();
        let (wm_width, wm_height) = watermark_img.dimensions();

        // Calculate watermark size
        let (target_wm_width, target_wm_height) = match config.size {
            WatermarkSize::Absolute { width, height } => {
                (width.min(img_width), height.min(img_height))
            }
            WatermarkSize::Relative { percent } => {
                let w = (img_width as f32 * percent / 100.0).round() as u32;
                let h = (img_height as f32 * percent / 100.0).round() as u32;
                (w.max(1).min(img_width), h.max(1).min(img_height))
            }
        };

        // Resize watermark if needed
        if wm_width != target_wm_width || wm_height != target_wm_height {
            let filter = select_filter(wm_width, wm_height, target_wm_width, target_wm_height);
            let resized = DynamicImage::ImageRgba8(watermark_img);
            let resized = resized.resize_exact(target_wm_width, target_wm_height, filter);
            watermark_img = resized.to_rgba8();
        }

        // Apply opacity
        if config.opacity < 1.0 {
            for pixel in watermark_img.pixels_mut() {
                pixel[3] = (pixel[3] as f32 * config.opacity) as u8;
            }
        }

        // Calculate position
        let (x, y) = match config.position {
            WatermarkPosition::TopLeft => (0, 0),
            WatermarkPosition::TopRight => ((img_width as i64 - target_wm_width as i64).max(0), 0),
            WatermarkPosition::BottomLeft => {
                (0, (img_height as i64 - target_wm_height as i64).max(0))
            }
            WatermarkPosition::BottomRight => (
                (img_width as i64 - target_wm_width as i64).max(0),
                (img_height as i64 - target_wm_height as i64).max(0),
            ),
            WatermarkPosition::Center => (
                ((img_width as i64 - target_wm_width as i64) / 2).max(0),
                ((img_height as i64 - target_wm_height as i64) / 2).max(0),
            ),
            WatermarkPosition::Custom { x, y } => (x as i64, y as i64),
        };

        // Convert main image to RGBA if needed
        let mut img_rgba = img.to_rgba8();

        // Overlay watermark
        imageops::overlay(&mut img_rgba, &watermark_img, x, y);

        Ok(DynamicImage::ImageRgba8(img_rgba))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
    use std::io::Cursor;

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            width,
            height,
            Rgba([255, 255, 255, 255]),
        ))
    }

    fn create_test_watermark() -> Vec<u8> {
        let img = RgbaImage::from_pixel(50, 50, Rgba([0, 0, 0, 255]));
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
        buffer
    }

    fn dummy_filter(_w1: u32, _h1: u32, _w2: u32, _h2: u32) -> image::imageops::FilterType {
        image::imageops::FilterType::Triangle
    }

    #[test]
    fn test_watermark_top_left() {
        let img = create_test_image(200, 200);
        let watermark_data = create_test_watermark();
        let config = WatermarkConfig {
            watermark_id: uuid::Uuid::new_v4(),
            position: WatermarkPosition::TopLeft,
            size: WatermarkSize::Absolute {
                width: 50,
                height: 50,
            },
            opacity: 1.0,
        };

        let result = Watermark::apply(img, &watermark_data, &config, dummy_filter).unwrap();
        assert_eq!(result.dimensions(), (200, 200));
    }

    #[test]
    fn test_watermark_bottom_right() {
        let img = create_test_image(200, 200);
        let watermark_data = create_test_watermark();
        let config = WatermarkConfig {
            watermark_id: uuid::Uuid::new_v4(),
            position: WatermarkPosition::BottomRight,
            size: WatermarkSize::Absolute {
                width: 50,
                height: 50,
            },
            opacity: 1.0,
        };

        let result = Watermark::apply(img, &watermark_data, &config, dummy_filter).unwrap();
        assert_eq!(result.dimensions(), (200, 200));
    }

    #[test]
    fn test_watermark_center() {
        let img = create_test_image(200, 200);
        let watermark_data = create_test_watermark();
        let config = WatermarkConfig {
            watermark_id: uuid::Uuid::new_v4(),
            position: WatermarkPosition::Center,
            size: WatermarkSize::Absolute {
                width: 50,
                height: 50,
            },
            opacity: 1.0,
        };

        let result = Watermark::apply(img, &watermark_data, &config, dummy_filter).unwrap();
        assert_eq!(result.dimensions(), (200, 200));
    }

    #[test]
    fn test_watermark_custom_position() {
        let img = create_test_image(200, 200);
        let watermark_data = create_test_watermark();
        let config = WatermarkConfig {
            watermark_id: uuid::Uuid::new_v4(),
            position: WatermarkPosition::Custom { x: 10, y: 20 },
            size: WatermarkSize::Absolute {
                width: 50,
                height: 50,
            },
            opacity: 1.0,
        };

        let result = Watermark::apply(img, &watermark_data, &config, dummy_filter).unwrap();
        assert_eq!(result.dimensions(), (200, 200));
    }

    #[test]
    fn test_watermark_relative_size() {
        let img = create_test_image(200, 200);
        let watermark_data = create_test_watermark();
        let config = WatermarkConfig {
            watermark_id: uuid::Uuid::new_v4(),
            position: WatermarkPosition::TopLeft,
            size: WatermarkSize::Relative { percent: 25.0 }, // 25% of image size = 50x50
            opacity: 1.0,
        };

        let result = Watermark::apply(img, &watermark_data, &config, dummy_filter).unwrap();
        assert_eq!(result.dimensions(), (200, 200));
    }

    #[test]
    fn test_watermark_opacity() {
        let img = create_test_image(200, 200);
        let watermark_data = create_test_watermark();
        let config = WatermarkConfig {
            watermark_id: uuid::Uuid::new_v4(),
            position: WatermarkPosition::TopLeft,
            size: WatermarkSize::Absolute {
                width: 50,
                height: 50,
            },
            opacity: 0.5, // 50% opacity
        };

        let result = Watermark::apply(img, &watermark_data, &config, dummy_filter).unwrap();
        assert_eq!(result.dimensions(), (200, 200));
    }

    #[test]
    fn test_watermark_size_larger_than_image() {
        let img = create_test_image(100, 100);
        let watermark_data = create_test_watermark();
        let config = WatermarkConfig {
            watermark_id: uuid::Uuid::new_v4(),
            position: WatermarkPosition::TopLeft,
            size: WatermarkSize::Absolute {
                width: 200,
                height: 200,
            }, // Larger than image
            opacity: 1.0,
        };

        // Should clamp to image size
        let result = Watermark::apply(img, &watermark_data, &config, dummy_filter).unwrap();
        assert_eq!(result.dimensions(), (100, 100));
    }
}
