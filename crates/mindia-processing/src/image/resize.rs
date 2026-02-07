use image::{imageops, DynamicImage, GenericImageView, Rgba, RgbaImage};

/// Stretch mode for image resizing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StretchMode {
    #[default]
    On, // Allow upscaling
    Off,  // Don't upscale (preserve original if target is larger)
    Fill, // Fill with white background if upscaling
}

/// Resize dimensions specification
#[derive(Debug, Clone, Copy)]
pub struct ResizeDimensions {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl ResizeDimensions {
    /// Parse dimensions from string format: "WxH", "Wx", or "xH"
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('x').collect();

        if parts.len() != 2 {
            return Err("Invalid dimensions format. Expected: WxH, Wx, or xH".to_string());
        }

        let width = if parts[0].is_empty() {
            None
        } else {
            Some(
                parts[0]
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid width: {}", parts[0]))?,
            )
        };

        let height = if parts[1].is_empty() {
            None
        } else {
            Some(
                parts[1]
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid height: {}", parts[1]))?,
            )
        };

        if width.is_none() && height.is_none() {
            return Err("At least one dimension must be specified".to_string());
        }

        Ok(ResizeDimensions { width, height })
    }
}

/// Image resize operations
pub struct ImageResize;

impl ImageResize {
    /// Calculate target dimensions based on resize specification
    pub fn calculate_dimensions(
        orig_width: u32,
        orig_height: u32,
        dimensions: ResizeDimensions,
        _stretch_mode: StretchMode,
    ) -> (u32, u32) {
        match (dimensions.width, dimensions.height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => {
                let aspect_ratio = orig_height as f32 / orig_width as f32;
                let h = (w as f32 * aspect_ratio).round() as u32;
                (w, h.max(1))
            }
            (None, Some(h)) => {
                let aspect_ratio = orig_width as f32 / orig_height as f32;
                let w = (h as f32 * aspect_ratio).round() as u32;
                (w.max(1), h)
            }
            (None, None) => (orig_width, orig_height),
        }
    }

    /// Select appropriate filter type based on resize ratio
    pub fn select_filter(
        orig_width: u32,
        orig_height: u32,
        new_width: u32,
        new_height: u32,
    ) -> image::imageops::FilterType {
        let width_ratio = orig_width as f32 / new_width as f32;
        let height_ratio = orig_height as f32 / new_height as f32;
        let max_ratio = width_ratio.max(height_ratio);

        if max_ratio > 2.0 {
            image::imageops::FilterType::Triangle
        } else if max_ratio > 1.5 {
            image::imageops::FilterType::CatmullRom
        } else {
            image::imageops::FilterType::Lanczos3
        }
    }

    /// Resize image to exact dimensions
    pub fn resize_image(img: &DynamicImage, width: u32, height: u32) -> DynamicImage {
        let (orig_width, orig_height) = img.dimensions();
        let filter = Self::select_filter(orig_width, orig_height, width, height);
        img.resize_exact(width, height, filter)
    }

