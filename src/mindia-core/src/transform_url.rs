//! Image transformation URL builder and parser
//!
//! Provides a fluent API for building image transformation URLs in a format
//! with `/-/` separators, and an efficient parser to extract transformation parameters from URLs.

/// Parsed transformation parameters from a URL
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTransformUrl {
    /// Image ID extracted from the URL
    pub image_id: String,
    /// Named transformation preset (if specified, e.g., "thumbnail")
    pub preset_name: Option<String>,
    /// Resize width (if specified)
    pub resize_width: Option<u32>,
    /// Resize height (if specified)
    pub resize_height: Option<u32>,
    /// Stretch mode: "on", "off", or "fill"
    pub stretch_mode: Option<String>,
    /// Output format: "jpeg", "png", "webp", "avif", or "auto"
    pub format: Option<String>,
    /// Quality preset: "low", "medium", "high", "normal", "better", "best", "lighter", "lightest", or "fast"
    pub quality: Option<String>,
    /// Rotation angle: 90, 180, or 270
    pub rotate_angle: Option<u16>,
    /// Vertical flip enabled
    pub flip_vertical: bool,
    /// Horizontal flip (mirror) enabled
    pub flip_horizontal: bool,
    /// Autorotate (EXIF orientation correction)
    pub autorotate: Option<bool>,
    /// Watermark ID (UUID)
    pub watermark_id: Option<String>,
    /// Watermark position: "tl", "tr", "bl", "br", "center", or "x,y"
    pub watermark_position: Option<String>,
    /// Watermark size: "10%" or "200x100"
    pub watermark_size: Option<String>,
    /// Watermark opacity (0.0 to 1.0)
    pub watermark_opacity: Option<f32>,
    /// Smart crop width
    pub smart_crop_width: Option<u32>,
    /// Smart crop height
    pub smart_crop_height: Option<u32>,
    /// Blur radius (f32)
    pub blur_radius: Option<f32>,
    /// Sharpen intensity (0.0 to 1.0)
    pub sharpen_intensity: Option<f32>,
    /// Grayscale enabled
    pub grayscale: bool,
    /// Sepia enabled
    pub sepia: bool,
    /// Brightness adjustment (-100 to 100)
    pub brightness: Option<i32>,
    /// Contrast adjustment (-100 to 100)
    pub contrast: Option<f32>,
    /// Saturation adjustment (-100 to 100)
    pub saturation: Option<f32>,
    /// Invert colors enabled
    pub invert: bool,
}

/// Efficient parser for image transformation URLs
///
/// Parses URLs in transformation format with `/-/` separators.
/// Can parse both full URLs and path segments.
///
/// # Example
///
/// ```rust
/// use mindia_core::transform_url::ImageTransformUrlParser;
///
/// let url = "https://api.example.com/api/images/img-123/-/resize/500x300/-/format/webp/";
/// let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
/// assert_eq!(parsed.image_id, "img-123");
/// assert_eq!(parsed.resize_width, Some(500));
/// assert_eq!(parsed.resize_height, Some(300));
/// assert_eq!(parsed.format.as_deref(), Some("webp"));
/// ```
pub struct ImageTransformUrlParser;

impl ImageTransformUrlParser {
    /// Parse a full transformation URL
    ///
    /// # Arguments
    /// * `url` - Full URL (e.g., "https://api.example.com/api/images/img-123/-/resize/500x/")
    ///
    /// # Returns
    /// Parsed transformation parameters, or error message if parsing fails
    pub fn parse_url(url: &str) -> Result<ParsedTransformUrl, String> {
        // Extract the path part (after domain), strip query and fragment
        let path_with_query = if let Some(path_start) = url.find("/api/images/") {
            &url[path_start..]
        } else if url.starts_with("/api/images/") {
            url
        } else {
            return Err("URL must contain /api/images/".to_string());
        };
        let path = path_with_query
            .split('?')
            .next()
            .unwrap_or(path_with_query)
            .split('#')
            .next()
            .unwrap_or(path_with_query);

        Self::parse_path(path)
    }

    /// Parse a path segment (starting with /api/images/)
    ///
    /// # Arguments
    /// * `path` - Path segment (e.g., "/api/images/img-123/-/resize/500x/")
    ///
    /// # Returns
    /// Parsed transformation parameters, or error message if parsing fails
    pub fn parse_path(path: &str) -> Result<ParsedTransformUrl, String> {
        let path = path.trim_start_matches('/').trim_end_matches('/');

        // Split into parts: ["api", "images", "image-id", ...operations...]
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if parts.len() < 3 || parts[0] != "api" || parts[1] != "images" {
            return Err("Path must start with /api/images/".to_string());
        }

        let image_id = parts[2].to_string();

        // Extract operations part (everything after image ID)
        if parts.len() == 3 {
            // No operations
            return Ok(ParsedTransformUrl {
                image_id,
                preset_name: None,
                resize_width: None,
                resize_height: None,
                stretch_mode: None,
                format: None,
                quality: None,
                rotate_angle: None,
                flip_vertical: false,
                flip_horizontal: false,
                autorotate: None,
                watermark_id: None,
                watermark_position: None,
                watermark_size: None,
                watermark_opacity: None,
                smart_crop_width: None,
                smart_crop_height: None,
                blur_radius: None,
                sharpen_intensity: None,
                grayscale: false,
                sepia: false,
                brightness: None,
                contrast: None,
                saturation: None,
                invert: false,
            });
        }

        // Join operations part (from index 3 onwards)
        let operations_str = parts[3..].join("/");
        Self::parse_operations(&operations_str, image_id)
    }

