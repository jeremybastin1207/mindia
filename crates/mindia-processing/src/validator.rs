use mindia_core::models::MediaType;
use std::path::Path;

/// Common validation errors for media files
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("File too large: {size} bytes (max: {max} bytes)")]
    FileTooLarge { size: usize, max: usize },

    #[error("Invalid file extension: {extension} (allowed: {allowed:?})")]
    InvalidExtension {
        extension: String,
        allowed: Vec<String>,
    },

    #[error("Invalid content type: {content_type} (allowed: {allowed:?})")]
    InvalidContentType {
        content_type: String,
        allowed: Vec<String>,
    },

    #[error("Invalid filename: {0}")]
    InvalidFilename(String),

    #[error("Empty file")]
    EmptyFile,
}

/// Media file validator
///
/// Provides common validation logic for all media types without coupling
/// to storage implementation details.
pub struct MediaValidator {
    max_file_size: usize,
    allowed_extensions: Vec<String>,
    allowed_content_types: Vec<String>,
}

impl MediaValidator {
    pub fn new(
        max_file_size: usize,
        allowed_extensions: Vec<String>,
        allowed_content_types: Vec<String>,
    ) -> Self {
        Self {
            max_file_size,
            allowed_extensions,
            allowed_content_types,
        }
    }

    /// Validate file size
    pub fn validate_file_size(&self, size: usize) -> Result<(), ValidationError> {
        if size == 0 {
            return Err(ValidationError::EmptyFile);
        }

        if size > self.max_file_size {
            return Err(ValidationError::FileTooLarge {
                size,
                max: self.max_file_size,
            });
        }

        Ok(())
    }

    /// Validate file extension
    pub fn validate_extension(&self, filename: &str) -> Result<(), ValidationError> {
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .ok_or_else(|| ValidationError::InvalidFilename(filename.to_string()))?;

        if !self.allowed_extensions.contains(&extension) {
            return Err(ValidationError::InvalidExtension {
                extension,
                allowed: self.allowed_extensions.clone(),
            });
        }

        Ok(())
    }

    /// Validate content type
    pub fn validate_content_type(&self, content_type: &str) -> Result<(), ValidationError> {
        let normalized = content_type.to_lowercase();

        if !self
            .allowed_content_types
            .iter()
            .any(|ct| ct == &normalized)
        {
            return Err(ValidationError::InvalidContentType {
                content_type: content_type.to_string(),
                allowed: self.allowed_content_types.clone(),
            });
        }

        Ok(())
    }

