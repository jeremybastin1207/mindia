use mindia_core::transform_url::{ImageTransformUrlParser, ParsedTransformUrl};
use mindia_services::{
    FilterConfig, OutputFormat, QualityPreset, ResizeDimensions, SmartCropConfig, StretchMode,
    WatermarkConfig, WatermarkPosition, WatermarkSize,
};
use uuid::Uuid;

/// Parsed transformation operations using service types
#[derive(Debug)]
pub struct TransformOperations {
    /// Named transformation preset (if specified)
    pub preset_name: Option<String>,
    pub resize_dims: Option<ResizeDimensions>,
    pub stretch_mode: StretchMode,
    pub format: Option<OutputFormat>,
    pub quality: QualityPreset,
    pub rotate_angle: Option<u16>, // 90, 180, or 270
    pub flip_vertical: bool,       // vertical flip
    pub flip_horizontal: bool,     // horizontal flip (mirror)
    pub autorotate: bool,          // default true
    pub watermark: Option<WatermarkConfig>,
    pub smart_crop: Option<SmartCropConfig>,
    pub filter_config: Option<FilterConfig>,
}

/// Convert ParsedTransformUrl to TransformOperations
impl From<ParsedTransformUrl> for TransformOperations {
    fn from(parsed: ParsedTransformUrl) -> Self {
        // Convert resize dimensions
        let resize_dims = if parsed.resize_width.is_some() || parsed.resize_height.is_some() {
            Some(ResizeDimensions {
                width: parsed.resize_width,
                height: parsed.resize_height,
            })
        } else {
            None
        };

        // Convert stretch mode
        let stretch_mode = if let Some(ref mode_str) = parsed.stretch_mode {
            match mode_str.as_str() {
                "on" => StretchMode::On,
                "off" => StretchMode::Off,
                "fill" => StretchMode::Fill,
                _ => StretchMode::default(),
            }
        } else {
            StretchMode::default()
        };

        // Convert format
        let format = parsed
            .format
            .as_ref()
            .and_then(|f| OutputFormat::parse(f).ok());

        // Convert quality
        let quality = parsed
            .quality
            .as_ref()
            .and_then(|q| QualityPreset::parse(q).ok())
            .unwrap_or_default();

        // Convert watermark
        let watermark = if let Some(ref watermark_id_str) = parsed.watermark_id {
            if let Ok(watermark_id) = watermark_id_str.parse::<Uuid>() {
                let position = parsed
                    .watermark_position
                    .as_ref()
                    .map(|pos| match pos.as_str() {
                        "tl" => WatermarkPosition::TopLeft,
                        "tr" => WatermarkPosition::TopRight,
                        "bl" => WatermarkPosition::BottomLeft,
                        "br" => WatermarkPosition::BottomRight,
                        "center" => WatermarkPosition::Center,
                        custom => {
                            let coords: Vec<&str> = custom.split(',').collect();
                            if coords.len() == 2 {
                                if let (Ok(x), Ok(y)) =
                                    (coords[0].parse::<u32>(), coords[1].parse::<u32>())
                                {
                                    WatermarkPosition::Custom { x, y }
                                } else {
                                    WatermarkPosition::BottomRight
                                }
                            } else {
                                WatermarkPosition::BottomRight
                            }
                        }
                    })
                    .unwrap_or(WatermarkPosition::BottomRight);

                let size = parsed
                    .watermark_size
                    .as_ref()
                    .map(|size_str| {
                        if size_str.ends_with('%') {
                            if let Ok(percent) = size_str.trim_end_matches('%').parse::<f32>() {
                                WatermarkSize::Relative { percent }
                            } else {
                                WatermarkSize::Relative { percent: 10.0 }
                            }
                        } else if size_str.contains('x') {
                            if let Ok(dims) = ResizeDimensions::parse(size_str) {
                                if let (Some(width), Some(height)) = (dims.width, dims.height) {
                                    WatermarkSize::Absolute { width, height }
                                } else {
                                    WatermarkSize::Relative { percent: 10.0 }
                                }
                            } else {
                                WatermarkSize::Relative { percent: 10.0 }
                            }
                        } else {
                            WatermarkSize::Relative { percent: 10.0 }
                        }
                    })
                    .unwrap_or(WatermarkSize::Relative { percent: 10.0 });

                let opacity = parsed.watermark_opacity.unwrap_or(0.5);

                Some(WatermarkConfig {
                    watermark_id,
                    position,
                    size,
                    opacity,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Convert smart crop
        let smart_crop = if let (Some(width), Some(height)) =
            (parsed.smart_crop_width, parsed.smart_crop_height)
        {
            Some(SmartCropConfig { width, height })
        } else {
            None
        };

        // Convert filter config
        let has_filters = parsed.blur_radius.is_some()
            || parsed.sharpen_intensity.is_some()
            || parsed.grayscale
            || parsed.sepia
            || parsed.brightness.is_some()
            || parsed.contrast.is_some()
            || parsed.saturation.is_some()
            || parsed.invert;

        let filter_config = if has_filters {
            Some(FilterConfig {
                blur: parsed.blur_radius,
                sharpen: parsed.sharpen_intensity,
                grayscale: parsed.grayscale,
                sepia: parsed.sepia,
                brightness: parsed.brightness,
                contrast: parsed.contrast,
                saturation: parsed.saturation,
                invert: parsed.invert,
            })
        } else {
            None
        };

        Self {
            preset_name: parsed.preset_name,
            resize_dims,
            stretch_mode,
            format,
            quality,
            rotate_angle: parsed.rotate_angle,
            flip_vertical: parsed.flip_vertical,
            flip_horizontal: parsed.flip_horizontal,
            autorotate: parsed.autorotate.unwrap_or(true),
            watermark,
            smart_crop,
            filter_config,
        }
    }
}

pub fn parse_operations(ops: &str) -> Result<TransformOperations, String> {
    // Use the existing parser from mindia-core
    let parsed = ImageTransformUrlParser::parse_operations(ops, String::new())?;
    Ok(TransformOperations::from(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mindia_services::{
        OutputFormat, QualityPreset, StretchMode, WatermarkPosition, WatermarkSize,
    };
    use uuid::Uuid;

    #[test]
    fn test_parse_operations() {
        let ops = parse_operations("/-/resize/320x240/").unwrap();
        assert!(ops.resize_dims.is_some());
        let dims = ops.resize_dims.unwrap();
        assert_eq!(dims.width, Some(320));
        assert_eq!(dims.height, Some(240));
        assert_eq!(ops.stretch_mode, StretchMode::On);

        let ops = parse_operations("/-/stretch/off/-/resize/320x/").unwrap();
        assert!(ops.resize_dims.is_some());
        let dims = ops.resize_dims.unwrap();
        assert_eq!(dims.width, Some(320));
        assert_eq!(dims.height, None);
        assert_eq!(ops.stretch_mode, StretchMode::Off);

        let ops = parse_operations("/-/stretch/fill/-/resize/x240/").unwrap();
        assert!(ops.resize_dims.is_some());
        let dims = ops.resize_dims.unwrap();
        assert_eq!(dims.width, None);
        assert_eq!(dims.height, Some(240));
        assert_eq!(ops.stretch_mode, StretchMode::Fill);

        let ops = parse_operations("/-/resize/800x600/").unwrap();
        assert!(ops.resize_dims.is_some());
        let dims = ops.resize_dims.unwrap();
        assert_eq!(dims.width, Some(800));
        assert_eq!(dims.height, Some(600));
        assert_eq!(ops.stretch_mode, StretchMode::On);
    }

    #[test]
    fn test_parse_format_operations() {
        let ops = parse_operations("/-/format/webp/-/quality/better/").unwrap();
        assert!(ops.format.is_some());
        assert_eq!(ops.format.unwrap(), OutputFormat::WebP);
        assert_eq!(ops.quality, QualityPreset::Better);

        let ops = parse_operations("/-/resize/800x600/-/format/jpeg/-/quality/lighter/").unwrap();
        assert!(ops.resize_dims.is_some());
        assert_eq!(ops.format.unwrap(), OutputFormat::Jpeg);
        assert_eq!(ops.quality, QualityPreset::Lighter);

        let ops = parse_operations("/-/format/auto/").unwrap();
        assert_eq!(ops.format.unwrap(), OutputFormat::Auto);
    }

    #[test]
    fn test_parse_rotation_operations() {
        // Test rotate 90
        let ops = parse_operations("/-/rotate/90/").unwrap();
        assert_eq!(ops.rotate_angle, Some(90));
        assert!(!ops.flip_horizontal);
        assert!(!ops.flip_vertical);
        assert!(ops.autorotate); // Default is true

        // Test rotate 180
        let ops = parse_operations("/-/rotate/180/").unwrap();
        assert_eq!(ops.rotate_angle, Some(180));

        // Test rotate 270
        let ops = parse_operations("/-/rotate/270/").unwrap();
        assert_eq!(ops.rotate_angle, Some(270));

        // Test invalid angle
        assert!(parse_operations("/-/rotate/45/").is_err());
        assert!(parse_operations("/-/rotate/360/").is_err());

        // Test missing angle
        assert!(parse_operations("/-/rotate/").is_err());
    }

    #[test]
    fn test_parse_flip_mirror_operations() {
        // Test flip (vertical)
        let ops = parse_operations("/-/flip/").unwrap();
        assert!(ops.flip_vertical);
        assert!(!ops.flip_horizontal);

        // Test mirror (horizontal)
        let ops = parse_operations("/-/mirror/").unwrap();
        assert!(ops.flip_horizontal);
        assert!(!ops.flip_vertical);

        // Test both flip and mirror
        let ops = parse_operations("/-/flip/-/mirror/").unwrap();
        assert!(ops.flip_vertical);
        assert!(ops.flip_horizontal);
    }

    #[test]
    fn test_parse_autorotate_operations() {
        // Test autorotate yes (explicit)
        let ops = parse_operations("/-/autorotate/yes/").unwrap();
        assert!(ops.autorotate);

        // Test autorotate no
        let ops = parse_operations("/-/autorotate/no/").unwrap();
        assert!(!ops.autorotate);

        // Test invalid autorotate value
        assert!(parse_operations("/-/autorotate/maybe/").is_err());

        // Test missing autorotate value
        assert!(parse_operations("/-/autorotate/").is_err());

        // Test default is true
        let ops = parse_operations("/-/resize/100x/").unwrap();
        assert!(ops.autorotate);
    }

    #[test]
    fn test_parse_combined_rotation_operations() {
        // Test rotate + resize
        let ops = parse_operations("/-/rotate/90/-/resize/800x600/").unwrap();
        assert_eq!(ops.rotate_angle, Some(90));
        assert!(ops.resize_dims.is_some());

        // Test autorotate no + manual rotate
        let ops = parse_operations("/-/autorotate/no/-/rotate/180/").unwrap();
        assert!(!ops.autorotate);
        assert_eq!(ops.rotate_angle, Some(180));

        // Test mirror + rotate + resize
        let ops = parse_operations("/-/mirror/-/rotate/90/-/resize/400x/").unwrap();
        assert!(ops.flip_horizontal);
        assert_eq!(ops.rotate_angle, Some(90));
        assert!(ops.resize_dims.is_some());

        // Test all rotation features combined
        let ops =
            parse_operations("/-/autorotate/no/-/rotate/270/-/flip/-/mirror/-/resize/640x480/")
                .unwrap();
        assert!(!ops.autorotate);
        assert_eq!(ops.rotate_angle, Some(270));
        assert!(ops.flip_vertical);
        assert!(ops.flip_horizontal);
        assert!(ops.resize_dims.is_some());
        let dims = ops.resize_dims.unwrap();
        assert_eq!(dims.width, Some(640));
        assert_eq!(dims.height, Some(480));
    }

    #[test]
    fn test_parse_watermark_operations() {
        let watermark_id = Uuid::new_v4();
        let ops = parse_operations(&format!(
            "/-/watermark/{}/position/br/size/10%/opacity/0.7",
            watermark_id
        ))
        .unwrap();
        assert!(ops.watermark.is_some());
        let wm = ops.watermark.unwrap();
        assert_eq!(wm.watermark_id, watermark_id);
        assert!(matches!(wm.position, WatermarkPosition::BottomRight));
        assert!(
            matches!(wm.size, WatermarkSize::Relative { percent: p } if (p - 10.0).abs() < 0.01)
        );
        assert!((wm.opacity - 0.7).abs() < 0.01);

        let ops = parse_operations(&format!(
            "/-/watermark/{}/position/tl/size/200x100/opacity/0.5",
            watermark_id
        ))
        .unwrap();
        let wm = ops.watermark.unwrap();
        assert!(matches!(wm.position, WatermarkPosition::TopLeft));
        assert!(matches!(
            wm.size,
            WatermarkSize::Absolute {
                width: 200,
                height: 100
            }
        ));

        let ops =
            parse_operations(&format!("/-/watermark/{}/position/center", watermark_id)).unwrap();
        let wm = ops.watermark.unwrap();
        assert!(matches!(wm.position, WatermarkPosition::Center));
        assert!(
            matches!(wm.size, WatermarkSize::Relative { percent: p } if (p - 10.0).abs() < 0.01)
        ); // default
        assert!((wm.opacity - 0.5).abs() < 0.01); // default
    }

    #[test]
    fn test_parse_smart_crop_operations() {
        let ops = parse_operations("/-/crop/smart/400x300/").unwrap();
        assert!(ops.smart_crop.is_some());
        let crop = ops.smart_crop.unwrap();
        assert_eq!(crop.width, 400);
        assert_eq!(crop.height, 300);

        let ops = parse_operations("/-/crop/smart/500x500/").unwrap();
        let crop = ops.smart_crop.unwrap();
        assert_eq!(crop.width, 500);
        assert_eq!(crop.height, 500);

        // Invalid crop mode
        assert!(parse_operations("/-/crop/manual/400x300/").is_err());

        // Missing dimensions
        assert!(parse_operations("/-/crop/smart/").is_err());
    }

    #[test]
    fn test_parse_transform_url_format() {
        // Test transformation URL format with /-/ separators
        let ops = parse_operations("/-/resize/500x300/-/format/webp/-/quality/better").unwrap();
        assert!(ops.resize_dims.is_some());
        let dims = ops.resize_dims.unwrap();
        assert_eq!(dims.width, Some(500));
        assert_eq!(dims.height, Some(300));
        assert_eq!(ops.format.unwrap(), OutputFormat::WebP);
        assert_eq!(ops.quality, QualityPreset::Better);

        // Test with leading and trailing separators
        let ops = parse_operations("/-/resize/800x600/-/format/webp/").unwrap();
        assert!(ops.resize_dims.is_some());
        assert_eq!(ops.format.unwrap(), OutputFormat::WebP);

        // Test single operation
        let ops = parse_operations("/-/resize/500x/").unwrap();
        assert!(ops.resize_dims.is_some());
        let dims = ops.resize_dims.unwrap();
        assert_eq!(dims.width, Some(500));
        assert_eq!(dims.height, None);

        // Test complex operations with /-/
        let ops =
            parse_operations("/-/crop/smart/400x300/-/resize/800x600/-/format/webp/").unwrap();
        assert!(ops.smart_crop.is_some());
        assert!(ops.resize_dims.is_some());
        assert_eq!(ops.format.unwrap(), OutputFormat::WebP);
    }

    #[test]
    fn test_parse_combined_watermark_crop() {
        let watermark_id = Uuid::new_v4();
        let ops = parse_operations(&format!(
            "/-/crop/smart/400x300/-/resize/800x600/-/watermark/{}/position/br/size/10%",
            watermark_id
        ))
        .unwrap();
        assert!(ops.smart_crop.is_some());
        assert!(ops.watermark.is_some());
        assert!(ops.resize_dims.is_some());
    }
}