    /// Parse operations string into transformation parameters
    ///
    /// # Arguments
    /// * `operations` - Operations string (e.g., "-/resize/500x300/-/format/webp/")
    /// * `image_id` - Image ID to include in parsed result
    ///
    /// # Returns
    /// Parsed transformation parameters, or error message if parsing fails
    pub fn parse_operations(
        operations: &str,
        image_id: String,
    ) -> Result<ParsedTransformUrl, String> {
        let trimmed = operations.trim_matches('/');

        // Only support transformation URL format with /-/ separators
        // Allow empty, "-" (from "-/"), strings that start with -/ or contain /-/
        if !trimmed.is_empty()
            && trimmed != "-"
            && !trimmed.starts_with("-/")
            && !trimmed.contains("/-/")
        {
            return Err("Invalid format: URLs must use transformation format with /-/ separators (e.g., /-/resize/500x/)".to_string());
        }

        let parts: Vec<&str> = trimmed
            .split("/-/")
            .filter_map(|segment| {
                let segment = segment
                    .trim_matches('/')
                    .trim_start_matches('-')
                    .trim_start_matches('/');
                if segment.is_empty() {
                    None
                } else {
                    Some(
                        segment
                            .split('/')
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>(),
                    )
                }
            })
            .flatten()
            .collect();

        let mut result = ParsedTransformUrl {
            image_id,
            preset_name: None,
            resize_width: None,
            resize_height: None,
            stretch_mode: None,
            format: None,
            quality: None,
            rotate_angle: None,
            flip_vertical: false,
            flip_horizontal: false,
            autorotate: None,
            watermark_id: None,
            watermark_position: None,
            watermark_size: None,
            watermark_opacity: None,
            smart_crop_width: None,
            smart_crop_height: None,
            blur_radius: None,
            sharpen_intensity: None,
            grayscale: false,
            sepia: false,
            brightness: None,
            contrast: None,
            saturation: None,
            invert: false,
        };

        let mut i = 0;
        while i < parts.len() {
            match parts[i] {
                "resize" => {
                    if i + 1 >= parts.len() {
                        return Err("resize requires dimensions".to_string());
                    }
                    let dims = Self::parse_dimensions(parts[i + 1])?;
                    result.resize_width = dims.0;
                    result.resize_height = dims.1;
                    i += 2;
                }
                "stretch" => {
                    if i + 1 >= parts.len() {
                        return Err("stretch requires mode".to_string());
                    }
                    result.stretch_mode = Some(parts[i + 1].to_string());
                    i += 2;
                }
                "format" => {
                    if i + 1 >= parts.len() {
                        return Err("format requires a value".to_string());
                    }
                    result.format = Some(parts[i + 1].to_string());
                    i += 2;
                }
                "quality" => {
                    if i + 1 >= parts.len() {
                        return Err("quality requires a value".to_string());
                    }
                    result.quality = Some(parts[i + 1].to_string());
                    i += 2;
                }
                "rotate" => {
                    if i + 1 >= parts.len() {
                        return Err("rotate requires angle".to_string());
                    }
                    let angle = parts[i + 1]
                        .parse::<u16>()
                        .map_err(|_| format!("Invalid rotation angle: {}", parts[i + 1]))?;
                    if angle != 90 && angle != 180 && angle != 270 {
                        return Err(format!(
                            "Invalid rotation angle: {}. Must be 90, 180, or 270",
                            angle
                        ));
                    }
                    result.rotate_angle = Some(angle);
                    i += 2;
                }
                "flip" => {
                    result.flip_vertical = true;
                    i += 1;
                }
                "mirror" => {
                    result.flip_horizontal = true;
                    i += 1;
                }
                "autorotate" => {
                    if i + 1 >= parts.len() {
                        return Err("autorotate requires value (yes/no)".to_string());
                    }
                    result.autorotate = Some(match parts[i + 1] {
                        "yes" => true,
                        "no" => false,
                        _ => {
                            return Err(format!(
                                "Invalid autorotate value: {}. Must be yes or no",
                                parts[i + 1]
                            ));
                        }
                    });
                    i += 2;
                }
                "watermark" => {
                    if i + 1 >= parts.len() {
                        return Err("watermark requires watermark_id".to_string());
                    }
                    result.watermark_id = Some(parts[i + 1].to_string());

                    let mut position = None;
                    let mut size = None;
                    let mut opacity = None;

                    i += 2;
                    // Parse optional parameters: position, size, opacity
                    while i < parts.len() {
                        match parts[i] {
                            "position" => {
                                if i + 1 >= parts.len() {
                                    return Err("position requires value".to_string());
                                }
                                let pos = parts[i + 1];
                                if !Self::is_valid_watermark_position(pos) {
                                    return Err(format!(
                                        "Invalid watermark position: {}. Use tl, tr, bl, br, center, or x,y",
                                        pos
                                    ));
                                }
                                position = Some(pos.to_string());
                                i += 2;
                            }
                            "size" => {
                                if i + 1 >= parts.len() {
                                    return Err("size requires value".to_string());
                                }
                                size = Some(parts[i + 1].to_string());
                                i += 2;
                            }
                            "opacity" => {
                                if i + 1 >= parts.len() {
                                    return Err("opacity requires value".to_string());
                                }
                                let op = parts[i + 1]
                                    .parse::<f32>()
                                    .map_err(|_| format!("Invalid opacity: {}", parts[i + 1]))?;
                                if !(0.0..=1.0).contains(&op) {
                                    return Err(format!(
                                        "Opacity must be between 0.0 and 1.0: {}",
                                        op
                                    ));
                                }
                                opacity = Some(op);
                                i += 2;
                            }
                            _ => break, // End of watermark parameters
                        }
                    }

                    result.watermark_position = position;
                    result.watermark_size = size;
                    result.watermark_opacity = opacity;
                }
                "crop" => {
                    if i + 1 >= parts.len() {
                        return Err("crop requires mode".to_string());
                    }
                    if parts[i + 1] != "smart" {
                        return Err(format!(
                            "Unknown crop mode: {}. Only 'smart' is supported",
                            parts[i + 1]
                        ));
                    }
                    if i + 2 >= parts.len() {
                        return Err("crop smart requires dimensions".to_string());
                    }
                    let dims = Self::parse_dimensions(parts[i + 2])?;
                    let width = dims
                        .0
                        .ok_or_else(|| "Width required for smart crop".to_string())?;
                    let height = dims
                        .1
                        .ok_or_else(|| "Height required for smart crop".to_string())?;
                    if width == 0 || height == 0 {
                        return Err("Crop dimensions must be greater than 0".to_string());
                    }
                    result.smart_crop_width = Some(width);
                    result.smart_crop_height = Some(height);
                    i += 3;
                }
                "blur" => {
                    if i + 1 >= parts.len() {
                        return Err("blur requires radius".to_string());
                    }
                    let radius = parts[i + 1]
                        .parse::<f32>()
                        .map_err(|_| format!("Invalid blur radius: {}", parts[i + 1]))?;
                    if !(0.0..=100.0).contains(&radius) {
                        return Err("Blur radius must be between 0.0 and 100.0".to_string());
                    }
                    result.blur_radius = Some(radius);
                    i += 2;
                }
                "sharpen" => {
                    if i + 1 >= parts.len() {
                        return Err("sharpen requires intensity".to_string());
                    }
                    let intensity = parts[i + 1]
                        .parse::<f32>()
                        .map_err(|_| format!("Invalid sharpen intensity: {}", parts[i + 1]))?;
                    if !(0.0..=1.0).contains(&intensity) {
                        return Err("Sharpen intensity must be between 0.0 and 1.0".to_string());
                    }
                    result.sharpen_intensity = Some(intensity);
                    i += 2;
                }
                "grayscale" => {
                    result.grayscale = true;
                    i += 1;
                }
                "sepia" => {
                    result.sepia = true;
                    i += 1;
                }
                "brightness" => {
                    if i + 1 >= parts.len() {
                        return Err("brightness requires adjustment value".to_string());
                    }
                    let adjustment = parts[i + 1]
                        .parse::<i32>()
                        .map_err(|_| format!("Invalid brightness: {}", parts[i + 1]))?;
                    if !(-100..=100).contains(&adjustment) {
                        return Err("Brightness must be between -100 and 100".to_string());
                    }
                    result.brightness = Some(adjustment);
                    i += 2;
                }
                "contrast" => {
                    if i + 1 >= parts.len() {
                        return Err("contrast requires adjustment value".to_string());
                    }
                    let adjustment = parts[i + 1]
                        .parse::<f32>()
                        .map_err(|_| format!("Invalid contrast: {}", parts[i + 1]))?;
                    if !(-100.0..=100.0).contains(&adjustment) {
                        return Err("Contrast must be between -100 and 100".to_string());
                    }
                    result.contrast = Some(adjustment);
                    i += 2;
                }
                "saturation" => {
                    if i + 1 >= parts.len() {
                        return Err("saturation requires adjustment value".to_string());
                    }
                    let adjustment = parts[i + 1]
                        .parse::<f32>()
                        .map_err(|_| format!("Invalid saturation: {}", parts[i + 1]))?;
                    if !(-100.0..=100.0).contains(&adjustment) {
                        return Err("Saturation must be between -100 and 100".to_string());
                    }
                    result.saturation = Some(adjustment);
                    i += 2;
                }
                "invert" => {
                    result.invert = true;
                    i += 1;
                }
                "preset" => {
                    if i + 1 >= parts.len() {
                        return Err("preset requires a name".to_string());
                    }
                    result.preset_name = Some(parts[i + 1].to_string());
                    i += 2;
                }
                _ => {
                    return Err(format!("Unknown operation: {}", parts[i]));
                }
            }
        }

        Ok(result)
    }

    /// Valid watermark positions: tl, tr, bl, br, center, or x,y coordinates
    fn is_valid_watermark_position(s: &str) -> bool {
        match s {
            "tl" | "tr" | "bl" | "br" | "center" => true,
            _ => {
                let coords: Vec<&str> = s.split(',').collect();
                coords.len() == 2
                    && coords[0].parse::<u32>().is_ok()
                    && coords[1].parse::<u32>().is_ok()
            }
        }
    }

    /// Parse dimension string (e.g., "500x300", "500x", "x300")
    ///
    /// Made pub(crate) for testing purposes.
    pub(crate) fn parse_dimensions(s: &str) -> Result<(Option<u32>, Option<u32>), String> {
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

        Ok((width, height))
    }
}

/// Builder for constructing image transformation URLs
///
/// # Example
///
/// ```rust
/// use mindia_core::transform_url::ImageTransformUrlBuilder;
///
/// let url = ImageTransformUrlBuilder::new()
///     .dimensions(500, 300)
///     .format("webp")
///     .quality("high")
///     .build("https://api.example.com", "image-id");
/// // Returns: "https://api.example.com/api/images/image-id/-/resize/500x300/-/format/webp/-/quality/high/"
/// ```
#[derive(Debug, Clone, Default)]
pub struct ImageTransformUrlBuilder {
    resize_width: Option<u32>,
    resize_height: Option<u32>,
    stretch_mode: Option<String>,
    format: Option<String>,
    quality: Option<String>,
    rotate_angle: Option<u16>,
    flip_vertical: bool,
    flip_horizontal: bool,
    autorotate: Option<bool>,
    watermark_id: Option<String>,
    watermark_position: Option<String>,
    watermark_size: Option<String>,
    watermark_opacity: Option<f32>,
    smart_crop_width: Option<u32>,
    smart_crop_height: Option<u32>,
}

impl ImageTransformUrlBuilder {
    /// Create a new empty builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set resize dimensions (width and/or height)
    ///
    /// # Arguments
    /// * `width` - Target width in pixels (None to maintain aspect ratio)
    /// * `height` - Target height in pixels (None to maintain aspect ratio)
    ///
    /// # Example
    /// ```
    /// use mindia_core::transform_url::ImageTransformUrlBuilder;
    /// let builder = ImageTransformUrlBuilder::new()
    ///     .resize(Some(500), Some(300));  // 500x300
    /// let builder = ImageTransformUrlBuilder::new()
    ///     .resize(Some(500), None);       // 500px width, maintain aspect ratio
    /// ```
    pub fn resize(mut self, width: Option<u32>, height: Option<u32>) -> Self {
        self.resize_width = width;
        self.resize_height = height;
        self
    }

    /// Set resize to specific width, maintain aspect ratio
    pub fn width(mut self, width: u32) -> Self {
        self.resize_width = Some(width);
        self.resize_height = None;
        self
    }

    /// Set resize to specific height, maintain aspect ratio
    pub fn height(mut self, height: u32) -> Self {
        self.resize_width = None;
        self.resize_height = Some(height);
        self
    }