    /// Validate that Content-Type matches the file extension
    /// This prevents Content-Type spoofing attacks where malicious files
    /// are uploaded with legitimate Content-Types.
    pub fn validate_extension_content_type_match(
        &self,
        filename: &str,
        content_type: &str,
    ) -> Result<(), ValidationError> {
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .ok_or_else(|| ValidationError::InvalidFilename(filename.to_string()))?;

        let normalized_content_type = content_type.to_lowercase();

        // Map common extensions to expected Content-Types
        let expected_content_types: Vec<&str> = match extension.as_str() {
            // Images
            "jpg" | "jpeg" => vec!["image/jpeg"],
            "png" => vec!["image/png"],
            "gif" => vec!["image/gif"],
            "webp" => vec!["image/webp"],
            "avif" => vec!["image/avif"],
            "svg" => vec!["image/svg+xml"],
            "bmp" => vec!["image/bmp"],
            "ico" => vec!["image/x-icon", "image/vnd.microsoft.icon"],
            // Videos
            "mp4" => vec!["video/mp4"],
            "webm" => vec!["video/webm"],
            "mov" => vec!["video/quicktime"],
            "avi" => vec!["video/x-msvideo"],
            "mkv" => vec!["video/x-matroska"],
            "m4v" => vec!["video/x-m4v"],
            // Audio
            "mp3" => vec!["audio/mpeg", "audio/mp3"],
            "wav" => vec!["audio/wav", "audio/wave", "audio/x-wav"],
            "ogg" => vec!["audio/ogg", "application/ogg"],
            "m4a" => vec!["audio/mp4", "audio/x-m4a"],
            "flac" => vec!["audio/flac"],
            "aac" => vec!["audio/aac"],
            // Documents
            "pdf" => vec!["application/pdf"],
            "doc" => vec!["application/msword"],
            "docx" => {
                vec!["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]
            }
            "xls" => vec!["application/vnd.ms-excel"],
            "xlsx" => vec!["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
            "ppt" => vec!["application/vnd.ms-powerpoint"],
            "pptx" => {
                vec!["application/vnd.openxmlformats-officedocument.presentationml.presentation"]
            }
            "txt" => vec!["text/plain"],
            "csv" => vec!["text/csv"],
            "zip" => vec!["application/zip"],
            "tar" => vec!["application/x-tar"],
            "gz" => vec!["application/gzip"],
            _ => {
                // For unknown extensions, skip cross-validation but log a warning
                // The extension and content-type will still be validated individually
                tracing::debug!(
                    extension = %extension,
                    content_type = %content_type,
                    "Unknown extension, skipping Content-Type/extension cross-validation"
                );
                return Ok(());
            }
        };

        // Check if the provided Content-Type matches any expected type for this extension
        if !expected_content_types
            .iter()
            .any(|ct| ct == &normalized_content_type)
        {
            return Err(ValidationError::InvalidContentType {
                content_type: format!(
                    "{} (does not match extension '{}'. Expected one of: {})",
                    content_type,
                    extension,
                    expected_content_types.join(", ")
                ),
                allowed: self.allowed_content_types.clone(),
            });
        }

        Ok(())
    }

    /// Validate all aspects of a file, including Content-Type/extension matching
    pub fn validate_all(
        &self,
        filename: &str,
        content_type: &str,
        file_size: usize,
    ) -> Result<(), ValidationError> {
        self.validate_file_size(file_size)?;
        self.validate_extension(filename)?;
        self.validate_content_type(content_type)?;
        self.validate_extension_content_type_match(filename, content_type)?;
        Ok(())
    }
}

/// Create validator for specific media type
///
/// This is a helper that creates validators based on the media type being uploaded.
#[allow(clippy::too_many_arguments)]
pub fn validator_for_media_type(
    media_type: MediaType,
    image_max_file_size: usize,
    image_allowed_extensions: Vec<String>,
    image_allowed_content_types: Vec<String>,
    video_max_file_size: usize,
    video_allowed_extensions: Vec<String>,
    video_allowed_content_types: Vec<String>,
    audio_max_file_size: usize,
    audio_allowed_extensions: Vec<String>,
    audio_allowed_content_types: Vec<String>,
    document_max_file_size: usize,
    document_allowed_extensions: Vec<String>,
    document_allowed_content_types: Vec<String>,
) -> MediaValidator {
    match media_type {
        MediaType::Image => MediaValidator::new(
            image_max_file_size,
            image_allowed_extensions,
            image_allowed_content_types,
        ),
        MediaType::Video => MediaValidator::new(
            video_max_file_size,
            video_allowed_extensions,
            video_allowed_content_types,
        ),
        MediaType::Audio => MediaValidator::new(
            audio_max_file_size,
            audio_allowed_extensions,
            audio_allowed_content_types,
        ),
        MediaType::Document => MediaValidator::new(
            document_max_file_size,
            document_allowed_extensions,
            document_allowed_content_types,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validator() -> MediaValidator {
        MediaValidator::new(
            1024 * 1024, // 1MB
            vec!["jpg".to_string(), "png".to_string()],
            vec!["image/jpeg".to_string(), "image/png".to_string()],
        )
    }

    #[test]
    fn test_validate_file_size_ok() {
        let validator = test_validator();
        assert!(validator.validate_file_size(512 * 1024).is_ok());
    }

    #[test]
    fn test_validate_file_size_too_large() {
        let validator = test_validator();
        assert!(validator.validate_file_size(2 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_validate_file_size_empty() {
        let validator = test_validator();
        assert!(matches!(
            validator.validate_file_size(0),
            Err(ValidationError::EmptyFile)
        ));
    }

    #[test]
    fn test_validate_extension_ok() {
        let validator = test_validator();
        assert!(validator.validate_extension("test.jpg").is_ok());
        assert!(validator.validate_extension("test.PNG").is_ok()); // case insensitive
    }

    #[test]
    fn test_validate_extension_invalid() {
        let validator = test_validator();
        assert!(validator.validate_extension("test.gif").is_err());
    }

    #[test]
    fn test_validate_content_type_ok() {
        let validator = test_validator();
        assert!(validator.validate_content_type("image/jpeg").is_ok());
        assert!(validator.validate_content_type("IMAGE/PNG").is_ok()); // case insensitive
    }

    #[test]
    fn test_validate_content_type_invalid() {
        let validator = test_validator();
        assert!(validator.validate_content_type("image/gif").is_err());
    }

    #[test]
    fn test_validate_all_ok() {
        let validator = test_validator();
        assert!(validator
            .validate_all("test.jpg", "image/jpeg", 512 * 1024)
            .is_ok());
    }

    #[test]
    fn test_validate_all_fails_on_size() {
        let validator = test_validator();
        assert!(validator
            .validate_all("test.jpg", "image/jpeg", 2 * 1024 * 1024)
            .is_err());
    }

    #[test]
    fn test_validate_all_fails_on_extension() {
        let validator = test_validator();
        assert!(validator
            .validate_all("test.gif", "image/gif", 512 * 1024)
            .is_err());
    }

    #[test]
    fn test_validate_extension_content_type_match_jpeg() {
        let validator = test_validator();
        // Valid match
        assert!(validator
            .validate_extension_content_type_match("test.jpg", "image/jpeg")
            .is_ok());
        assert!(validator
            .validate_extension_content_type_match("test.jpeg", "image/jpeg")
            .is_ok());

        // Invalid match
        assert!(validator
            .validate_extension_content_type_match("test.jpg", "image/png")
            .is_err());
    }

    #[test]
    fn test_validate_extension_content_type_match_png() {
        let validator = test_validator();
        assert!(validator
            .validate_extension_content_type_match("test.png", "image/png")
            .is_ok());
        assert!(validator
            .validate_extension_content_type_match("test.png", "image/jpeg")
            .is_err());
    }

    #[test]
    fn test_validate_extension_content_type_match_video() {
        let validator = MediaValidator::new(
            10 * 1024 * 1024, // 10MB
            vec!["mp4".to_string(), "webm".to_string()],
            vec!["video/mp4".to_string(), "video/webm".to_string()],
        );

        assert!(validator
            .validate_extension_content_type_match("test.mp4", "video/mp4")
            .is_ok());
        assert!(validator
            .validate_extension_content_type_match("test.webm", "video/webm")
            .is_ok());
        assert!(validator
            .validate_extension_content_type_match("test.mp4", "video/webm")
            .is_err());
    }

    #[test]
    fn test_validate_extension_content_type_match_audio() {
        let validator = MediaValidator::new(
            10 * 1024 * 1024,
            vec!["mp3".to_string(), "wav".to_string()],
            vec!["audio/mpeg".to_string(), "audio/wav".to_string()],
        );

        assert!(validator
            .validate_extension_content_type_match("test.mp3", "audio/mpeg")
            .is_ok());
        assert!(validator
            .validate_extension_content_type_match("test.mp3", "audio/mp3")
            .is_ok()); // Multiple valid content types
        assert!(validator
            .validate_extension_content_type_match("test.wav", "audio/wav")
            .is_ok());
        assert!(validator
            .validate_extension_content_type_match("test.wav", "audio/wave")
            .is_ok());
    }

    #[test]
    fn test_validate_extension_content_type_match_document() {
        let validator = MediaValidator::new(
            10 * 1024 * 1024,
            vec!["pdf".to_string()],
            vec!["application/pdf".to_string()],
        );

        assert!(validator
            .validate_extension_content_type_match("test.pdf", "application/pdf")
            .is_ok());
        assert!(validator
            .validate_extension_content_type_match("test.pdf", "image/jpeg")
            .is_err());
    }

    #[test]
    fn test_validate_extension_content_type_match_case_insensitive() {
        let validator = test_validator();
        assert!(validator
            .validate_extension_content_type_match("test.JPG", "image/jpeg")
            .is_ok());
        assert!(validator
            .validate_extension_content_type_match("test.jpg", "IMAGE/JPEG")
            .is_ok());
    }

    #[test]
    fn test_validate_extension_content_type_match_unknown_extension() {
        let validator = test_validator();
        // Unknown extensions should not fail cross-validation
        // (they'll fail individual validation though)
        assert!(validator
            .validate_extension_content_type_match("test.xyz", "application/xyz")
            .is_ok());
    }

    #[test]
    fn test_validate_extension_no_extension() {
        let validator = test_validator();
        assert!(validator.validate_extension("noextension").is_err());
    }

    #[test]
    fn test_validator_for_media_type() {
        use mindia_core::models::MediaType;

        let image_validator = validator_for_media_type(
            MediaType::Image,
            1024 * 1024,
            vec!["jpg".to_string()],
            vec!["image/jpeg".to_string()],
            0,
            vec![],
            vec![], // video
            0,
            vec![],
            vec![], // audio
            0,
            vec![],
            vec![], // document
        );

        assert!(image_validator.validate_extension("test.jpg").is_ok());
        assert!(image_validator.validate_extension("test.png").is_err());
    }
}
