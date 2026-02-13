//! Validation utilities for API handlers

use std::path::Path;

/// Validate that Content-Type matches the file extension
/// This prevents Content-Type spoofing attacks where malicious files
/// are uploaded with legitimate Content-Types.
pub fn validate_extension_content_type_match(
    filename: &str,
    content_type: &str,
) -> Result<(), String> {
    let extension = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if extension.is_empty() {
        return Err("File must have an extension".to_string());
    }

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
        "docx" => vec!["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
        "xls" => vec!["application/vnd.ms-excel"],
        "xlsx" => vec!["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "ppt" => vec!["application/vnd.ms-powerpoint"],
        "pptx" => vec!["application/vnd.openxmlformats-officedocument.presentationml.presentation"],
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
    if !expected_content_types.iter().any(|ct| {
        normalized_content_type == *ct || normalized_content_type.starts_with(&format!("{};", ct))
    }) {
        return Err(format!(
            "Content-Type '{}' does not match extension '{}'. Expected one of: {}",
            content_type,
            extension,
            expected_content_types.join(", ")
        ));
    }

    Ok(())
}
