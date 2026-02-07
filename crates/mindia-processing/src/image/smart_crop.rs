use image::{imageops, DynamicImage};

/// Smart crop configuration
#[derive(Debug, Clone)]
pub struct SmartCropConfig {
    pub width: u32,
    pub height: u32,
}

pub struct SmartCrop;

impl SmartCrop {
    /// Calculate saliency map for smart cropping
    fn calculate_saliency_map(img: &DynamicImage) -> Vec<Vec<f32>> {
        use image::GenericImageView;
        let (width, height) = img.dimensions();
        let gray = img.to_luma8();

        // Downscale for performance (work on smaller version)
        let scale = 4;
        let small_width = (width / scale).max(1);
        let small_height = (height / scale).max(1);
        let small_gray = imageops::resize(
            &gray,
            small_width,
            small_height,
            imageops::FilterType::Triangle,
        );

        let mut saliency = vec![vec![0.0f32; small_height as usize]; small_width as usize];

        // Calculate edge density using simple gradient
        for y in 1..small_height.saturating_sub(1) {
            for x in 1..small_width.saturating_sub(1) {
                let right = small_gray.get_pixel(x + 1, y)[0] as i32;
                let left = small_gray.get_pixel(x.saturating_sub(1), y)[0] as i32;
                let bottom = small_gray.get_pixel(x, y + 1)[0] as i32;
                let top = small_gray.get_pixel(x, y.saturating_sub(1))[0] as i32;

                let gx = (right - left).abs();
                let gy = (bottom - top).abs();
                let edge_strength = ((gx * gx + gy * gy) as f32).sqrt();

                // Calculate local variance (entropy approximation)
                let mut sum = 0i32;
                let mut sum_sq = 0i32;
                let mut count = 0;
                for dy in -1i32..=1i32 {
                    for dx in -1i32..=1i32 {
                        let px_x = (x as i32 + dx).max(0).min(small_width as i32 - 1) as u32;
                        let px_y = (y as i32 + dy).max(0).min(small_height as i32 - 1) as u32;
                        let px = small_gray.get_pixel(px_x, px_y)[0] as i32;
                        sum += px;
                        sum_sq += px * px;
                        count += 1;
                    }
                }
                let mean = sum as f32 / count as f32;
                let variance = (sum_sq as f32 / count as f32) - (mean * mean);

                // Combine edge strength and variance
                saliency[x as usize][y as usize] = edge_strength * 0.6 + variance * 0.4;
            }
        }

        // Upscale saliency map back to original size
        let mut full_saliency = vec![vec![0.0f32; height as usize]; width as usize];
        for y in 0..height {
            for x in 0..width {
                let sx = ((x / scale).min(small_width.saturating_sub(1))) as usize;
                let sy = ((y / scale).min(small_height.saturating_sub(1))) as usize;
                full_saliency[x as usize][y as usize] = saliency[sx][sy];
            }
        }

        full_saliency
    }

    /// Smart crop image to target dimensions using saliency map
    pub fn crop(
        img: DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> Result<DynamicImage, anyhow::Error> {
        use image::GenericImageView;
        let (orig_width, orig_height) = img.dimensions();

        if target_width > orig_width || target_height > orig_height {
            return Err(anyhow::anyhow!(
                "Crop dimensions ({}, {}) exceed image dimensions ({}, {})",
                target_width,
                target_height,
                orig_width,
                orig_height
            ));
        }

        if target_width == orig_width && target_height == orig_height {
            return Ok(img);
        }

        // Calculate saliency map
        let saliency = Self::calculate_saliency_map(&img);

        // Find best crop region
        let mut best_score = 0.0f32;
        let mut best_x = 0u32;
        let mut best_y = 0u32;

        for y in 0..=(orig_height - target_height) {
            for x in 0..=(orig_width - target_width) {
                let mut score = 0.0f32;
                for dy in 0..target_height {
                    for dx in 0..target_width {
                        score += saliency[(x + dx) as usize][(y + dy) as usize];
                    }
                }
                if score > best_score {
                    best_score = score;
                    best_x = x;
                    best_y = y;
                }
            }
        }

        // Crop the image
        let cropped = imageops::crop_imm(&img, best_x, best_y, target_width, target_height);
        Ok(DynamicImage::ImageRgba8(cropped.to_image()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

    #[test]
    fn test_smart_crop() {
        // Create a test image
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255])));

        // Crop to smaller size
        let cropped = SmartCrop::crop(img.clone(), 50, 50).unwrap();
        assert_eq!(cropped.dimensions(), (50, 50));

        // Crop to same size (should return original)
        let cropped = SmartCrop::crop(img.clone(), 100, 100).unwrap();
        assert_eq!(cropped.dimensions(), (100, 100));

        // Crop larger than image should fail
        assert!(SmartCrop::crop(img.clone(), 200, 200).is_err());
    }

    #[test]
    fn test_smart_crop_square_to_rectangle() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255])));
        let cropped = SmartCrop::crop(img, 50, 80).unwrap();
        assert_eq!(cropped.dimensions(), (50, 80));
    }

    #[test]
    fn test_smart_crop_rectangle_to_square() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(200, 100, Rgba([255, 255, 255, 255])));
        let cropped = SmartCrop::crop(img, 100, 100).unwrap();
        assert_eq!(cropped.dimensions(), (100, 100));
    }

    #[test]
    fn test_smart_crop_small_crop() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255])));
        let cropped = SmartCrop::crop(img, 10, 10).unwrap();
        assert_eq!(cropped.dimensions(), (10, 10));
    }

    #[test]
    fn test_smart_crop_exact_dimensions() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255])));
        // Cropping to exact same dimensions should return original
        let cropped = SmartCrop::crop(img.clone(), 100, 100).unwrap();
        assert_eq!(cropped.dimensions(), (100, 100));
    }

    #[test]
    fn test_smart_crop_error_cases() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255])));

        // Width too large
        assert!(SmartCrop::crop(img.clone(), 200, 50).is_err());

        // Height too large
        assert!(SmartCrop::crop(img.clone(), 50, 200).is_err());

        // Both too large
        assert!(SmartCrop::crop(img.clone(), 200, 200).is_err());
    }

    #[test]
    fn test_smart_crop_with_pattern() {
        // Create an image with a pattern (checkerboard) to test saliency
        let mut img = RgbaImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                let color = if (x + y) % 20 < 10 {
                    Rgba([0, 0, 0, 255]) // Black
                } else {
                    Rgba([255, 255, 255, 255]) // White
                };
                img.put_pixel(x, y, color);
            }
        }
        let dynamic_img = DynamicImage::ImageRgba8(img);

        // Should successfully crop
        let cropped = SmartCrop::crop(dynamic_img, 50, 50).unwrap();
        assert_eq!(cropped.dimensions(), (50, 50));
    }
}
