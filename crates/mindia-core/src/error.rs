//! Error types module
//!
//! This module provides the core error types used throughout the Mindia application.
//! All errors are unified under the `AppError` enum which can represent database,
//! storage, validation, and other domain-specific errors.
//!
//! The `Database` variant and `From<sqlx::Error>` are gated behind the `sqlx` feature.
//! With `default-features = false`, build without the `sqlx` feature; then `AppError` has no database variant and you must use other error types for DB errors.

use std::io;

#[cfg(feature = "sqlx")]
use sqlx::Error as SqlxError;

/// Log level for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug level - for expected errors like validation failures
    Debug,
    /// Warning level - for recoverable issues like resource limits
    Warn,
    /// Error level - for unexpected failures
    Error,
}

/// Metadata for error responses - defines how an error should be presented
/// This trait allows errors to self-describe their HTTP response characteristics
pub trait ErrorMetadata {
    /// HTTP status code to return
    fn http_status_code(&self) -> u16;

    /// Machine-readable error code (e.g., "DATABASE_ERROR")
    fn error_code(&self) -> &'static str;

    /// Whether this error is recoverable (can be retried)
    fn is_recoverable(&self) -> bool;

    /// Suggested action for the client
    fn suggested_action(&self) -> Option<&'static str>;

    /// Client-facing message (may differ from internal error message)
    fn client_message(&self) -> String;

    /// Whether details should be hidden in production
    fn is_sensitive(&self) -> bool;