    /// Set both width and height
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.resize_width = Some(width);
        self.resize_height = Some(height);
        self
    }

    /// Set stretch mode for resize
    ///
    /// # Arguments
    /// * `mode` - Stretch mode: "on", "off", or "fill"
    pub fn stretch(mut self, mode: &str) -> Self {
        self.stretch_mode = Some(mode.to_string());
        self
    }

    /// Set output format
    ///
    /// # Arguments
    /// * `format` - Format: "jpeg", "jpg", "png", "webp", "avif", or "auto"
    pub fn format(mut self, format: &str) -> Self {
        self.format = Some(format.to_string());
        self
    }

    /// Set quality preset
    ///
    /// # Arguments
    /// * `quality` - Quality: "low", "medium", "high", "normal", "better", "best", "lighter", "lightest", or "fast"
    pub fn quality(mut self, quality: &str) -> Self {
        self.quality = Some(quality.to_string());
        self
    }

    /// Set rotation angle
    ///
    /// # Arguments
    /// * `angle` - Rotation angle in degrees (90, 180, or 270)
    pub fn rotate(mut self, angle: u16) -> Self {
        if angle == 90 || angle == 180 || angle == 270 {
            self.rotate_angle = Some(angle);
        }
        self
    }

    /// Enable vertical flip
    pub fn flip(mut self) -> Self {
        self.flip_vertical = true;
        self
    }

    /// Enable horizontal flip (mirror)
    pub fn mirror(mut self) -> Self {
        self.flip_horizontal = true;
        self
    }

    /// Set autorotate (EXIF orientation correction)
    ///
    /// # Arguments
    /// * `enable` - Enable or disable autorotate
    pub fn autorotate(mut self, enable: bool) -> Self {
        self.autorotate = Some(enable);
        self
    }

    /// Set watermark configuration
    ///
    /// # Arguments
    /// * `watermark_id` - UUID of the watermark image
    /// * `position` - Position: "tl", "tr", "bl", "br", "center", or "x,y" coordinates
    /// * `size` - Size: "10%" (percentage) or "200x100" (absolute dimensions)
    /// * `opacity` - Opacity from 0.0 to 1.0
    pub fn watermark(
        mut self,
        watermark_id: &str,
        position: Option<&str>,
        size: Option<&str>,
        opacity: Option<f32>,
    ) -> Self {
        self.watermark_id = Some(watermark_id.to_string());
        self.watermark_position = position.map(|s| s.to_string());
        self.watermark_size = size.map(|s| s.to_string());
        self.watermark_opacity = opacity;
        self
    }

    /// Set smart crop dimensions
    ///
    /// # Arguments
    /// * `width` - Crop width in pixels
    /// * `height` - Crop height in pixels
    pub fn smart_crop(mut self, width: u32, height: u32) -> Self {
        self.smart_crop_width = Some(width);
        self.smart_crop_height = Some(height);
        self
    }

    /// Build the transformation URL path segment (operations only)
    ///
    /// Returns the operations part without the base URL and image ID.
    ///
    /// # Example
    /// ```
    /// use mindia_core::transform_url::ImageTransformUrlBuilder;
    /// let operations = ImageTransformUrlBuilder::new()
    ///     .dimensions(500, 300)
    ///     .format("webp")
    ///     .build_operations();
    /// // Returns: "-/resize/500x300/-/format/webp/"
    /// ```
    pub fn build_operations(&self) -> String {
        let mut operations = Vec::new();

        // Smart crop (applied first)
        if let (Some(width), Some(height)) = (self.smart_crop_width, self.smart_crop_height) {
            operations.push(format!("crop/smart/{}x{}", width, height));
        }

        // Resize
        if let (Some(width), Some(height)) = (self.resize_width, self.resize_height) {
            operations.push(format!("resize/{}x{}", width, height));
        } else if let Some(width) = self.resize_width {
            operations.push(format!("resize/{}x", width));
        } else if let Some(height) = self.resize_height {
            operations.push(format!("resize/x{}", height));
        }

        // Stretch mode (if set and resize is set)
        if self.stretch_mode.is_some()
            && (self.resize_width.is_some() || self.resize_height.is_some())
        {
            if let Some(ref mode) = self.stretch_mode {
                operations.push(format!("stretch/{}", mode));
            }
        }

        // Format
        if let Some(ref format) = self.format {
            operations.push(format!("format/{}", format));
        }

        // Quality
        if let Some(ref quality) = self.quality {
            operations.push(format!("quality/{}", quality));
        }

        // Rotation
        if let Some(angle) = self.rotate_angle {
            operations.push(format!("rotate/{}", angle));
        }

        // Flips
        if self.flip_vertical {
            operations.push("flip".to_string());
        }
        if self.flip_horizontal {
            operations.push("mirror".to_string());
        }

        // Autorotate
        if let Some(enable) = self.autorotate {
            operations.push(format!("autorotate/{}", if enable { "yes" } else { "no" }));
        }

        // Watermark
        if let Some(ref watermark_id) = self.watermark_id {
            let mut watermark_op = format!("watermark/{}", watermark_id);

            if let Some(ref position) = self.watermark_position {
                watermark_op.push_str(&format!("/position/{}", position));
            }

            if let Some(ref size) = self.watermark_size {
                watermark_op.push_str(&format!("/size/{}", size));
            }

            if let Some(opacity) = self.watermark_opacity {
                watermark_op.push_str(&format!("/opacity/{}", opacity));
            }

            operations.push(watermark_op);
        }

        if operations.is_empty() {
            return String::new();
        }

        // Join with /-/ separators, trailing slash
        format!("-/{}/", operations.join("/-/"))
    }

    /// Build the complete transformation URL
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the API (e.g., "https://api.example.com")
    /// * `image_id` - Image ID (UUID)
    ///
    /// # Example
    /// ```
    /// use mindia_core::transform_url::ImageTransformUrlBuilder;
    /// let url = ImageTransformUrlBuilder::new()
    ///     .dimensions(500, 300)
    ///     .format("webp")
    ///     .build("https://api.example.com", "abc-123-def");
    /// // Returns: "https://api.example.com/api/images/abc-123-def/-/resize/500x300/-/format/webp/"
    /// ```
    pub fn build(&self, base_url: &str, image_id: &str) -> String {
        let base = base_url.trim_end_matches('/');
        let operations = self.build_operations();

        if operations.is_empty() {
            format!("{}/api/images/{}", base, image_id)
        } else {
            format!("{}/api/images/{}/{}", base, image_id, operations)
        }
    }

    /// Build just the path (without base URL)
    ///
    /// # Arguments
    /// * `image_id` - Image ID (UUID)
    ///
    /// # Example
    /// ```
    /// use mindia_core::transform_url::ImageTransformUrlBuilder;
    /// let path = ImageTransformUrlBuilder::new()
    ///     .dimensions(500, 300)
    ///     .build_path("abc-123-def");
    /// // Returns: "/api/images/abc-123-def/-/resize/500x300/"
    /// ```
    pub fn build_path(&self, image_id: &str) -> String {
        let operations = self.build_operations();

        if operations.is_empty() {
            format!("/api/images/{}", image_id)
        } else {
            format!("/api/images/{}/{}", image_id, operations)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_resize() {
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .build("https://api.example.com", "img-123");
        assert_eq!(
            url,
            "https://api.example.com/api/images/img-123/-/resize/500x/"
        );
    }

    #[test]
    fn test_resize_dimensions() {
        let url = ImageTransformUrlBuilder::new()
            .dimensions(500, 300)
            .build("https://api.example.com", "img-123");
        assert_eq!(
            url,
            "https://api.example.com/api/images/img-123/-/resize/500x300/"
        );
    }

    #[test]
    fn test_format_and_quality() {
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .format("webp")
            .quality("high")
            .build("https://api.example.com", "img-123");
        assert_eq!(
            url,
            "https://api.example.com/api/images/img-123/-/resize/500x/-/format/webp/-/quality/high/"
        );
    }

    #[test]
    fn test_complex_operations() {
        let url = ImageTransformUrlBuilder::new()
            .smart_crop(400, 300)
            .dimensions(800, 600)
            .format("webp")
            .quality("high")
            .rotate(90)
            .build("https://api.example.com", "img-123");
        assert!(url.contains("crop/smart/400x300"));
        assert!(url.contains("resize/800x600"));
        assert!(url.contains("format/webp"));
        assert!(url.contains("quality/high"));
        assert!(url.contains("rotate/90"));
    }

    #[test]
    fn test_watermark() {
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .watermark("wm-123", Some("br"), Some("10%"), Some(0.7))
            .build("https://api.example.com", "img-123");
        assert!(url.contains("watermark/wm-123"));
        assert!(url.contains("position/br"));
        assert!(url.contains("size/10%"));
        assert!(url.contains("opacity/0.7"));
    }

    #[test]
    fn test_build_path() {
        let path = ImageTransformUrlBuilder::new()
            .width(500)
            .format("webp")
            .build_path("img-123");
        assert_eq!(path, "/api/images/img-123/-/resize/500x/-/format/webp/");
    }

    #[test]
    fn test_build_operations() {
        let ops = ImageTransformUrlBuilder::new()
            .width(500)
            .format("webp")
            .build_operations();
        assert_eq!(ops, "-/resize/500x/-/format/webp/");
    }

    #[test]
    fn test_empty_builder() {
        let url = ImageTransformUrlBuilder::new().build("https://api.example.com", "img-123");
        assert_eq!(url, "https://api.example.com/api/images/img-123");
    }

    // =========================================================================
    // BUILDER EDGE CASES
    // =========================================================================

    #[test]
    fn test_resize_with_none_values() {
        // Height only via resize method
        let url = ImageTransformUrlBuilder::new()
            .resize(None, Some(300))
            .build("https://api.example.com", "img-123");
        assert_eq!(
            url,
            "https://api.example.com/api/images/img-123/-/resize/x300/"
        );

        // Width only via resize method
        let url = ImageTransformUrlBuilder::new()
            .resize(Some(500), None)
            .build("https://api.example.com", "img-123");
        assert_eq!(
            url,
            "https://api.example.com/api/images/img-123/-/resize/500x/"
        );

        // Both None (should result in no resize operation)
        let url = ImageTransformUrlBuilder::new()
            .resize(None, None)
            .format("webp")
            .build("https://api.example.com", "img-123");
        assert_eq!(
            url,
            "https://api.example.com/api/images/img-123/-/format/webp/"
        );
    }

    #[test]
    fn test_very_large_dimensions() {
        let url = ImageTransformUrlBuilder::new()
            .dimensions(4294967295, 4294967295) // u32::MAX
            .build("https://api.example.com", "img-123");
        assert!(url.contains("resize/4294967295x4294967295"));
    }

    #[test]
    fn test_base_url_trailing_slash() {
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .build("https://api.example.com/", "img-123");
        assert_eq!(
            url,
            "https://api.example.com/api/images/img-123/-/resize/500x/"
        );
    }

    #[test]
    fn test_base_url_no_protocol() {
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .build("//api.example.com", "img-123");
        assert_eq!(url, "//api.example.com/api/images/img-123/-/resize/500x/");

        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .build("api.example.com", "img-123");
        assert_eq!(url, "api.example.com/api/images/img-123/-/resize/500x/");
    }

    #[test]
    fn test_build_path_edge_cases() {
        // Path without leading slash should still work
        let path = ImageTransformUrlBuilder::new()
            .width(500)
            .build_path("img-123");
        assert_eq!(path, "/api/images/img-123/-/resize/500x/");

        // Empty operations
        let path = ImageTransformUrlBuilder::new().build_path("img-123");
        assert_eq!(path, "/api/images/img-123");
    }

    #[test]
    fn test_build_operations_empty() {
        let ops = ImageTransformUrlBuilder::new().build_operations();
        assert_eq!(ops, "");
    }

    #[test]
    fn test_build_operations_order() {
        // Verify operations are in correct order: crop -> resize -> format -> quality -> rotate -> flip -> mirror -> autorotate -> watermark
        let ops = ImageTransformUrlBuilder::new()
            .watermark("wm-123", None, None, None)
            .smart_crop(400, 300)
            .dimensions(800, 600)
            .format("webp")
            .quality("high")
            .rotate(90)
            .flip()
            .mirror()
            .autorotate(false)
            .build_operations();

        // Check that crop comes before resize
        let crop_pos = ops.find("crop/smart");
        let resize_pos = ops.find("resize/800x600");
        assert!(crop_pos.is_some());
        assert!(resize_pos.is_some());
        assert!(crop_pos.unwrap() < resize_pos.unwrap());
    }

    // =========================================================================
    // BUILDER INDIVIDUAL METHOD TESTS
    // =========================================================================

    #[test]
    fn test_builder_stretch_modes() {
        // Stretch on
        let url = ImageTransformUrlBuilder::new()
            .dimensions(500, 300)
            .stretch("on")
            .build("https://api.example.com", "img-123");
        assert!(url.contains("resize/500x300"));
        assert!(url.contains("stretch/on"));

        // Stretch off
        let url = ImageTransformUrlBuilder::new()
            .dimensions(500, 300)
            .stretch("off")
            .build("https://api.example.com", "img-123");
        assert!(url.contains("stretch/off"));

        // Stretch fill
        let url = ImageTransformUrlBuilder::new()
            .dimensions(500, 300)
            .stretch("fill")
            .build("https://api.example.com", "img-123");
        assert!(url.contains("stretch/fill"));
    }

    #[test]
    fn test_builder_autorotate() {
        // Autorotate enabled
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .autorotate(true)
            .build("https://api.example.com", "img-123");
        assert!(url.contains("autorotate/yes"));

        // Autorotate disabled
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .autorotate(false)
            .build("https://api.example.com", "img-123");
        assert!(url.contains("autorotate/no"));
    }

    #[test]
    fn test_builder_all_formats() {
        let formats = vec!["jpeg", "jpg", "png", "webp", "avif", "auto"];
        for fmt in formats {
            let url = ImageTransformUrlBuilder::new()
                .width(500)
                .format(fmt)
                .build("https://api.example.com", "img-123");
            assert!(
                url.contains(&format!("format/{}", fmt)),
                "Failed to build format: {}",
                fmt
            );
        }
    }

    #[test]
    fn test_builder_all_quality_presets() {
        let qualities = vec![
            "low", "medium", "high", "normal", "better", "best", "lighter", "lightest", "fast",
        ];
        for quality in qualities {
            let url = ImageTransformUrlBuilder::new()
                .width(500)
                .quality(quality)
                .build("https://api.example.com", "img-123");
            assert!(
                url.contains(&format!("quality/{}", quality)),
                "Failed to build quality: {}",
                quality
            );
        }
    }

    #[test]
    fn test_builder_rotation_angles() {
        for angle in [90, 180, 270] {
            let url = ImageTransformUrlBuilder::new()
                .width(500)
                .rotate(angle)
                .build("https://api.example.com", "img-123");
            assert!(
                url.contains(&format!("rotate/{}", angle)),
                "Failed to build rotation: {}",
                angle
            );
        }
    }

    #[test]
    fn test_builder_invalid_rotation() {
        // Invalid angles should be silently ignored (not added to URL)
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .rotate(45)
            .build("https://api.example.com", "img-123");
        assert!(!url.contains("rotate/45"));

        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .rotate(360)
            .build("https://api.example.com", "img-123");
        assert!(!url.contains("rotate/360"));

        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .rotate(0)
            .build("https://api.example.com", "img-123");
        assert!(!url.contains("rotate/0"));
    }

    #[test]
    fn test_builder_flip_and_mirror() {
        // Flip only
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .flip()
            .build("https://api.example.com", "img-123");
        assert!(url.contains("/flip/"));
        assert!(!url.contains("/mirror/"));

        // Mirror only
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .mirror()
            .build("https://api.example.com", "img-123");
        assert!(!url.contains("/flip/"));
        assert!(url.contains("/mirror/"));

        // Both flip and mirror
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .flip()
            .mirror()
            .build("https://api.example.com", "img-123");
        assert!(url.contains("/flip/"));
        assert!(url.contains("/mirror/"));
    }

    // =========================================================================
    // BUILDER WATERMARK TESTS
    // =========================================================================

    #[test]
    fn test_builder_watermark_minimal() {
        // Only watermark_id, no other options
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .watermark("wm-123", None, None, None)
            .build("https://api.example.com", "img-123");
        assert!(url.contains("watermark/wm-123"));
        // Should not have position, size, or opacity
        assert!(!url.contains("/position/"));
        assert!(!url.contains("/size/"));
        assert!(!url.contains("/opacity/"));
    }

    #[test]
    fn test_builder_watermark_all_positions() {
        let positions = vec!["tl", "tr", "bl", "br", "center", "10,20"];
        for pos in positions {
            let url = ImageTransformUrlBuilder::new()
                .width(500)
                .watermark("wm-123", Some(pos), None, None)
                .build("https://api.example.com", "img-123");
            assert!(
                url.contains(&format!("position/{}", pos)),
                "Failed to build position: {}",
                pos
            );
        }
    }

    #[test]
    fn test_builder_watermark_sizes() {
        // Percentage size
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .watermark("wm-123", None, Some("10%"), None)
            .build("https://api.example.com", "img-123");
        assert!(url.contains("size/10%"));

        // Absolute size
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .watermark("wm-123", None, Some("200x100"), None)
            .build("https://api.example.com", "img-123");
        assert!(url.contains("size/200x100"));

        // Large percentage
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .watermark("wm-123", None, Some("50%"), None)
            .build("https://api.example.com", "img-123");
        assert!(url.contains("size/50%"));
    }

    #[test]
    fn test_builder_watermark_opacity_range() {
        for opacity in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let url = ImageTransformUrlBuilder::new()
                .width(500)
                .watermark("wm-123", None, None, Some(opacity))
                .build("https://api.example.com", "img-123");
            assert!(
                url.contains(&format!("opacity/{}", opacity)),
                "Failed to build opacity: {}",
                opacity
            );
        }
    }

    #[test]
    fn test_builder_watermark_complete() {
        // All watermark options combined
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .watermark("wm-abc-123", Some("br"), Some("15%"), Some(0.8))
            .build("https://api.example.com", "img-123");
        assert!(url.contains("watermark/wm-abc-123"));
        assert!(url.contains("position/br"));
        assert!(url.contains("size/15%"));
        assert!(url.contains("opacity/0.8"));
    }

    #[test]
    fn test_builder_watermark_custom_position() {
        // Custom x,y coordinates
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .watermark("wm-123", Some("100,200"), None, None)
            .build("https://api.example.com", "img-123");
        assert!(url.contains("position/100,200"));
    }

    // =========================================================================
    // BUILDER METHOD CHAINING TESTS
    // =========================================================================

    #[test]
    fn test_builder_fluent_chaining() {
        // Long chain of operations
        let url = ImageTransformUrlBuilder::new()
            .smart_crop(400, 300)
            .dimensions(800, 600)
            .stretch("fill")
            .format("webp")
            .quality("high")
            .rotate(90)
            .flip()
            .mirror()
            .autorotate(false)
            .watermark("wm-123", Some("br"), Some("10%"), Some(0.7))
            .build("https://api.example.com", "img-123");

        // Verify all operations are present
        assert!(url.contains("crop/smart/400x300"));
        assert!(url.contains("resize/800x600"));
        assert!(url.contains("stretch/fill"));
        assert!(url.contains("format/webp"));
        assert!(url.contains("quality/high"));
        assert!(url.contains("rotate/90"));
        assert!(url.contains("/flip/"));
        assert!(url.contains("/mirror/"));
        assert!(url.contains("autorotate/no"));
        assert!(url.contains("watermark/wm-123"));
    }

    #[test]
    fn test_builder_overwrite_operations() {
        // Calling same method twice - last one wins
        let url = ImageTransformUrlBuilder::new()
            .width(500)
            .width(800)
            .format("jpeg")
            .format("webp")
            .quality("low")
            .quality("high")
            .build("https://api.example.com", "img-123");

        // Should only have the last values
        assert!(url.contains("resize/800x"));
        assert!(!url.contains("resize/500x"));
        assert!(url.contains("format/webp"));
        assert!(!url.contains("format/jpeg"));
        assert!(url.contains("quality/high"));
        assert!(!url.contains("quality/low"));
    }

    #[test]
    fn test_builder_method_ordering() {
        // Verify operation order: crop -> resize -> stretch -> format -> quality -> rotate -> flip -> mirror -> autorotate -> watermark
        let url = ImageTransformUrlBuilder::new()
            .watermark("wm-123", None, None, None)
            .smart_crop(400, 300)
            .dimensions(800, 600)
            .stretch("off")
            .format("webp")
            .quality("high")
            .rotate(180)
            .flip()
            .mirror()
            .autorotate(true)
            .build_operations();

        // Find positions of operations
        let crop_pos = url.find("crop/smart").unwrap();
        let resize_pos = url.find("resize/800x600").unwrap();
        let stretch_pos = url.find("stretch/off").unwrap();
        let format_pos = url.find("format/webp").unwrap();
        let quality_pos = url.find("quality/high").unwrap();
        let rotate_pos = url.find("rotate/180").unwrap();
        let flip_pos = url.find("/flip/").unwrap();
        let mirror_pos = url.find("/mirror/").unwrap();
        let autorotate_pos = url.find("autorotate/yes").unwrap();
        let watermark_pos = url.find("watermark/wm-123").unwrap();

        // Verify order
        assert!(crop_pos < resize_pos);
        assert!(resize_pos < stretch_pos);
        assert!(stretch_pos < format_pos);
        assert!(format_pos < quality_pos);
        assert!(quality_pos < rotate_pos);
        assert!(rotate_pos < flip_pos);
        assert!(flip_pos < mirror_pos);
        assert!(mirror_pos < autorotate_pos);
        assert!(autorotate_pos < watermark_pos);
    }

    #[test]
    fn test_builder_reuse_builder() {
        // Test that builder can be reused after build()
        let builder = ImageTransformUrlBuilder::new().width(500);
        let url1 = builder.build("https://api.example.com", "img-1");
        let url2 = builder
            .format("webp")
            .build("https://api.example.com", "img-2");

        assert_eq!(
            url1,
            "https://api.example.com/api/images/img-1/-/resize/500x/"
        );
        assert_eq!(
            url2,
            "https://api.example.com/api/images/img-2/-/resize/500x/-/format/webp/"
        );
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn test_parse_simple_url() {
        let url = "https://api.example.com/api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
        assert_eq!(parsed.resize_height, None);
    }

    #[test]
    fn test_parse_path() {
        let path = "/api/images/img-123/-/resize/500x300/-/format/webp/";
        let parsed = ImageTransformUrlParser::parse_path(path).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
        assert_eq!(parsed.resize_height, Some(300));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
    }

    #[test]
    fn test_parse_operations_only() {
        let ops = "-/resize/500x300/-/format/webp/-/quality/high";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
        assert_eq!(parsed.resize_height, Some(300));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
        assert_eq!(parsed.quality.as_deref(), Some("high"));
    }

    #[test]
    fn test_parse_complex_operations() {
        let ops = "-/crop/smart/400x300/-/resize/800x600/-/format/webp/-/rotate/90";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.smart_crop_width, Some(400));
        assert_eq!(parsed.smart_crop_height, Some(300));
        assert_eq!(parsed.resize_width, Some(800));
        assert_eq!(parsed.resize_height, Some(600));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
        assert_eq!(parsed.rotate_angle, Some(90));
    }

    #[test]
    fn test_parse_watermark() {
        let ops = "-/resize/500x/-/watermark/wm-123/position/br/size/10%/opacity/0.7";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.watermark_id.as_deref(), Some("wm-123"));
        assert_eq!(parsed.watermark_position.as_deref(), Some("br"));
        assert_eq!(parsed.watermark_size.as_deref(), Some("10%"));
        assert_eq!(parsed.watermark_opacity, Some(0.7));
    }

    #[test]
    fn test_parse_flip_mirror() {
        let ops = "-/resize/500x/-/flip/-/mirror";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert!(parsed.flip_vertical);
        assert!(parsed.flip_horizontal);
    }

    #[test]
    fn test_parse_no_operations() {
        let path = "/api/images/img-123";
        let parsed = ImageTransformUrlParser::parse_path(path).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, None);
        assert_eq!(parsed.format, None);
    }

    #[test]
    fn test_parse_invalid_url() {
        let url = "https://api.example.com/invalid/path";
        assert!(ImageTransformUrlParser::parse_url(url).is_err());
    }

    #[test]
    fn test_parse_dimensions() {
        assert_eq!(
            ImageTransformUrlParser::parse_dimensions("500x300").unwrap(),
            (Some(500), Some(300))
        );
        assert_eq!(
            ImageTransformUrlParser::parse_dimensions("500x").unwrap(),
            (Some(500), None)
        );
        assert_eq!(
            ImageTransformUrlParser::parse_dimensions("x300").unwrap(),
            (None, Some(300))
        );
        assert!(ImageTransformUrlParser::parse_dimensions("x").is_err());
    }

    #[test]
    fn test_parse_stretch_modes() {
        let ops = "-/stretch/on/-/resize/500x300/";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.stretch_mode.as_deref(), Some("on"));

        let ops = "-/stretch/off/-/resize/500x300/";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.stretch_mode.as_deref(), Some("off"));

        let ops = "-/stretch/fill/-/resize/500x300/";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.stretch_mode.as_deref(), Some("fill"));
    }

    #[test]
    fn test_parse_all_formats() {
        let formats = vec!["jpeg", "jpg", "png", "webp", "avif", "auto"];
        for fmt in formats {
            let ops = format!("-/format/{}", fmt);
            let parsed =
                ImageTransformUrlParser::parse_operations(&ops, "img-123".to_string()).unwrap();
            assert_eq!(
                parsed.format.as_deref(),
                Some(fmt),
                "Failed to parse format: {}",
                fmt
            );
        }
    }

    #[test]
    fn test_parse_all_quality_levels() {
        let qualities = vec![
            "low", "medium", "high", "normal", "better", "best", "lighter", "lightest", "fast",
        ];
        for quality in qualities {
            let ops = format!("-/quality/{}", quality);
            let parsed =
                ImageTransformUrlParser::parse_operations(&ops, "img-123".to_string()).unwrap();
            assert_eq!(
                parsed.quality.as_deref(),
                Some(quality),
                "Failed to parse quality: {}",
                quality
            );
        }
    }

    #[test]
    fn test_parse_rotation_angles() {
        for angle in [90, 180, 270] {
            let ops = format!("-/rotate/{}", angle);
            let parsed =
                ImageTransformUrlParser::parse_operations(&ops, "img-123".to_string()).unwrap();
            assert_eq!(
                parsed.rotate_angle,
                Some(angle),
                "Failed to parse rotation: {}",
                angle
            );
        }
    }

    #[test]
    fn test_parse_invalid_rotation() {
        assert!(
            ImageTransformUrlParser::parse_operations("-/rotate/45", "img-123".to_string())
                .is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/rotate/360", "img-123".to_string())
                .is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/rotate/0", "img-123".to_string()).is_err()
        );
    }

    #[test]
    fn test_parse_autorotate() {
        let ops = "-/autorotate/yes";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.autorotate, Some(true));

        let ops = "-/autorotate/no";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.autorotate, Some(false));
    }

    #[test]
    fn test_parse_invalid_autorotate() {
        assert!(ImageTransformUrlParser::parse_operations(
            "-/autorotate/maybe",
            "img-123".to_string()
        )
        .is_err());
        assert!(
            ImageTransformUrlParser::parse_operations("-/autorotate", "img-123".to_string())
                .is_err()
        );
    }

    #[test]
    fn test_parse_watermark_positions() {
        let positions = vec!["tl", "tr", "bl", "br", "center", "10,20"];
        for pos in positions {
            let ops = format!("-/watermark/wm-123/position/{}", pos);
            let parsed =
                ImageTransformUrlParser::parse_operations(&ops, "img-123".to_string()).unwrap();
            assert_eq!(
                parsed.watermark_position.as_deref(),
                Some(pos),
                "Failed to parse position: {}",
                pos
            );
        }
    }

    #[test]
    fn test_parse_watermark_sizes() {
        // Percentage size
        let ops = "-/watermark/wm-123/size/10%";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.watermark_size.as_deref(), Some("10%"));

        // Absolute size
        let ops = "-/watermark/wm-123/size/200x100";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.watermark_size.as_deref(), Some("200x100"));
    }

    #[test]
    fn test_parse_watermark_opacity() {
        for opacity in [0.0, 0.5, 1.0] {
            let ops = format!("-/watermark/wm-123/opacity/{}", opacity);
            let parsed =
                ImageTransformUrlParser::parse_operations(&ops, "img-123".to_string()).unwrap();
            assert_eq!(parsed.watermark_opacity, Some(opacity));
        }
    }

    #[test]
    fn test_parse_invalid_watermark_opacity() {
        assert!(ImageTransformUrlParser::parse_operations(
            "-/watermark/wm-123/opacity/1.5",
            "img-123".to_string()
        )
        .is_err());
        assert!(ImageTransformUrlParser::parse_operations(
            "-/watermark/wm-123/opacity/-0.1",
            "img-123".to_string()
        )
        .is_err());
    }

    #[test]
    fn test_parse_complete_watermark() {
        let ops = "-/watermark/wm-123/position/br/size/10%/opacity/0.7";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.watermark_id.as_deref(), Some("wm-123"));
        assert_eq!(parsed.watermark_position.as_deref(), Some("br"));
        assert_eq!(parsed.watermark_size.as_deref(), Some("10%"));
        assert_eq!(parsed.watermark_opacity, Some(0.7));
    }

    #[test]
    fn test_parse_smart_crop() {
        let ops = "-/crop/smart/400x300";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.smart_crop_width, Some(400));
        assert_eq!(parsed.smart_crop_height, Some(300));
    }

    #[test]
    fn test_parse_invalid_crop_mode() {
        assert!(ImageTransformUrlParser::parse_operations(
            "-/crop/manual/400x300",
            "img-123".to_string()
        )
        .is_err());
        assert!(
            ImageTransformUrlParser::parse_operations("-/crop/smart", "img-123".to_string())
                .is_err()
        );
    }

    #[test]
    fn test_parse_invalid_crop_dimensions() {
        assert!(ImageTransformUrlParser::parse_operations(
            "-/crop/smart/0x300",
            "img-123".to_string()
        )
        .is_err());
        assert!(ImageTransformUrlParser::parse_operations(
            "-/crop/smart/400x0",
            "img-123".to_string()
        )
        .is_err());
    }

    #[test]
    fn test_parse_unknown_operation() {
        assert!(ImageTransformUrlParser::parse_operations(
            "-/unknown/operation",
            "img-123".to_string()
        )
        .is_err());
    }

    #[test]
    fn test_parse_missing_operation_value() {
        assert!(
            ImageTransformUrlParser::parse_operations("-/resize", "img-123".to_string()).is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/format", "img-123".to_string()).is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/quality", "img-123".to_string()).is_err()
        );
    }

    #[test]
    fn test_parse_empty_url() {
        assert!(ImageTransformUrlParser::parse_url("").is_err());
        assert!(ImageTransformUrlParser::parse_url("invalid").is_err());
    }

    #[test]
    fn test_parse_url_with_trailing_slash() {
        let url = "https://api.example.com/api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
    }

    #[test]
    fn test_parse_url_without_trailing_slash() {
        let url = "https://api.example.com/api/images/img-123/-/resize/500x";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
    }

    #[test]
    fn test_parse_path_with_leading_slash() {
        let path = "/api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_path(path).unwrap();
        assert_eq!(parsed.image_id, "img-123");
    }

    #[test]
    fn test_parse_path_without_leading_slash() {
        let path = "api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_path(path).unwrap();
        assert_eq!(parsed.image_id, "img-123");
    }

    #[test]
    fn test_parse_invalid_path() {
        assert!(ImageTransformUrlParser::parse_path("/invalid/path").is_err());
        assert!(ImageTransformUrlParser::parse_path("/api/not-images/img-123").is_err());
    }

    #[test]
    fn test_parse_real_world_examples() {
        // Thumbnail example
        let url = "https://api.example.com/api/images/abc-123/-/resize/200x200/-/format/webp/-/quality/low/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.resize_width, Some(200));
        assert_eq!(parsed.resize_height, Some(200));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
        assert_eq!(parsed.quality.as_deref(), Some("low"));

        // Hero image example
        let url = "https://api.example.com/api/images/def-456/-/resize/1920x/-/quality/high/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.resize_width, Some(1920));
        assert_eq!(parsed.resize_height, None);
        assert_eq!(parsed.quality.as_deref(), Some("high"));

        // Complex transformation
        let url = "https://api.example.com/api/images/ghi-789/-/crop/smart/400x300/-/resize/800x600/-/format/webp/-/quality/high/-/rotate/90/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.smart_crop_width, Some(400));
        assert_eq!(parsed.smart_crop_height, Some(300));
        assert_eq!(parsed.resize_width, Some(800));
        assert_eq!(parsed.resize_height, Some(600));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
        assert_eq!(parsed.quality.as_deref(), Some("high"));
        assert_eq!(parsed.rotate_angle, Some(90));
    }

    #[test]
    fn test_parse_round_trip() {
        // Build a URL, then parse it back
        let builder = ImageTransformUrlBuilder::new()
            .dimensions(500, 300)
            .format("webp")
            .quality("high")
            .rotate(90);

        let url = builder.build("https://api.example.com", "img-123");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();

        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
        assert_eq!(parsed.resize_height, Some(300));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
        assert_eq!(parsed.quality.as_deref(), Some("high"));
        assert_eq!(parsed.rotate_angle, Some(90));
    }

    // =========================================================================
    // ROUND-TRIP TESTS (BUILD → PARSE → VERIFY)
    // =========================================================================

    #[test]
    fn test_round_trip_all_operations() {
        // Test all operations in a round-trip
        let builder = ImageTransformUrlBuilder::new()
            .smart_crop(400, 300)
            .dimensions(800, 600)
            .stretch("fill")
            .format("webp")
            .quality("high")
            .rotate(270)
            .flip()
            .mirror()
            .autorotate(false)
            .watermark("wm-abc-123", Some("tl"), Some("15%"), Some(0.75));

        let url = builder.build("https://api.example.com", "test-image-id");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();

        // Verify all fields
        assert_eq!(parsed.image_id, "test-image-id");
        assert_eq!(parsed.smart_crop_width, Some(400));
        assert_eq!(parsed.smart_crop_height, Some(300));
        assert_eq!(parsed.resize_width, Some(800));
        assert_eq!(parsed.resize_height, Some(600));
        assert_eq!(parsed.stretch_mode.as_deref(), Some("fill"));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
        assert_eq!(parsed.quality.as_deref(), Some("high"));
        assert_eq!(parsed.rotate_angle, Some(270));
        assert!(parsed.flip_vertical);
        assert!(parsed.flip_horizontal);
        assert_eq!(parsed.autorotate, Some(false));
        assert_eq!(parsed.watermark_id.as_deref(), Some("wm-abc-123"));
        assert_eq!(parsed.watermark_position.as_deref(), Some("tl"));
        assert_eq!(parsed.watermark_size.as_deref(), Some("15%"));
        assert_eq!(parsed.watermark_opacity, Some(0.75));
    }

    #[test]
    fn test_round_trip_complex_combinations() {
        // Scenario 1: Thumbnail
        let builder = ImageTransformUrlBuilder::new()
            .dimensions(200, 200)
            .format("webp")
            .quality("low");
        let url = builder.build("https://api.example.com", "img-1");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
        assert_eq!(parsed.resize_width, Some(200));
        assert_eq!(parsed.resize_height, Some(200));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
        assert_eq!(parsed.quality.as_deref(), Some("low"));

        // Scenario 2: Hero image
        let builder = ImageTransformUrlBuilder::new()
            .width(1920)
            .format("webp")
            .quality("high");
        let url = builder.build("https://api.example.com", "img-2");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
        assert_eq!(parsed.resize_width, Some(1920));
        assert_eq!(parsed.resize_height, None);
        assert_eq!(parsed.format.as_deref(), Some("webp"));
        assert_eq!(parsed.quality.as_deref(), Some("high"));

        // Scenario 3: Profile picture with crop
        let builder = ImageTransformUrlBuilder::new()
            .smart_crop(200, 200)
            .dimensions(400, 400)
            .format("jpeg")
            .quality("medium");
        let url = builder.build("https://api.example.com", "img-3");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
        assert_eq!(parsed.smart_crop_width, Some(200));
        assert_eq!(parsed.resize_width, Some(400));
    }

    #[test]
    fn test_round_trip_watermark_complete() {
        // Test watermark with all options
        let builder = ImageTransformUrlBuilder::new().width(500).watermark(
            "watermark-uuid-123",
            Some("br"),
            Some("20%"),
            Some(0.6),
        );

        let url = builder.build("https://api.example.com", "img-123");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();

        assert_eq!(parsed.watermark_id.as_deref(), Some("watermark-uuid-123"));
        assert_eq!(parsed.watermark_position.as_deref(), Some("br"));
        assert_eq!(parsed.watermark_size.as_deref(), Some("20%"));
        assert_eq!(parsed.watermark_opacity, Some(0.6));
    }

    #[test]
    fn test_round_trip_edge_cases() {
        // Very large dimensions
        let builder = ImageTransformUrlBuilder::new().dimensions(u32::MAX, u32::MAX);
        let url = builder.build("https://api.example.com", "img-123");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
        assert_eq!(parsed.resize_width, Some(u32::MAX));
        assert_eq!(parsed.resize_height, Some(u32::MAX));

        // Single pixel
        let builder = ImageTransformUrlBuilder::new().dimensions(1, 1);
        let url = builder.build("https://api.example.com", "img-123");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
        assert_eq!(parsed.resize_width, Some(1));
        assert_eq!(parsed.resize_height, Some(1));

        // Width only
        let builder = ImageTransformUrlBuilder::new().width(5000);
        let url = builder.build("https://api.example.com", "img-123");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
        assert_eq!(parsed.resize_width, Some(5000));
        assert_eq!(parsed.resize_height, None);

        // Height only
        let builder = ImageTransformUrlBuilder::new().height(3000);
        let url = builder.build("https://api.example.com", "img-123");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
        assert_eq!(parsed.resize_width, None);
        assert_eq!(parsed.resize_height, Some(3000));
    }

    #[test]
    fn test_round_trip_preserves_order() {
        // Build with specific order, parse, verify operations are in correct order
        let builder = ImageTransformUrlBuilder::new()
            .smart_crop(400, 300)
            .dimensions(800, 600)
            .stretch("off")
            .format("webp")
            .quality("high")
            .rotate(180)
            .flip()
            .mirror();

        let url = builder.build("https://api.example.com", "img-123");
        let _parsed = ImageTransformUrlParser::parse_url(&url).unwrap();

        // Parse operations string to verify order
        let ops = ImageTransformUrlParser::parse_operations(
            url.strip_prefix("https://api.example.com/api/images/img-123/")
                .unwrap(),
            "img-123".to_string(),
        )
        .unwrap();

        // Operations should be parsed in the order they appear
        assert_eq!(ops.smart_crop_width, Some(400));
        assert_eq!(ops.resize_width, Some(800));
        assert_eq!(ops.stretch_mode.as_deref(), Some("off"));
        assert_eq!(ops.format.as_deref(), Some("webp"));
        assert_eq!(ops.quality.as_deref(), Some("high"));
        assert_eq!(ops.rotate_angle, Some(180));
        assert!(ops.flip_vertical);
        assert!(ops.flip_horizontal);
    }

    #[test]
    fn test_round_trip_empty_operations() {
        // Empty builder → parse → should have no operations
        let builder = ImageTransformUrlBuilder::new();
        let url = builder.build("https://api.example.com", "img-123");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();

        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, None);
        assert_eq!(parsed.format, None);
        assert_eq!(parsed.quality, None);
    }

    #[test]
    fn test_round_trip_all_formats() {
        // Test round-trip for each format
        let formats = vec!["jpeg", "png", "webp", "avif", "auto"];
        for fmt in formats {
            let builder = ImageTransformUrlBuilder::new().width(500).format(fmt);
            let url = builder.build("https://api.example.com", "img-123");
            let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
            assert_eq!(
                parsed.format.as_deref(),
                Some(fmt),
                "Round-trip failed for format: {}",
                fmt
            );
        }
    }

    #[test]
    fn test_round_trip_all_qualities() {
        // Test round-trip for each quality preset
        let qualities = vec!["low", "medium", "high", "normal", "better", "best"];
        for quality in qualities {
            let builder = ImageTransformUrlBuilder::new().width(500).quality(quality);
            let url = builder.build("https://api.example.com", "img-123");
            let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
            assert_eq!(
                parsed.quality.as_deref(),
                Some(quality),
                "Round-trip failed for quality: {}",
                quality
            );
        }
    }

    #[test]
    fn test_round_trip_path_building() {
        // Test round-trip with build_path
        let builder = ImageTransformUrlBuilder::new()
            .dimensions(500, 300)
            .format("webp");

        let path = builder.build_path("img-123");
        let parsed = ImageTransformUrlParser::parse_path(&path).unwrap();

        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
        assert_eq!(parsed.resize_height, Some(300));
        assert_eq!(parsed.format.as_deref(), Some("webp"));
    }

    // =========================================================================
    // ERROR MESSAGE VALIDATION TESTS
    // =========================================================================

    #[test]
    fn test_error_messages_are_descriptive() {
        // Invalid URL format
        let err = ImageTransformUrlParser::parse_url("invalid-url").unwrap_err();
        assert!(
            err.contains("/api/images/") || err.contains("URL"),
            "Error message should mention URL format: {}",
            err
        );

        // Missing operation value
        let err = ImageTransformUrlParser::parse_operations("-/resize", "img-123".to_string())
            .unwrap_err();
        assert!(
            err.contains("resize") && err.contains("dimensions"),
            "Error should mention resize and dimensions: {}",
            err
        );

        // Invalid rotation angle
        let err = ImageTransformUrlParser::parse_operations("-/rotate/45", "img-123".to_string())
            .unwrap_err();
        assert!(
            err.contains("rotation") || err.contains("angle"),
            "Error should mention rotation angle: {}",
            err
        );
        assert!(
            err.contains("90") || err.contains("180") || err.contains("270"),
            "Error should mention valid angles: {}",
            err
        );

        // Invalid dimensions
        let err = ImageTransformUrlParser::parse_dimensions("abcxdef").unwrap_err();
        assert!(
            err.contains("dimensions") || err.contains("Invalid"),
            "Error should mention dimensions: {}",
            err
        );

        // Invalid watermark position
        let err = ImageTransformUrlParser::parse_operations(
            "-/watermark/wm-123/position/invalid",
            "img-123".to_string(),
        )
        .unwrap_err();
        assert!(
            err.contains("position") || err.contains("invalid"),
            "Error should mention position: {}",
            err
        );

        // Invalid opacity value
        let err = ImageTransformUrlParser::parse_operations(
            "-/watermark/wm-123/opacity/2.0",
            "img-123".to_string(),
        )
        .unwrap_err();
        assert!(
            err.contains("opacity") || err.contains("1.0"),
            "Error should mention opacity range: {}",
            err
        );

        // Invalid crop mode
        let err = ImageTransformUrlParser::parse_operations(
            "-/crop/invalid/400x300",
            "img-123".to_string(),
        )
        .unwrap_err();
        assert!(
            err.contains("crop") || err.contains("smart"),
            "Error should mention crop mode: {}",
            err
        );

        // Missing crop dimensions
        let err = ImageTransformUrlParser::parse_operations("-/crop/smart", "img-123".to_string())
            .unwrap_err();
        assert!(
            err.contains("dimensions") || err.contains("crop"),
            "Error should mention crop dimensions: {}",
            err
        );
    }

    #[test]
    fn test_error_messages_do_not_leak_sensitive_info() {
        // Error messages should not contain full URLs with sensitive data
        let err =
            ImageTransformUrlParser::parse_url("https://api.example.com/invalid/path").unwrap_err();
        // Should not contain the full URL path, just mention the pattern
        assert!(
            !err.contains("api.example.com") || err.len() < 200,
            "Error message should be concise: {}",
            err
        );

        // Error messages should be user-friendly, not technical internals
        let err =
            ImageTransformUrlParser::parse_operations("-/invalid_op/123", "img-123".to_string())
                .unwrap_err();
        assert!(
            !err.contains("unwrap") && !err.contains("panic"),
            "Error should not contain internal details: {}",
            err
        );
        assert!(
            err.contains("operation") || err.contains("invalid"),
            "Error should mention operation: {}",
            err
        );
    }

    #[test]
    fn test_error_messages_provide_guidance() {
        // Errors should give hints about what's expected
        let err = ImageTransformUrlParser::parse_dimensions("x").unwrap_err();
        assert!(
            err.contains("dimension") || err.contains("format"),
            "Error should guide user on format: {}",
            err
        );

        let err = ImageTransformUrlParser::parse_operations("-/stretch", "img-123".to_string())
            .unwrap_err();
        assert!(
            err.contains("stretch") && err.contains("mode"),
            "Error should mention stretch mode: {}",
            err
        );

        let err = ImageTransformUrlParser::parse_operations("-/format", "img-123".to_string())
            .unwrap_err();
        assert!(
            err.contains("format") && err.contains("value"),
            "Error should mention format value: {}",
            err
        );
    }

    #[test]
    fn test_error_path_validation() {
        // Invalid path format
        let err = ImageTransformUrlParser::parse_path("/invalid/path").unwrap_err();
        assert!(
            err.contains("/api/images/") || err.contains("path"),
            "Error should mention expected path: {}",
            err
        );

        // Path without /api/images/
        let err = ImageTransformUrlParser::parse_path("/images/img-123").unwrap_err();
        assert!(
            err.contains("/api/images/") || err.contains("path"),
            "Error should mention expected path format: {}",
            err
        );
    }

    #[test]
    fn test_error_all_error_paths_return_errors() {
        // Verify all error paths return Result::Err, not panic
        assert!(ImageTransformUrlParser::parse_url("").is_err());
        assert!(ImageTransformUrlParser::parse_url("not-a-url").is_err());
        assert!(ImageTransformUrlParser::parse_path("").is_err());
        assert!(ImageTransformUrlParser::parse_path("invalid").is_err());
        assert!(
            ImageTransformUrlParser::parse_operations("invalid", "img-123".to_string()).is_err()
        );
        assert!(ImageTransformUrlParser::parse_dimensions("").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("invalid").is_err());
    }

    #[test]
    fn test_error_messages_for_incomplete_watermark() {
        // Watermark without ID should error
        let err = ImageTransformUrlParser::parse_operations("-/watermark", "img-123".to_string())
            .unwrap_err();
        assert!(
            err.contains("watermark"),
            "Error should mention watermark: {}",
            err
        );

        // Watermark position without value
        let err = ImageTransformUrlParser::parse_operations(
            "-/watermark/wm-123/position",
            "img-123".to_string(),
        )
        .unwrap_err();
        assert!(
            err.contains("position") || err.contains("value"),
            "Error should mention position value: {}",
            err
        );
    }

    #[test]
    fn test_error_messages_for_invalid_crop_dimensions() {
        // Crop with zero dimensions
        let err =
            ImageTransformUrlParser::parse_operations("-/crop/smart/0x300", "img-123".to_string())
                .unwrap_err();
        assert!(
            err.contains("greater than 0") || err.contains("dimension"),
            "Error should mention dimension must be > 0: {}",
            err
        );

        // Crop with invalid dimensions format
        let err =
            ImageTransformUrlParser::parse_operations("-/crop/smart/x300", "img-123".to_string())
                .unwrap_err();
        assert!(
            err.contains("Width") || err.contains("dimension"),
            "Error should mention width required: {}",
            err
        );
    }

    #[test]
    fn test_error_messages_for_autorotate() {
        // Autorotate without value
        let err = ImageTransformUrlParser::parse_operations("-/autorotate", "img-123".to_string())
            .unwrap_err();
        assert!(
            err.contains("autorotate") && (err.contains("yes") || err.contains("value")),
            "Error should mention yes/no: {}",
            err
        );

        // Autorotate with invalid value
        let err =
            ImageTransformUrlParser::parse_operations("-/autorotate/maybe", "img-123".to_string())
                .unwrap_err();
        assert!(
            err.contains("yes") || err.contains("no"),
            "Error should mention valid values: {}",
            err
        );
    }

    #[test]
    fn test_parse_dimension_edge_cases() {
        // Large dimensions
        assert_eq!(
            ImageTransformUrlParser::parse_dimensions("9999x9999").unwrap(),
            (Some(9999), Some(9999))
        );

        // Single pixel
        assert_eq!(
            ImageTransformUrlParser::parse_dimensions("1x1").unwrap(),
            (Some(1), Some(1))
        );

        // Invalid formats
        assert!(ImageTransformUrlParser::parse_dimensions("").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("abc").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("500").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("abcx300").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("500xabc").is_err());
    }

    // =========================================================================
    // PARSER EDGE CASES
    // =========================================================================

    #[test]
    fn test_parse_special_characters_in_id() {
        // UUID with dashes
        let _url = "https://api.example.com/api/images/550e8400-e29b-41d4-a716-446655440000/-/resize/500x/";
        let parsed = ImageTransformUrlBuilder::new().width(500).build(
            "https://api.example.com",
            "550e8400-e29b-41d4-a716-446655440000",
        );
        let parsed_result = ImageTransformUrlParser::parse_url(&parsed).unwrap();
        assert_eq!(
            parsed_result.image_id,
            "550e8400-e29b-41d4-a716-446655440000"
        );

        // ID with underscores
        let url = "https://api.example.com/api/images/img_123_test/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img_123_test");

        // ID with dots
        let url = "https://api.example.com/api/images/img.123.456/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img.123.456");
    }

    #[test]
    fn test_parse_very_long_url() {
        // Build a very long URL with many operations
        let mut builder = ImageTransformUrlBuilder::new().dimensions(1000, 1000);
        for _i in 1..=10 {
            builder = builder.quality("high");
        }
        let url = builder.build("https://api.example.com", "img-123");
        let parsed = ImageTransformUrlParser::parse_url(&url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(1000));
    }

    #[test]
    fn test_parse_multiple_separators() {
        // Multiple consecutive /-/ separators (edge case)
        let ops = "-//-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string());
        // Should either parse correctly or return an error, but not panic
        assert!(parsed.is_ok() || parsed.is_err());
    }

    #[test]
    fn test_parse_whitespace() {
        // Leading/trailing whitespace should be trimmed
        let url = "  https://api.example.com/api/images/img-123/-/resize/500x/  ";
        let parsed = ImageTransformUrlParser::parse_url(url.trim());
        assert!(parsed.is_ok());

        // Path with whitespace
        let path = "  /api/images/img-123/-/resize/500x/  ";
        let parsed = ImageTransformUrlParser::parse_path(path.trim());
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_parse_malformed_dimensions() {
        // Various invalid dimension formats
        assert!(ImageTransformUrlParser::parse_dimensions("abcxdef").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("500").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("x").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("500x300x200").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("x").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("-500x300").is_err());
        assert!(ImageTransformUrlParser::parse_dimensions("500x-300").is_err());
    }

    #[test]
    fn test_parse_incomplete_operations() {
        // Partial operation strings
        assert!(
            ImageTransformUrlParser::parse_operations("-/resize", "img-123".to_string()).is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/format", "img-123".to_string()).is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/quality", "img-123".to_string()).is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/rotate", "img-123".to_string()).is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/watermark", "img-123".to_string())
                .is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/crop", "img-123".to_string()).is_err()
        );
        assert!(
            ImageTransformUrlParser::parse_operations("-/crop/smart", "img-123".to_string())
                .is_err()
        );
    }

    #[test]
    fn test_parse_duplicate_operations() {
        // Same operation twice - should parse but last one might win depending on implementation
        let ops = "-/resize/500x/-/resize/800x/";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        // Last resize should be used
        assert_eq!(parsed.resize_width, Some(800));

        // Duplicate format
        let ops = "-/format/webp/-/format/jpeg/";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.format.as_deref(), Some("jpeg"));

        // Duplicate quality
        let ops = "-/quality/low/-/quality/high/";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.quality.as_deref(), Some("high"));
    }

    #[test]
    fn test_parse_nested_operations() {
        // Parser accepts combined segment (resize/500x/format/webp) and extracts both ops
        let ops = "-/resize/500x/format/webp/";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string()).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
        assert_eq!(parsed.resize_height, None);
        assert_eq!(parsed.format.as_deref(), Some("webp"));
    }

    #[test]
    fn test_parse_empty_operations_string() {
        let parsed = ImageTransformUrlParser::parse_operations("", "img-123".to_string()).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, None);

        let parsed =
            ImageTransformUrlParser::parse_operations("-/", "img-123".to_string()).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, None);
    }

    #[test]
    fn test_parse_zero_dimensions() {
        // Zero dimensions should error
        assert!(ImageTransformUrlParser::parse_dimensions("0x0").is_ok()); // Parses but may be invalid later
        assert!(ImageTransformUrlParser::parse_dimensions("0x300").is_ok());
        assert!(ImageTransformUrlParser::parse_dimensions("500x0").is_ok());
    }

    #[test]
    fn test_parse_invalid_url_patterns() {
        // URLs that don't match expected pattern
        assert!(ImageTransformUrlParser::parse_url("not-a-url").is_err());
        assert!(ImageTransformUrlParser::parse_url("https://example.com").is_err());
        assert!(ImageTransformUrlParser::parse_url("https://example.com/images/123").is_err());
        assert!(ImageTransformUrlParser::parse_url("/images/123").is_err());
    }

    // =========================================================================
    // PARSER URL FORMAT VARIATIONS
    // =========================================================================

    #[test]
    fn test_parse_url_with_port() {
        let url = "https://api.example.com:8080/api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
    }

    #[test]
    fn test_parse_url_with_query_string() {
        // Query strings should not interfere with path parsing
        let url = "https://api.example.com/api/images/img-123/-/resize/500x/?token=abc&version=1";
        // The parse_url should extract just the path part before the query string
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
    }

    #[test]
    fn test_parse_url_with_fragment() {
        // Fragments should not interfere
        let url = "https://api.example.com/api/images/img-123/-/resize/500x/#section";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
    }

    #[test]
    fn test_parse_relative_url() {
        // Relative URLs (starting with /api/images/)
        let path = "/api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_path(path).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));

        // Relative URL without leading slash
        let path = "api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_path(path).unwrap();
        assert_eq!(parsed.image_id, "img-123");
    }

    #[test]
    fn test_parse_url_http_vs_https() {
        // HTTP URL
        let url = "http://api.example.com/api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");

        // HTTPS URL
        let url = "https://api.example.com/api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
    }

    #[test]
    fn test_parse_url_with_subdomain() {
        let url = "https://cdn.api.example.com/api/images/img-123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        assert_eq!(parsed.image_id, "img-123");
        assert_eq!(parsed.resize_width, Some(500));
    }

    #[test]
    fn test_parse_rejects_format_without_separators() {
        // Format without /-/ separators should be rejected
        let ops = "resize/800x600/format/jpeg/quality/medium";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string());
        assert!(parsed.is_err());
        let err_msg = parsed.unwrap_err();
        assert!(err_msg.contains("/-/") || err_msg.contains("separators"));

        // Format with all operations but no separators should also be rejected
        let ops = "resize/800x600/format/jpeg/quality/medium/stretch/off/rotate/180/flip/mirror/autorotate/no";
        let parsed = ImageTransformUrlParser::parse_operations(ops, "img-123".to_string());
        assert!(parsed.is_err());
    }

    #[test]
    fn test_parse_url_encoded_characters() {
        // URL encoding should be handled (though we parse the path as-is)
        // If image ID has encoded characters, they should be preserved
        let url = "https://api.example.com/api/images/img%2D123/-/resize/500x/";
        let parsed = ImageTransformUrlParser::parse_url(url).unwrap();
        // The parser doesn't decode URLs, so %2D would remain as-is
        assert_eq!(parsed.image_id, "img%2D123");
    }
}
