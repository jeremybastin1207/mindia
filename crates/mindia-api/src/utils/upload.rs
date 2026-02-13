//! Common utilities for file upload handlers

use axum::extract::Multipart;
use chrono::{Duration, Utc};
use mindia_core::AppError;
use mindia_services::ScanResult;

/// Parse and validate store parameter from query string
pub fn parse_store_parameter(
    store: &str,
    auto_store_enabled: bool,
) -> Result<(bool, Option<chrono::DateTime<Utc>>), AppError> {
    let store_behavior = store.to_lowercase();
    if !["0", "1", "auto"].contains(&store_behavior.as_str()) {
        return Err(AppError::InvalidInput(
            "Invalid store parameter. Must be '0', '1', or 'auto'".to_string(),
        ));
    }

    let store_permanently = match store_behavior.as_str() {
        "1" => true,
        "0" => false,
        "auto" => auto_store_enabled,
        _ => auto_store_enabled, // fallback
    };

    let expires_at = if !store_permanently {
        Some(Utc::now() + Duration::hours(24))
    } else {
        None
    };

    Ok((store_permanently, expires_at))
}

/// Extract file data, filename, and content type from multipart form.
/// Only one field named "file" is accepted; multiple file fields are rejected.
pub async fn extract_multipart_file(
    mut multipart: Multipart,
) -> Result<(Vec<u8>, String, String), AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InvalidInput(format!("Failed to read multipart: {}", e)))?
    {
        let field_name = field.name().map(|s| s.to_string()).unwrap_or_default();

        if field_name == "file" {
            if file_data.is_some() {
                return Err(AppError::InvalidInput(
                    "Multiple file fields are not allowed; send exactly one field named 'file'".to_string(),
                ));
            }
            filename = field.file_name().map(|s: &str| s.to_string());
            content_type = field.content_type().map(|s: &str| s.to_string());

            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::InvalidInput(format!("Failed to read file data: {}", e)))?;

            file_data = Some(data.to_vec());
        }
    }

    let file_data =
        file_data.ok_or_else(|| AppError::InvalidInput("No file provided".to_string()))?;

    let original_filename = filename.unwrap_or_else(|| "unknown".to_string());
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    Ok((file_data, original_filename, content_type))
}

/// Validate file size
pub fn validate_file_size(file_size: usize, max_size: usize) -> Result<(), AppError> {
    if file_size > max_size {
        return Err(AppError::PayloadTooLarge(format!(
            "File size exceeds maximum allowed size of {} MB",
            max_size / 1024 / 1024
        )));
    }
    Ok(())
}

/// Normalize MIME type by stripping parameters (e.g. "image/jpeg; charset=utf-8" -> "image/jpeg").
fn normalize_mime_type(content_type: &str) -> &str {
    content_type
        .split(';')
        .next()
        .map(|s| s.trim())
        .unwrap_or(content_type)
}

/// Validate content type against allowlist. Compares normalized MIME type only (no parameter bypass).
pub fn validate_content_type(content_type: &str, allowed_types: &[String]) -> Result<(), AppError> {
    let normalized = normalize_mime_type(content_type).to_lowercase();
    if !allowed_types.iter().any(|ct| normalized == ct.to_lowercase()) {
        return Err(AppError::InvalidInput(format!(
            "Invalid content type. Allowed types: {}",
            allowed_types.join(", ")
        )));
    }
    Ok(())
}

/// Validate file extension
pub fn validate_file_extension(
    filename: &str,
    allowed_extensions: &[String],
) -> Result<String, AppError> {
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    if !allowed_extensions.contains(&extension) {
        return Err(AppError::InvalidInput(format!(
            "Invalid file extension. Allowed extensions: {}",
            allowed_extensions.join(", ")
        )));
    }

    Ok(extension)
}

/// Sanitize filename to prevent path traversal and invalid characters.
/// Returns an error if the filename contains path traversal attempts.
pub fn sanitize_filename(filename: &str) -> Result<String, AppError> {
    const MAX_FILENAME_LENGTH: usize = 255;

    let path = std::path::Path::new(filename);
    let filename_only = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename);

    if filename_only.contains("..") {
        return Err(AppError::InvalidInput(
            "Filename contains invalid path traversal".to_string(),
        ));
    }

    let sanitized: String = filename_only
        .chars()
        .take(MAX_FILENAME_LENGTH)
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.trim().is_empty() || sanitized.len() < 3 {
        return Ok("file".to_string());
    }

    Ok(sanitized)
}

/// Validate Content-Type matches extension (wrapper for validation module)
pub fn validate_extension_content_type_match(
    filename: &str,
    content_type: &str,
) -> Result<(), AppError> {
    crate::validation::validate_extension_content_type_match(filename, content_type)
        .map_err(AppError::InvalidInput)
}

/// Handle ClamAV scan result and return appropriate error if needed
pub async fn handle_clamav_scan(result: ScanResult, filename: &str) -> Result<(), AppError> {
    match result {
        ScanResult::Clean => {
            tracing::debug!("File passed virus scan");
            Ok(())
        }
        ScanResult::Infected(virus_name) => {
            tracing::warn!(
                virus = %virus_name,
                filename = %filename,
                "Rejected infected file upload"
            );
            Err(AppError::InvalidInput(format!(
                "File rejected: virus detected ({})",
                virus_name
            )))
        }
        ScanResult::Error(err) => {
            tracing::error!(
                error = %err,
                filename = %filename,
                "ClamAV scan failed"
            );
            Err(AppError::Internal(
                "Virus scanning temporarily unavailable".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_rejects_path_traversal() {
        assert!(sanitize_filename("..").is_err());
        assert!(sanitize_filename("foo/../bar").is_err());
        assert!(sanitize_filename("....").is_err());
    }

    #[test]
    fn sanitize_filename_accepts_valid_names() {
        assert_eq!(sanitize_filename("image.png").unwrap(), "image.png");
        assert_eq!(sanitize_filename("my-file_1.jpg").unwrap(), "my-file_1.jpg");
    }
}