    /// Log level for this error
    fn log_level(&self) -> LogLevel;
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[cfg(feature = "sqlx")]
    #[error("Database error: {0}")]
    Database(#[source] SqlxError),

    #[cfg(not(feature = "sqlx"))]
    #[error("Database error: {0}")]
    Database(String),

    #[error("S3 error: {0}")]
    S3(String),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("File too large: {0}")]
    PayloadTooLarge(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Internal error with source")]
    InternalWithSource {
        message: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Insufficient disk space: {available} bytes available, {required} bytes required")]
    InsufficientDiskSpace { available: u64, required: u64 },

    #[error("Insufficient memory: {available} bytes available, {required} bytes required")]
    InsufficientMemory { available: u64, required: u64 },

    #[error("High CPU usage: {usage_percent}% exceeds threshold of {threshold}%")]
    HighCpuUsage { usage_percent: f64, threshold: f64 },

    #[error("High memory usage: {usage_percent}% exceeds threshold of {threshold}%")]
    HighMemoryUsage { usage_percent: f64, threshold: f64 },

    #[error("Resource exceeded during operation: {resource} usage {usage}% exceeds threshold {threshold}%")]
    ResourceExceededDuringOperation {
        resource: String,
        usage: f64,
        threshold: f64,
    },

    #[error("Invalid metadata key: {0}")]
    InvalidMetadataKey(String),

    #[error("Invalid metadata value: {0}")]
    InvalidMetadataValue(String),

    #[error("Metadata key limit exceeded: {0}")]
    MetadataKeyLimitExceeded(String),

    #[error("Metadata key not found: {0}")]
    MetadataKeyNotFound(String),

    #[error("Invalid metadata filter: {0}")]
    InvalidMetadataFilter(String),

    #[error("Metadata filter limit exceeded: {0}")]
    MetadataFilterLimitExceeded(String),

    #[error("Usage limit exceeded: {resource} usage {used}/{limit}")]
    UsageLimitExceeded {
        resource: String,
        used: i64,
        limit: i64,
    },

    #[error("Subscription required: {0}")]
    SubscriptionRequired(String),

    #[error("Organization not found: {0}")]
    OrganizationNotFound(String),

    #[error("Invalid OAuth token: {0}")]
    InvalidOAuthToken(String),

    #[error("Stripe error: {0}")]
    StripeError(String),

    #[error("Media conversion error: {0}")]
    MediaConversionError(String),
}

// Error conversion implementations following Rust best practices
#[cfg(feature = "sqlx")]
impl From<SqlxError> for AppError {
    fn from(err: SqlxError) -> Self {
        AppError::Database(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::InternalWithSource {
            message: err.to_string(),
            source: err,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Internal(format!("IO error: {}", err))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InvalidInput(format!("JSON parsing error: {}", err))
    }
}

impl From<uuid::Error> for AppError {
    fn from(err: uuid::Error) -> Self {
        AppError::InvalidInput(format!("UUID parsing error: {}", err))
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::InvalidInput(format!("Validation error: {}", err))
    }
}

/// Static metadata for each variant: (http_status, error_code, recoverable, suggested_action, sensitive, log_level).
/// Reduces duplication in ErrorMetadata impl; client_message stays per-variant for dynamic content.
fn app_error_static_metadata(
    err: &AppError,
) -> (
    u16,
    &'static str,
    bool,
    Option<&'static str>,
    bool,
    LogLevel,
) {
    match err {
        AppError::Database(_) => (
            500,
            "DATABASE_ERROR",
            true,
            Some("Retry after a short delay"),
            true,
            LogLevel::Error,
        ),
        AppError::S3(_) => (
            500,
            "STORAGE_ERROR",
            true,
            Some("Retry after a short delay"),
            true,
            LogLevel::Error,
        ),
        AppError::ImageProcessing(_) => (
            400,
            "IMAGE_PROCESSING_ERROR",
            false,
            Some("Check image format and try a different file"),
            false,
            LogLevel::Warn,
        ),
        AppError::MediaConversionError(_) => (
            500,
            "MEDIA_CONVERSION_ERROR",
            false,
            Some("Contact support if this error persists"),
            true,
            LogLevel::Error,
        ),
        AppError::InvalidInput(_) => (
            400,
            "INVALID_INPUT",
            false,
            Some("Check request parameters and try again"),
            false,
            LogLevel::Debug,
        ),
        AppError::BadRequest(_) => (
            400,
            "BAD_REQUEST",
            false,
            Some("Check request format and parameters"),
            false,
            LogLevel::Debug,
        ),
        AppError::NotFound(_) => (
            404,
            "NOT_FOUND",
            false,
            Some("Verify the resource ID exists"),
            false,
            LogLevel::Debug,
        ),
        AppError::MetadataKeyNotFound(_) => (
            404,
            "METADATA_KEY_NOT_FOUND",
            false,
            Some("Verify the metadata key exists"),
            false,
            LogLevel::Debug,
        ),
        AppError::OrganizationNotFound(_) => (
            404,
            "ORGANIZATION_NOT_FOUND",
            false,
            Some("Verify the organization ID exists"),
            false,
            LogLevel::Debug,
        ),
        AppError::PayloadTooLarge(_) => (
            413,
            "PAYLOAD_TOO_LARGE",
            false,
            Some("Reduce file size or use chunked upload"),
            false,
            LogLevel::Debug,
        ),
        AppError::Internal(_) => (
            500,
            "INTERNAL_ERROR",
            true,
            Some("Retry after a short delay"),
            true,
            LogLevel::Error,
        ),
        AppError::InternalWithSource { .. } => (
            500,
            "INTERNAL_ERROR",
            true,
            Some("Retry after a short delay"),
            true,
            LogLevel::Error,
        ),
        AppError::Unauthorized(_) => (
            401,
            "UNAUTHORIZED",
            false,
            Some("Check API key or authentication token"),
            false,
            LogLevel::Debug,
        ),
        AppError::InvalidOAuthToken(_) => (
            401,
            "INVALID_OAUTH_TOKEN",
            false,
            Some("Refresh OAuth token or re-authenticate"),
            false,
            LogLevel::Debug,
        ),
        AppError::InsufficientDiskSpace { .. } => (
            507,
            "INSUFFICIENT_DISK_SPACE",
            true,
            Some("Retry after cleanup or wait for capacity"),
            false,
            LogLevel::Warn,
        ),
        AppError::InsufficientMemory { .. } => (
            507,
            "INSUFFICIENT_MEMORY",
            true,
            Some("Retry after a short delay"),
            false,
            LogLevel::Warn,
        ),
        AppError::HighCpuUsage { .. } => (
            503,
            "HIGH_CPU_USAGE",
            true,
            Some("Wait 30-60 seconds and retry"),
            false,
            LogLevel::Warn,
        ),
        AppError::HighMemoryUsage { .. } => (
            503,
            "HIGH_MEMORY_USAGE",
            true,
            Some("Wait 30-60 seconds and retry"),
            false,
            LogLevel::Warn,
        ),
        AppError::ResourceExceededDuringOperation { .. } => (
            503,
            "RESOURCE_EXCEEDED",
            true,
            Some("Wait 30-60 seconds and retry"),
            false,
            LogLevel::Warn,
        ),
        AppError::InvalidMetadataKey(_) => (
            400,
            "INVALID_METADATA_KEY",
            false,
            Some("Check metadata key format and constraints"),
            false,
            LogLevel::Debug,
        ),
        AppError::InvalidMetadataValue(_) => (
            400,
            "INVALID_METADATA_VALUE",
            false,
            Some("Check metadata value format and constraints"),
            false,
            LogLevel::Debug,
        ),
        AppError::MetadataKeyLimitExceeded(_) => (
            400,
            "METADATA_KEY_LIMIT_EXCEEDED",
            false,
            Some("Remove some metadata keys or upgrade plan"),
            false,
            LogLevel::Debug,
        ),
        AppError::InvalidMetadataFilter(_) => (
            400,
            "INVALID_METADATA_FILTER",
            false,
            Some("Check metadata filter syntax"),
            false,
            LogLevel::Debug,
        ),
        AppError::MetadataFilterLimitExceeded(_) => (
            400,
            "METADATA_FILTER_LIMIT_EXCEEDED",
            false,
            Some("Reduce number of filter conditions"),
            false,
            LogLevel::Debug,
        ),
        AppError::UsageLimitExceeded { .. } => (
            402,
            "USAGE_LIMIT_EXCEEDED",
            false,
            Some("Upgrade plan or wait for limit reset"),
            false,
            LogLevel::Warn,
        ),
        AppError::SubscriptionRequired(_) => (
            402,
            "SUBSCRIPTION_REQUIRED",
            false,
            Some("Subscribe to a plan to access this feature"),
            false,
            LogLevel::Debug,
        ),
        AppError::StripeError(_) => (
            500,
            "STRIPE_ERROR",
            true,
            Some("Retry payment after a short delay"),
            true,
            LogLevel::Error,
        ),
    }
}

impl AppError {
    /// Get the error type name for detailed error responses
    pub fn error_type(&self) -> &str {
        match self {
            AppError::Database(_) => "Database",
            AppError::S3(_) => "S3",
            AppError::ImageProcessing(_) => "ImageProcessing",
            AppError::InvalidInput(_) => "InvalidInput",
            AppError::BadRequest(_) => "BadRequest",
            AppError::NotFound(_) => "NotFound",
            AppError::PayloadTooLarge(_) => "PayloadTooLarge",
            AppError::Internal(_) => "Internal",
            AppError::InternalWithSource { .. } => "Internal",
            AppError::Unauthorized(_) => "Unauthorized",
            AppError::InsufficientDiskSpace { .. } => "InsufficientDiskSpace",
            AppError::InsufficientMemory { .. } => "InsufficientMemory",
            AppError::HighCpuUsage { .. } => "HighCpuUsage",
            AppError::HighMemoryUsage { .. } => "HighMemoryUsage",
            AppError::ResourceExceededDuringOperation { .. } => "ResourceExceededDuringOperation",
            AppError::InvalidMetadataKey(_) => "InvalidMetadataKey",
            AppError::InvalidMetadataValue(_) => "InvalidMetadataValue",
            AppError::MetadataKeyLimitExceeded(_) => "MetadataKeyLimitExceeded",
            AppError::MetadataKeyNotFound(_) => "MetadataKeyNotFound",
            AppError::InvalidMetadataFilter(_) => "InvalidMetadataFilter",
            AppError::MetadataFilterLimitExceeded(_) => "MetadataFilterLimitExceeded",
            AppError::UsageLimitExceeded { .. } => "UsageLimitExceeded",
            AppError::SubscriptionRequired(_) => "SubscriptionRequired",
            AppError::OrganizationNotFound(_) => "OrganizationNotFound",
            AppError::InvalidOAuthToken(_) => "InvalidOAuthToken",
            AppError::StripeError(_) => "StripeError",
            AppError::MediaConversionError(_) => "MediaConversionError",
        }
    }

    /// Get detailed error information including error chain
    pub fn detailed_message(&self) -> String {
        use std::error::Error;

        let mut details = self.to_string();

        // Add source error chain
        let mut source = self.source();
        let mut depth = 0;
        while let Some(err) = source {
            depth += 1;
            if depth > 5 {
                details.push_str("\n  ... (truncated)");
                break;
            }
            details.push_str(&format!("\n  Caused by: {}", err));
            source = err.source();
        }

        details
    }
}

impl ErrorMetadata for AppError {
    fn http_status_code(&self) -> u16 {
        app_error_static_metadata(self).0
    }

    fn error_code(&self) -> &'static str {
        app_error_static_metadata(self).1
    }

    fn is_recoverable(&self) -> bool {
        app_error_static_metadata(self).2
    }

    fn suggested_action(&self) -> Option<&'static str> {
        app_error_static_metadata(self).3
    }

    fn is_sensitive(&self) -> bool {
        app_error_static_metadata(self).4
    }

    fn log_level(&self) -> LogLevel {
        app_error_static_metadata(self).5
    }

    fn client_message(&self) -> String {
        match self {
            AppError::Database(_) => "Failed to access database".to_string(),
            AppError::S3(_) => "Failed to access storage".to_string(),
            AppError::ImageProcessing(ref msg) => msg.clone(),
            AppError::MediaConversionError(_) => "Failed to process media data".to_string(),
            AppError::InvalidInput(ref msg) => msg.clone(),
            AppError::BadRequest(ref msg) => msg.clone(),
            AppError::NotFound(ref msg) => msg.clone(),
            AppError::MetadataKeyNotFound(ref msg) => msg.clone(),
            AppError::OrganizationNotFound(ref msg) => msg.clone(),
            AppError::PayloadTooLarge(ref msg) => msg.clone(),
            AppError::Internal(_) => "Internal server error".to_string(),
            AppError::InternalWithSource { .. } => "Internal server error".to_string(),
            AppError::Unauthorized(ref msg) => msg.clone(),
            AppError::InvalidOAuthToken(ref msg) => msg.clone(),
            AppError::InsufficientDiskSpace {
                available,
                required,
            } => {
                format!(
                    "Insufficient disk space: {} bytes available, {} bytes required",
                    available, required
                )
            }
            AppError::InsufficientMemory {
                available,
                required,
            } => {
                format!(
                    "Insufficient memory: {} bytes available, {} bytes required",
                    available, required
                )
            }
            AppError::HighCpuUsage {
                usage_percent,
                threshold,
            } => {
                format!(
                    "High CPU usage: {}% exceeds threshold of {}%",
                    usage_percent, threshold
                )
            }
            AppError::HighMemoryUsage {
                usage_percent,
                threshold,
            } => {
                format!(
                    "High memory usage: {}% exceeds threshold of {}%",
                    usage_percent, threshold
                )
            }
            AppError::ResourceExceededDuringOperation {
                resource,
                usage,
                threshold,
            } => {
                format!(
                    "Resource {} exceeded: {}% usage exceeds threshold {}%",
                    resource, usage, threshold
                )
            }
            AppError::InvalidMetadataKey(ref msg) => msg.clone(),
            AppError::InvalidMetadataValue(ref msg) => msg.clone(),
            AppError::MetadataKeyLimitExceeded(ref msg) => msg.clone(),
            AppError::InvalidMetadataFilter(ref msg) => msg.clone(),
            AppError::MetadataFilterLimitExceeded(ref msg) => msg.clone(),
            AppError::UsageLimitExceeded {
                resource,
                used,
                limit,
            } => {
                format!(
                    "Usage limit exceeded: {} usage {}/{}",
                    resource, used, limit
                )
            }
            AppError::SubscriptionRequired(ref msg) => msg.clone(),
            AppError::StripeError(_) => "Payment processing error".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_metadata_database() {
        #[cfg(feature = "sqlx")]
        let err = AppError::from(sqlx::Error::PoolClosed);
        #[cfg(not(feature = "sqlx"))]
        let err = AppError::Database("pool closed".to_string());
        assert_eq!(err.http_status_code(), 500);
        assert_eq!(err.error_code(), "DATABASE_ERROR");
        assert!(err.is_recoverable());
        assert_eq!(err.client_message(), "Failed to access database");
        assert!(err.is_sensitive());
        assert_eq!(err.log_level(), LogLevel::Error);
    }

    #[test]
    fn test_error_metadata_not_found() {
        let err = AppError::NotFound("Resource not found".to_string());
        assert_eq!(err.http_status_code(), 404);
        assert_eq!(err.error_code(), "NOT_FOUND");
        assert!(!err.is_recoverable());
        assert_eq!(err.client_message(), "Resource not found");
        assert!(!err.is_sensitive());
        assert_eq!(err.log_level(), LogLevel::Debug);
    }

    #[test]
    fn test_error_metadata_usage_limit_exceeded() {
        let err = AppError::UsageLimitExceeded {
            resource: "storage".to_string(),
            used: 100,
            limit: 50,
        };
        assert_eq!(err.http_status_code(), 402);
        assert_eq!(err.error_code(), "USAGE_LIMIT_EXCEEDED");
        assert!(!err.is_recoverable());
        assert!(err.client_message().contains("storage"));
        assert!(err.client_message().contains("100"));
        assert!(err.client_message().contains("50"));
        assert!(!err.is_sensitive());
        assert_eq!(err.log_level(), LogLevel::Warn);
    }

    #[test]
    fn test_error_metadata_insufficient_disk_space() {
        let err = AppError::InsufficientDiskSpace {
            available: 1000,
            required: 2000,
        };
        assert_eq!(err.http_status_code(), 507);
        assert_eq!(err.error_code(), "INSUFFICIENT_DISK_SPACE");
        assert!(err.is_recoverable());
        assert!(err.client_message().contains("1000"));
        assert!(err.client_message().contains("2000"));
        assert!(!err.is_sensitive());
        assert_eq!(err.log_level(), LogLevel::Warn);
    }

    #[test]
    fn test_error_metadata_suggested_actions() {
        #[cfg(feature = "sqlx")]
        let err1 = AppError::Database(sqlx::Error::PoolClosed);
        #[cfg(not(feature = "sqlx"))]
        let err1 = AppError::Database("test".to_string());
        assert_eq!(err1.suggested_action(), Some("Retry after a short delay"));

        let err2 = AppError::NotFound("test".to_string());
        assert_eq!(
            err2.suggested_action(),
            Some("Verify the resource ID exists")
        );

        let err3 = AppError::InvalidInput("test".to_string());
        assert_eq!(
            err3.suggested_action(),
            Some("Check request parameters and try again")
        );
    }
}