    /// Resize image with fill (for upscaling with white background)
    pub fn resize_with_fill(
        img: &DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> DynamicImage {
        let (orig_width, orig_height) = img.dimensions();

        let scale_width = target_width as f32 / orig_width as f32;
        let scale_height = target_height as f32 / orig_height as f32;
        let scale = scale_width.min(scale_height).min(1.0);

        let scaled_width = (orig_width as f32 * scale).round() as u32;
        let scaled_height = (orig_height as f32 * scale).round() as u32;

        let bg_color = Rgba([255u8, 255u8, 255u8, 255u8]);
        let canvas_img = RgbaImage::from_pixel(target_width, target_height, bg_color);
        let mut canvas = DynamicImage::ImageRgba8(canvas_img);

        let x_offset = (target_width - scaled_width) / 2;
        let y_offset = (target_height - scaled_height) / 2;

        if scale < 1.0 {
            let filter = Self::select_filter(orig_width, orig_height, scaled_width, scaled_height);
            let resized = img.resize_exact(scaled_width, scaled_height, filter);
            imageops::overlay(&mut canvas, &resized, x_offset as i64, y_offset as i64);
        } else {
            imageops::overlay(&mut canvas, img, x_offset as i64, y_offset as i64);
        }

        canvas
    }

    /// Apply resize with stretch mode handling
    pub fn apply_resize(
        img: &DynamicImage,
        dimensions: ResizeDimensions,
        stretch_mode: StretchMode,
    ) -> DynamicImage {
        let (orig_width, orig_height) = img.dimensions();
        let (target_width, target_height) =
            Self::calculate_dimensions(orig_width, orig_height, dimensions, stretch_mode);

        match stretch_mode {
            StretchMode::On => Self::resize_image(img, target_width, target_height),
            StretchMode::Off => {
                if target_width > orig_width || target_height > orig_height {
                    img.clone()
                } else {
                    Self::resize_image(img, target_width, target_height)
                }
            }
            StretchMode::Fill => {
                if target_width > orig_width || target_height > orig_height {
                    Self::resize_with_fill(img, target_width, target_height)
                } else {
                    Self::resize_image(img, target_width, target_height)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgba, RgbaImage};

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
    fn test_calculate_dimensions_both_specified() {
        let (w, h) = ImageResize::calculate_dimensions(
            100,
            100,
            ResizeDimensions {
                width: Some(50),
                height: Some(75),
            },
            StretchMode::On,
        );
        assert_eq!(w, 50);
        assert_eq!(h, 75);
    }

    #[test]
    fn test_calculate_dimensions_width_only() {
        let (w, h) = ImageResize::calculate_dimensions(
            100,
            50,
            ResizeDimensions {
                width: Some(200),
                height: None,
            },
            StretchMode::On,
        );
        assert_eq!(w, 200);
        // Height should maintain aspect ratio: 50/100 * 200 = 100
        assert_eq!(h, 100);
    }

    #[test]
    fn test_calculate_dimensions_height_only() {
        let (w, h) = ImageResize::calculate_dimensions(
            100,
            50,
            ResizeDimensions {
                width: None,
                height: Some(100),
            },
            StretchMode::On,
        );
        // Width should maintain aspect ratio: 100/50 * 100 = 200
        assert_eq!(w, 200);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_resize_image() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 0, 0, 255])));
        let resized = ImageResize::resize_image(&img, 50, 50);
        assert_eq!(resized.dimensions(), (50, 50));
    }

    #[test]
    fn test_resize_with_fill() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(50, 50, Rgba([255, 0, 0, 255])));
        let resized = ImageResize::resize_with_fill(&img, 100, 100);
        assert_eq!(resized.dimensions(), (100, 100));
    }

    #[test]
    fn test_apply_resize_stretch_on() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(50, 50, Rgba([255, 0, 0, 255])));
        let resized = ImageResize::apply_resize(
            &img,
            ResizeDimensions {
                width: Some(100),
                height: Some(100),
            },
            StretchMode::On,
        );
        assert_eq!(resized.dimensions(), (100, 100));
    }

    #[test]
    fn test_apply_resize_stretch_off() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(50, 50, Rgba([255, 0, 0, 255])));
        // Upscaling with StretchMode::Off should return original
        let resized = ImageResize::apply_resize(
            &img,
            ResizeDimensions {
                width: Some(100),
                height: Some(100),
            },
            StretchMode::Off,
        );
        assert_eq!(resized.dimensions(), (50, 50)); // Original size preserved

        // Downscaling should work
        let resized = ImageResize::apply_resize(
            &img,
            ResizeDimensions {
                width: Some(25),
                height: Some(25),
            },
            StretchMode::Off,
        );
        assert_eq!(resized.dimensions(), (25, 25));
    }

    #[test]
    fn test_apply_resize_stretch_fill() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(50, 50, Rgba([255, 0, 0, 255])));
        let resized = ImageResize::apply_resize(
            &img,
            ResizeDimensions {
                width: Some(100),
                height: Some(100),
            },
            StretchMode::Fill,
        );
        assert_eq!(resized.dimensions(), (100, 100));
    }
}
