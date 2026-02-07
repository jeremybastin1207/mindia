use super::processor::ImageProcessor;
use image::{imageops, DynamicImage};

/// Image orientation operations (rotation and flipping)
pub struct ImageOrientation;

impl ImageOrientation {
    /// Apply EXIF orientation correction to an image
    pub fn apply_exif_orientation(mut img: DynamicImage, data: &[u8]) -> DynamicImage {
        let orientation = ImageProcessor::read_exif_orientation(data);
        let (rotate, flip_h, flip_v) = ImageProcessor::get_orientation_transforms(orientation);

        tracing::debug!(
            orientation = orientation,
            rotate = ?rotate,
            flip_horizontal = flip_h,
            flip_vertical = flip_v,
            "Applying EXIF orientation"
        );

        // Apply rotation first
        if let Some(angle) = rotate {
            img = Self::rotate_by_angle(img, angle);
        }

        // Then apply flips
        if flip_h {
            img = Self::apply_flip_horizontal(img);
        }
        if flip_v {
            img = Self::apply_flip_vertical(img);
        }

        img
    }

    /// Rotate image by specified angle (90, 180, or 270 degrees clockwise)
    pub fn rotate_by_angle(img: DynamicImage, angle: u16) -> DynamicImage {
        match angle {
            90 => DynamicImage::ImageRgba8(imageops::rotate90(&img.to_rgba8())),
            180 => DynamicImage::ImageRgba8(imageops::rotate180(&img.to_rgba8())),
            270 => DynamicImage::ImageRgba8(imageops::rotate270(&img.to_rgba8())),
            _ => img, // Should never happen due to validation
        }
    }

    /// Apply horizontal flip (mirror)
    pub fn apply_flip_horizontal(img: DynamicImage) -> DynamicImage {
        DynamicImage::ImageRgba8(imageops::flip_horizontal(&img.to_rgba8()))
    }

    /// Apply vertical flip
    pub fn apply_flip_vertical(img: DynamicImage) -> DynamicImage {
        DynamicImage::ImageRgba8(imageops::flip_vertical(&img.to_rgba8()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

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
    fn test_exif_orientation() {
        // Create a test image
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([255, 0, 0, 255])));
        // Test with empty data (no EXIF, should return normal orientation)
        let data = b"";
        let oriented = ImageOrientation::apply_exif_orientation(img.clone(), data);
        assert_eq!(oriented.dimensions(), img.dimensions());
    }
}
