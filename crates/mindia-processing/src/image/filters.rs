use image::{imageops, DynamicImage, GenericImageView, Rgba, RgbaImage};

/// Image filter configuration
#[derive(Debug, Clone, Default)]
pub struct FilterConfig {
    /// Blur radius (Gaussian blur)
    pub blur: Option<f32>,
    /// Sharpen intensity (0.0 to 1.0)
    pub sharpen: Option<f32>,
    /// Grayscale enabled
    pub grayscale: bool,
    /// Sepia tone enabled
    pub sepia: bool,
    /// Brightness adjustment (-100 to 100, where 0 is no change)
    pub brightness: Option<i32>,
    /// Contrast adjustment (-100 to 100, where 0 is no change)
    pub contrast: Option<f32>,
    /// Saturation adjustment (-100 to 100, where 0 is no change)
    pub saturation: Option<f32>,
    /// Invert colors enabled
    pub invert: bool,
}

pub struct ImageFilters;

impl ImageFilters {
    /// Apply all configured filters to an image
    pub fn apply(img: DynamicImage, config: &FilterConfig) -> DynamicImage {
        let mut result = img;

        // Apply filters in a sensible order:
        // 1. Blur first (as it's most computationally expensive and affects everything)
        if let Some(radius) = config.blur {
            result = Self::apply_blur(result, radius);
        }

        // 2. Sharpen (sharpening after blur can be useful)
        if let Some(intensity) = config.sharpen {
            result = Self::apply_sharpen(result, intensity);
        }

        // 3. Color adjustments (brightness, contrast, saturation)
        if let Some(brightness) = config.brightness {
            result = Self::adjust_brightness(result, brightness);
        }

        if let Some(contrast) = config.contrast {
            result = Self::adjust_contrast(result, contrast);
        }

        if let Some(saturation) = config.saturation {
            result = Self::adjust_saturation(result, saturation);
        }

        // 4. Color transforms (grayscale, sepia, invert)
        if config.grayscale {
            result = Self::apply_grayscale(result);
        }

        if config.sepia {
            result = Self::apply_sepia(result);
        }

        if config.invert {
            result = Self::apply_invert(result);
        }

        result
    }

    /// Apply Gaussian blur using box blur approximation
    /// This uses a simple box blur which approximates Gaussian blur
    #[cfg(feature = "image")]
    pub fn apply_blur(img: DynamicImage, radius: f32) -> DynamicImage {
        // Convert radius to integer iterations (box blur needs multiple passes)
        let iterations = (radius / 2.0).clamp(1.0, 10.0) as u32;
        let mut blurred = img.to_rgba8();

        // Apply box blur multiple times to approximate Gaussian blur
        for _ in 0..iterations {
            blurred = imageops::blur(&blurred, radius / iterations as f32);
        }

        DynamicImage::ImageRgba8(blurred)
    }

    #[cfg(not(feature = "image"))]
    pub fn apply_blur(img: DynamicImage, _radius: f32) -> DynamicImage {
        // Fallback: no-op if image not available
        img
    }

