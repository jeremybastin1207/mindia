//! HTTP error response conversion
//!
//! This module provides HTTP-specific error response conversion for AppError.
//!
//! **Preferred handler pattern:** Return `Result<impl IntoResponse, HttpAppError>`. Use
//! `AppError` (or types that implement `Into<AppError>`) for errors and `.map_err(Into::into)`
//! so they become `HttpAppError` and render consistently (status, body, logging).

use axum::{
    extract::rejection::JsonRejection,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use mindia_core::{AppError, ErrorMetadata, LogLevel};
use mindia_processing::validator::ValidationError;
use mindia_storage::StorageError;
use serde::{de::DeserializeOwned, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    /// Machine-readable error code for programmatic handling
    pub code: String,
    /// Whether this error is recoverable (can be retried)
    pub recoverable: bool,
    /// Suggested action for the client (e.g., "Wait 60s and retry")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<String>,
}

impl ErrorResponse {
    /// Create a simple error response with default values
    #[allow(dead_code)]
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            details: None,
            error_type: None,
            code: code.into(),
            recoverable: false,
            suggested_action: None,
        }
    }

    /// Create an error response with all fields
    #[allow(dead_code)]
    pub fn full(
        error: impl Into<String>,
        code: impl Into<String>,
        recoverable: bool,
        suggested_action: Option<impl Into<String>>,
    ) -> Self {
        Self {
            error: error.into(),
            details: None,
            error_type: None,
            code: code.into(),
            recoverable,
            suggested_action: suggested_action.map(Into::into),
        }
    }
}

/// Wrapper type for AppError to implement IntoResponse
/// This is necessary because of Rust's orphan rules - we can't implement
/// IntoResponse (external trait) for AppError (external type from mindia-core)
#[derive(Debug)]
pub struct HttpAppError(pub AppError);

impl From<AppError> for HttpAppError {
    fn from(err: AppError) -> Self {
        HttpAppError(err)
    }
}

impl From<anyhow::Error> for HttpAppError {
    fn from(err: anyhow::Error) -> Self {
        HttpAppError(AppError::InternalWithSource {
            message: err.to_string(),
            source: err,
        })
    }
}

/// Convert JSON body deserialization failures into a 400 with our ErrorResponse format.
impl From<JsonRejection> for HttpAppError {
    fn from(rejection: JsonRejection) -> Self {
        let body_text = rejection.body_text();
        let message = if body_text.contains("expected a formatted UUID")
            || body_text.contains("invalid type")
        {
            // User-friendly summary for common cases (e.g. media_id: integer instead of UUID)
            "Invalid request body: check that fields like media_id are UUID strings, not numbers."
                .to_string()
        } else {
            format!("Invalid request body: {}", body_text)
        };
        HttpAppError(AppError::InvalidInput(message))
    }
}

/// JSON body extractor that returns our ErrorResponse format (400 + JSON) on deserialization failure.
/// Use this instead of `Json<T>` when you want a consistent API error shape for invalid bodies.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = HttpAppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(inner) = Json::<T>::from_request(req, state)
            .await
            .map_err(HttpAppError::from)?;
        Ok(ValidatedJson(inner))
    }
}

fn log_error(error: &AppError) {
    let error_type = error.error_type();
    match error.log_level() {
        LogLevel::Debug => {
            tracing::debug!(error = %error, error_type = error_type, "Error occurred");
        }
        LogLevel::Warn => {
            tracing::warn!(error = %error, error_type = error_type, "Error occurred");
        }
        LogLevel::Error => {
            tracing::error!(error = %error, error_type = error_type, "Error occurred");
        }
    }
}

fn is_production_env() -> bool {
    std::env::var("ENVIRONMENT")
        .or_else(|_| std::env::var("APP_ENV"))
        .map(|env| env.to_lowercase() == "production" || env.to_lowercase() == "prod")
        .unwrap_or(false)
}