    /// Apply sharpen filter using unsharp mask
    /// This applies a sharpen effect by enhancing edges
    #[cfg(feature = "image")]
    pub fn apply_sharpen(img: DynamicImage, intensity: f32) -> DynamicImage {
        let (width, height) = img.dimensions();
        let rgba8 = img.to_rgba8();
        let mut sharpened = RgbaImage::new(width, height);

        // Simple sharpening kernel applied manually
        // We'll use a 3x3 kernel for sharpening
        let kernel_center = 1.0 + intensity * 4.0;
        let kernel_edge = -intensity;

        for y in 0..height {
            for x in 0..width {
                let mut r = 0.0f32;
                let mut g = 0.0f32;
                let mut b = 0.0f32;
                let mut a = 0u32;

                // Apply 3x3 kernel
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = (x as i32 + dx).max(0).min(width as i32 - 1) as u32;
                        let ny = (y as i32 + dy).max(0).min(height as i32 - 1) as u32;

                        let pixel = rgba8.get_pixel(nx, ny);
                        let weight = if dx == 0 && dy == 0 {
                            kernel_center
                        } else {
                            kernel_edge
                        };

                        r += pixel[0] as f32 * weight;
                        g += pixel[1] as f32 * weight;
                        b += pixel[2] as f32 * weight;
                        a = pixel[3] as u32;
                    }
                }

                let new_r = (r.clamp(0.0, 255.0)) as u8;
                let new_g = (g.clamp(0.0, 255.0)) as u8;
                let new_b = (b.clamp(0.0, 255.0)) as u8;

                sharpened.put_pixel(x, y, Rgba([new_r, new_g, new_b, a as u8]));
            }
        }

        DynamicImage::ImageRgba8(sharpened)
    }

    #[cfg(not(feature = "image"))]
    pub fn apply_sharpen(img: DynamicImage, _intensity: f32) -> DynamicImage {
        img
    }

    /// Convert image to grayscale
    pub fn apply_grayscale(img: DynamicImage) -> DynamicImage {
        img.grayscale()
    }

    /// Apply sepia tone effect
    pub fn apply_sepia(img: DynamicImage) -> DynamicImage {
        let (width, height) = img.dimensions();
        let rgba8 = img.to_rgba8();
        let mut sepia_img = RgbaImage::new(width, height);

        for (x, y, pixel) in rgba8.enumerate_pixels() {
            let Rgba([r, g, b, a]) = *pixel;

            // Sepia formula
            let tr = (0.393 * r as f32 + 0.769 * g as f32 + 0.189 * b as f32).min(255.0) as u8;
            let tg = (0.349 * r as f32 + 0.686 * g as f32 + 0.168 * b as f32).min(255.0) as u8;
            let tb = (0.272 * r as f32 + 0.534 * g as f32 + 0.131 * b as f32).min(255.0) as u8;

            sepia_img.put_pixel(x, y, Rgba([tr, tg, tb, a]));
        }

        DynamicImage::ImageRgba8(sepia_img)
    }

    /// Adjust brightness (-100 to 100, where 0 is no change)
    pub fn adjust_brightness(img: DynamicImage, adjustment: i32) -> DynamicImage {
        let (width, height) = img.dimensions();
        let rgba8 = img.to_rgba8();
        let mut adjusted = RgbaImage::new(width, height);

        // Convert -100..100 to multiplier
        let multiplier = 1.0 + (adjustment as f32 / 100.0);

        for (x, y, pixel) in rgba8.enumerate_pixels() {
            let Rgba([r, g, b, a]) = *pixel;

            let new_r = ((r as f32 * multiplier).clamp(0.0, 255.0)) as u8;
            let new_g = ((g as f32 * multiplier).clamp(0.0, 255.0)) as u8;
            let new_b = ((b as f32 * multiplier).clamp(0.0, 255.0)) as u8;

            adjusted.put_pixel(x, y, Rgba([new_r, new_g, new_b, a]));
        }

        DynamicImage::ImageRgba8(adjusted)
    }

    /// Adjust contrast (-100 to 100, where 0 is no change)
    pub fn adjust_contrast(img: DynamicImage, adjustment: f32) -> DynamicImage {
        let (width, height) = img.dimensions();
        let rgba8 = img.to_rgba8();
        let mut adjusted = RgbaImage::new(width, height);

        // Convert -100..100 to factor (0.0 to 2.0, where 1.0 is no change)
        let factor = 1.0 + (adjustment / 100.0);
        let intercept = 128.0 * (1.0 - factor);

        for (x, y, pixel) in rgba8.enumerate_pixels() {
            let Rgba([r, g, b, a]) = *pixel;

            let new_r = ((r as f32 * factor + intercept).clamp(0.0, 255.0)) as u8;
            let new_g = ((g as f32 * factor + intercept).clamp(0.0, 255.0)) as u8;
            let new_b = ((b as f32 * factor + intercept).clamp(0.0, 255.0)) as u8;

            adjusted.put_pixel(x, y, Rgba([new_r, new_g, new_b, a]));
        }

        DynamicImage::ImageRgba8(adjusted)
    }

    /// Adjust saturation (-100 to 100, where 0 is no change)
    pub fn adjust_saturation(img: DynamicImage, adjustment: f32) -> DynamicImage {
        let (width, height) = img.dimensions();
        let rgba8 = img.to_rgba8();
        let mut adjusted = RgbaImage::new(width, height);

        // Convert -100..100 to factor (0.0 to 2.0, where 1.0 is no change)
        let factor = 1.0 + (adjustment / 100.0);

        for (x, y, pixel) in rgba8.enumerate_pixels() {
            let Rgba([r, g, b, a]) = *pixel;

            // Convert RGB to grayscale for desaturation
            let gray = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;

            // Interpolate between original and grayscale based on factor
            let new_r =
                ((r as f32 + (r as f32 - gray as f32) * (factor - 1.0)).clamp(0.0, 255.0)) as u8;
            let new_g =
                ((g as f32 + (g as f32 - gray as f32) * (factor - 1.0)).clamp(0.0, 255.0)) as u8;
            let new_b =
                ((b as f32 + (b as f32 - gray as f32) * (factor - 1.0)).clamp(0.0, 255.0)) as u8;

            adjusted.put_pixel(x, y, Rgba([new_r, new_g, new_b, a]));
        }

        DynamicImage::ImageRgba8(adjusted)
    }

    /// Invert colors
    pub fn apply_invert(img: DynamicImage) -> DynamicImage {
        let (width, height) = img.dimensions();
        let rgba8 = img.to_rgba8();
        let mut inverted = RgbaImage::new(width, height);

        for (x, y, pixel) in rgba8.enumerate_pixels() {
            let Rgba([r, g, b, a]) = *pixel;
            inverted.put_pixel(x, y, Rgba([255 - r, 255 - g, 255 - b, a]));
        }

        DynamicImage::ImageRgba8(inverted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_grayscale() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255])));
        let gray = ImageFilters::apply_grayscale(img);
        let (width, height) = gray.dimensions();
        assert_eq!(width, 10);
        assert_eq!(height, 10);
    }

    #[test]
    fn test_invert() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([255, 100, 50, 255])));
        let inverted = ImageFilters::apply_invert(img);
        let (width, height) = inverted.dimensions();
        assert_eq!(width, 10);
        assert_eq!(height, 10);
        // White should become black
        let inverted_rgba = inverted.to_rgba8();
        let pixel = inverted_rgba.get_pixel(0, 0);
        assert_eq!(pixel[0], 0); // R inverted from 255
    }

    #[test]
    fn test_brightness() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([100, 100, 100, 255])));
        let brighter = ImageFilters::adjust_brightness(img, 50);
        let brighter_rgba = brighter.to_rgba8();
        let pixel = brighter_rgba.get_pixel(0, 0);
        assert!(pixel[0] > 100); // Should be brighter
    }

    #[test]
    fn test_contrast() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([100, 100, 100, 255])));
        let more_contrast = ImageFilters::adjust_contrast(img, 50.0);
        let contrast_rgba = more_contrast.to_rgba8();
        let pixel = contrast_rgba.get_pixel(0, 0);
        // With increased contrast, values move away from 128
        assert!(pixel[0] != 100);
    }

    #[test]
    fn test_sepia() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([255, 255, 255, 255])));
        let sepia = ImageFilters::apply_sepia(img);
        let sepia_rgba = sepia.to_rgba8();
        let pixel = sepia_rgba.get_pixel(0, 0);
        // Sepia should have different R, G, B values (warm tone)
        assert!(pixel[0] != pixel[1] || pixel[1] != pixel[2]);
    }

    #[test]
    fn test_saturation() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([100, 150, 200, 255])));
        let saturated = ImageFilters::adjust_saturation(img, 50.0);
        let saturated_rgba = saturated.to_rgba8();
        let pixel = saturated_rgba.get_pixel(0, 0);
        // Increased saturation should make colors more vibrant
        assert!(pixel[0] != 100 || pixel[1] != 150 || pixel[2] != 200);
    }

    #[test]
    fn test_saturation_desaturate() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([100, 150, 200, 255])));
        let desaturated = ImageFilters::adjust_saturation(img, -100.0); // Full desaturation
        let desaturated_rgba = desaturated.to_rgba8();
        let pixel = desaturated_rgba.get_pixel(0, 0);
        // Desaturated should have equal R, G, B (grayscale)
        // Note: Due to rounding, they might not be exactly equal, but close
        let avg = (pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3;
        assert!((pixel[0] as i32 - avg as i32).abs() < 5);
    }

    #[test]
    fn test_filter_config_default() {
        let config = FilterConfig::default();
        assert_eq!(config.blur, None);
        assert_eq!(config.sharpen, None);
        assert!(!config.grayscale);
        assert!(!config.sepia);
        assert_eq!(config.brightness, None);
        assert_eq!(config.contrast, None);
        assert_eq!(config.saturation, None);
        assert!(!config.invert);
    }

    #[test]
    fn test_apply_multiple_filters() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([100, 100, 100, 255])));
        let config = FilterConfig {
            blur: Some(2.0),
            sharpen: Some(0.5),
            grayscale: true,
            sepia: false,
            brightness: Some(20),
            contrast: Some(10.0),
            saturation: Some(-50.0),
            invert: false,
        };

        let filtered = ImageFilters::apply(img, &config);
        let (width, height) = filtered.dimensions();
        assert_eq!(width, 10);
        assert_eq!(height, 10);
    }

    #[test]
    fn test_brightness_negative() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([100, 100, 100, 255])));
        let darker = ImageFilters::adjust_brightness(img, -50);
        let darker_rgba = darker.to_rgba8();
        let pixel = darker_rgba.get_pixel(0, 0);
        assert!(pixel[0] < 100); // Should be darker
    }

    #[test]
    fn test_contrast_negative() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([100, 100, 100, 255])));
        let less_contrast = ImageFilters::adjust_contrast(img, -50.0);
        let contrast_rgba = less_contrast.to_rgba8();
        let pixel = contrast_rgba.get_pixel(0, 0);
        // With decreased contrast, values move toward 128
        assert!((pixel[0] as i32 - 128).abs() < (100i32 - 128).abs());
    }

    #[test]
    fn test_invert_roundtrip() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([100, 150, 200, 255])));
        let inverted = ImageFilters::apply_invert(img.clone());
        let inverted_twice = ImageFilters::apply_invert(inverted);
        let original_rgba = img.to_rgba8();
        let twice_inverted_rgba = inverted_twice.to_rgba8();
        let original_pixel = original_rgba.get_pixel(0, 0);
        let twice_inverted_pixel = twice_inverted_rgba.get_pixel(0, 0);
        // Inverting twice should return to original (approximately, due to rounding)
        assert!((original_pixel[0] as i32 - twice_inverted_pixel[0] as i32).abs() <= 1);
    }

    #[test]
    fn test_blur() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255])));
        let blurred = ImageFilters::apply_blur(img, 2.0);
        let (width, height) = blurred.dimensions();
        assert_eq!(width, 10);
        assert_eq!(height, 10);
    }

    #[test]
    fn test_sharpen() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([128, 128, 128, 255])));
        let sharpened = ImageFilters::apply_sharpen(img, 0.5);
        let (width, height) = sharpened.dimensions();
        assert_eq!(width, 10);
        assert_eq!(height, 10);
    }
}