impl IntoResponse for HttpAppError {
    fn into_response(self) -> Response {
        let app_error = &self.0;
        let is_production = is_production_env();

        let status = StatusCode::from_u16(app_error.http_status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        log_error(app_error);

        // Always hide details in production for security; in non-production, only show details for non-sensitive errors.
        let body = if is_production || app_error.is_sensitive() {
            Json(ErrorResponse {
                error: app_error.client_message(),
                details: None,
                error_type: None,
                code: app_error.error_code().to_string(),
                recoverable: app_error.is_recoverable(),
                suggested_action: app_error.suggested_action().map(String::from),
            })
        } else {
            Json(ErrorResponse {
                error: app_error.client_message(),
                details: Some(app_error.detailed_message()),
                error_type: Some(app_error.error_type().to_string()),
                code: app_error.error_code().to_string(),
                recoverable: app_error.is_recoverable(),
                suggested_action: app_error.suggested_action().map(String::from),
            })
        };

        (status, body).into_response()
    }
}

/// Converts HttpAppError to Response and attaches error context to the wide event.
pub fn error_response_with_event(
    error: HttpAppError,
    mut event: crate::telemetry::wide_event::WideEvent,
) -> Response {
    let app_error = &error.0;

    let status = StatusCode::from_u16(app_error.http_status_code())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let error_message = app_error.client_message();
    let error_code = app_error.error_code();
    let retriable = app_error.is_recoverable();

    event.with_error(crate::telemetry::wide_event::ErrorContext {
        error_type: app_error.error_type().to_string(),
        code: Some(error_code.to_string()),
        message: error_message.clone(),
        retriable,
        stack_trace: None,
    });

    let is_production = is_production_env();

    let error_response = if is_production || app_error.is_sensitive() {
        ErrorResponse {
            error: error_message,
            details: None,
            error_type: None,
            code: error_code.to_string(),
            recoverable: retriable,
            suggested_action: app_error.suggested_action().map(String::from),
        }
    } else {
        ErrorResponse {
            error: error_message,
            details: Some(app_error.detailed_message()),
            error_type: Some(app_error.error_type().to_string()),
            code: error_code.to_string(),
            recoverable: retriable,
            suggested_action: app_error.suggested_action().map(String::from),
        }
    };

    let mut response = (status, Json(error_response)).into_response();
    response
        .extensions_mut()
        .insert(crate::middleware::wide_event::WideEventExtension(event));
    response
}

// Convert domain errors to HttpAppError (avoids orphan rule: we impl for local HttpAppError)

impl From<StorageError> for HttpAppError {
    fn from(err: StorageError) -> Self {
        let app = match err {
            StorageError::NotFound(msg) => AppError::NotFound(msg),
            StorageError::UploadFailed(msg) => AppError::S3(msg),
            StorageError::DownloadFailed(msg) => AppError::S3(msg),
            StorageError::DeleteFailed(msg) => AppError::S3(msg),
            StorageError::InvalidKey(msg) => AppError::InvalidInput(msg),
            StorageError::BackendError(msg) => AppError::S3(msg),
            StorageError::IoError(err) => AppError::Internal(format!("IO error: {}", err)),
            StorageError::ConfigError(msg) => AppError::Internal(msg),
        };
        HttpAppError(app)
    }
}

impl From<ValidationError> for HttpAppError {
    fn from(err: ValidationError) -> Self {
        let app = match err {
            ValidationError::FileTooLarge { size, max } => {
                AppError::PayloadTooLarge(format!("{} bytes exceeds max {} bytes", size, max))
            }
            ValidationError::InvalidExtension { extension, allowed } => AppError::InvalidInput(
                format!("Invalid extension '{}', allowed: {:?}", extension, allowed),
            ),
            ValidationError::InvalidContentType {
                content_type,
                allowed,
            } => AppError::InvalidInput(format!(
                "Invalid content type '{}', allowed: {:?}",
                content_type, allowed
            )),
            ValidationError::InvalidFilename(msg) => AppError::InvalidInput(msg),
            ValidationError::MissingExtension(filename) => {
                AppError::InvalidInput(format!("Missing file extension (filename: {})", filename))
            }
            ValidationError::EmptyFile => AppError::InvalidInput("File is empty".to_string()),
        };
        HttpAppError(app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mindia_processing::validator::ValidationError;
    use mindia_storage::StorageError;

    #[test]
    fn test_from_storage_error_not_found() {
        let storage_err = StorageError::NotFound("File not found".to_string());
        let HttpAppError(app_err) = storage_err.into();
        match app_err {
            AppError::NotFound(msg) => assert_eq!(msg, "File not found"),
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[test]
    fn test_from_storage_error_upload_failed() {
        let storage_err = StorageError::UploadFailed("Upload failed".to_string());
        let HttpAppError(app_err) = storage_err.into();
        match app_err {
            AppError::S3(msg) => assert_eq!(msg, "Upload failed"),
            _ => panic!("Expected S3 variant"),
        }
    }

    #[test]
    fn test_from_storage_error_invalid_key() {
        let storage_err = StorageError::InvalidKey("Invalid key".to_string());
        let HttpAppError(app_err) = storage_err.into();
        match app_err {
            AppError::InvalidInput(msg) => assert_eq!(msg, "Invalid key"),
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_from_storage_error_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "IO error");
        let storage_err = StorageError::IoError(io_err);
        let HttpAppError(app_err) = storage_err.into();
        match app_err {
            AppError::Internal(msg) => assert!(msg.contains("IO error")),
            _ => panic!("Expected Internal variant"),
        }
    }

    #[test]
    fn test_from_validation_error_file_too_large() {
        let validation_err = ValidationError::FileTooLarge {
            size: 1000,
            max: 500,
        };
        let HttpAppError(app_err) = validation_err.into();
        match app_err {
            AppError::PayloadTooLarge(msg) => {
                assert!(msg.contains("1000"));
                assert!(msg.contains("500"));
            }
            _ => panic!("Expected PayloadTooLarge variant"),
        }
    }

    #[test]
    fn test_from_validation_error_invalid_extension() {
        let validation_err = ValidationError::InvalidExtension {
            extension: "exe".to_string(),
            allowed: vec!["jpg".to_string(), "png".to_string()],
        };
        let HttpAppError(app_err) = validation_err.into();
        match app_err {
            AppError::InvalidInput(msg) => {
                assert!(msg.contains("exe"));
                assert!(msg.contains("jpg"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_from_validation_error_empty_file() {
        let validation_err = ValidationError::EmptyFile;
        let HttpAppError(app_err) = validation_err.into();
        match app_err {
            AppError::InvalidInput(msg) => assert_eq!(msg, "File is empty"),
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    /// Verifies the public error response contract: serialized ErrorResponse has "error",
    /// "code", "recoverable", and optionally "details" / "error_type" / "suggested_action".
    #[test]
    fn test_error_response_shape() {
        let response = ErrorResponse {
            error: "Not found".to_string(),
            details: Some("Resource not found".to_string()),
            error_type: Some("NotFound".to_string()),
            code: "not_found".to_string(),
            recoverable: false,
            suggested_action: None,
        };
        let json = serde_json::to_value(&response).expect("serialize");
        assert!(json.get("error").and_then(|v| v.as_str()).is_some());
        assert!(json.get("code").and_then(|v| v.as_str()).is_some());
        assert!(json.get("recoverable").and_then(|v| v.as_bool()).is_some());
        assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("not_found"));
        assert!(json.is_object());
    }
}
