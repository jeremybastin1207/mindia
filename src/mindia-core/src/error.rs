//! Error types module
//!
//! This module provides the core error types used throughout the Mindia application.
//! All errors are unified under the `AppError` enum which can represent database,
//! storage, validation, and other domain-specific errors.

use std::io;

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
    #[error("Database error: {0}")]
    Database(#[source] SqlxError),

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

// Ensure Send + Sync bounds for async contexts
// thiserror automatically implements Send + Sync for AppError when all variants are Send + Sync
// All our variants use Send + Sync types, so this is satisfied

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
        match self {
            AppError::Database(_) => 500,
            AppError::S3(_) => 500,
            AppError::ImageProcessing(_) => 400,
            AppError::MediaConversionError(_) => 500,
            AppError::InvalidInput(_) => 400,
            AppError::BadRequest(_) => 400,
            AppError::NotFound(_) => 404,
            AppError::MetadataKeyNotFound(_) => 404,
            AppError::OrganizationNotFound(_) => 404,
            AppError::PayloadTooLarge(_) => 413,
            AppError::Internal(_) => 500,
            AppError::InternalWithSource { .. } => 500,
            AppError::Unauthorized(_) => 401,
            AppError::InvalidOAuthToken(_) => 401,
            AppError::InsufficientDiskSpace { .. } => 507,
            AppError::InsufficientMemory { .. } => 507,
            AppError::HighCpuUsage { .. } => 503,
            AppError::HighMemoryUsage { .. } => 503,
            AppError::ResourceExceededDuringOperation { .. } => 503,
            AppError::InvalidMetadataKey(_) => 400,
            AppError::InvalidMetadataValue(_) => 400,
            AppError::MetadataKeyLimitExceeded(_) => 400,
            AppError::InvalidMetadataFilter(_) => 400,
            AppError::MetadataFilterLimitExceeded(_) => 400,
            AppError::UsageLimitExceeded { .. } => 402,
            AppError::SubscriptionRequired(_) => 402,
            AppError::StripeError(_) => 500,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::S3(_) => "STORAGE_ERROR",
            AppError::ImageProcessing(_) => "IMAGE_PROCESSING_ERROR",
            AppError::MediaConversionError(_) => "MEDIA_CONVERSION_ERROR",
            AppError::InvalidInput(_) => "INVALID_INPUT",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::MetadataKeyNotFound(_) => "METADATA_KEY_NOT_FOUND",
            AppError::OrganizationNotFound(_) => "ORGANIZATION_NOT_FOUND",
            AppError::PayloadTooLarge(_) => "PAYLOAD_TOO_LARGE",
            AppError::Internal(_) => "INTERNAL_ERROR",
            AppError::InternalWithSource { .. } => "INTERNAL_ERROR",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::InvalidOAuthToken(_) => "INVALID_OAUTH_TOKEN",
            AppError::InsufficientDiskSpace { .. } => "INSUFFICIENT_DISK_SPACE",
            AppError::InsufficientMemory { .. } => "INSUFFICIENT_MEMORY",
            AppError::HighCpuUsage { .. } => "HIGH_CPU_USAGE",
            AppError::HighMemoryUsage { .. } => "HIGH_MEMORY_USAGE",
            AppError::ResourceExceededDuringOperation { .. } => "RESOURCE_EXCEEDED",
            AppError::InvalidMetadataKey(_) => "INVALID_METADATA_KEY",
            AppError::InvalidMetadataValue(_) => "INVALID_METADATA_VALUE",
            AppError::MetadataKeyLimitExceeded(_) => "METADATA_KEY_LIMIT_EXCEEDED",
            AppError::InvalidMetadataFilter(_) => "INVALID_METADATA_FILTER",
            AppError::MetadataFilterLimitExceeded(_) => "METADATA_FILTER_LIMIT_EXCEEDED",
            AppError::UsageLimitExceeded { .. } => "USAGE_LIMIT_EXCEEDED",
            AppError::SubscriptionRequired(_) => "SUBSCRIPTION_REQUIRED",
            AppError::StripeError(_) => "STRIPE_ERROR",
        }
    }

    fn is_recoverable(&self) -> bool {
        match self {
            AppError::Database(_) => true,
            AppError::S3(_) => true,
            AppError::ImageProcessing(_) => false,
            AppError::MediaConversionError(_) => false,
            AppError::InvalidInput(_) => false,
            AppError::BadRequest(_) => false,
            AppError::NotFound(_) => false,
            AppError::MetadataKeyNotFound(_) => false,
            AppError::OrganizationNotFound(_) => false,
            AppError::PayloadTooLarge(_) => false,
            AppError::Internal(_) => true,
            AppError::InternalWithSource { .. } => true,
            AppError::Unauthorized(_) => false,
            AppError::InvalidOAuthToken(_) => false,
            AppError::InsufficientDiskSpace { .. } => true,
            AppError::InsufficientMemory { .. } => true,
            AppError::HighCpuUsage { .. } => true,
            AppError::HighMemoryUsage { .. } => true,
            AppError::ResourceExceededDuringOperation { .. } => true,
            AppError::InvalidMetadataKey(_) => false,
            AppError::InvalidMetadataValue(_) => false,
            AppError::MetadataKeyLimitExceeded(_) => false,
            AppError::InvalidMetadataFilter(_) => false,
            AppError::MetadataFilterLimitExceeded(_) => false,
            AppError::UsageLimitExceeded { .. } => false,
            AppError::SubscriptionRequired(_) => false,
            AppError::StripeError(_) => true,
        }
    }

    fn suggested_action(&self) -> Option<&'static str> {
        match self {
            AppError::Database(_) => Some("Retry after a short delay"),
            AppError::S3(_) => Some("Retry after a short delay"),
            AppError::ImageProcessing(_) => Some("Check image format and try a different file"),
            AppError::MediaConversionError(_) => Some("Contact support if this error persists"),
            AppError::InvalidInput(_) => Some("Check request parameters and try again"),
            AppError::BadRequest(_) => Some("Check request format and parameters"),
            AppError::NotFound(_) => Some("Verify the resource ID exists"),
            AppError::MetadataKeyNotFound(_) => Some("Verify the metadata key exists"),
            AppError::OrganizationNotFound(_) => Some("Verify the organization ID exists"),
            AppError::PayloadTooLarge(_) => Some("Reduce file size or use chunked upload"),
            AppError::Internal(_) => Some("Retry after a short delay"),
            AppError::InternalWithSource { .. } => Some("Retry after a short delay"),
            AppError::Unauthorized(_) => Some("Check API key or authentication token"),
            AppError::InvalidOAuthToken(_) => Some("Refresh OAuth token or re-authenticate"),
            AppError::InsufficientDiskSpace { .. } => {
                Some("Retry after cleanup or wait for capacity")
            }
            AppError::InsufficientMemory { .. } => Some("Retry after a short delay"),
            AppError::HighCpuUsage { .. } => Some("Wait 30-60 seconds and retry"),
            AppError::HighMemoryUsage { .. } => Some("Wait 30-60 seconds and retry"),
            AppError::ResourceExceededDuringOperation { .. } => {
                Some("Wait 30-60 seconds and retry")
            }
            AppError::InvalidMetadataKey(_) => Some("Check metadata key format and constraints"),
            AppError::InvalidMetadataValue(_) => {
                Some("Check metadata value format and constraints")
            }
            AppError::MetadataKeyLimitExceeded(_) => {
                Some("Remove some metadata keys or upgrade plan")
            }
            AppError::InvalidMetadataFilter(_) => Some("Check metadata filter syntax"),
            AppError::MetadataFilterLimitExceeded(_) => Some("Reduce number of filter conditions"),
            AppError::UsageLimitExceeded { .. } => Some("Upgrade plan or wait for limit reset"),
            AppError::SubscriptionRequired(_) => Some("Subscribe to a plan to access this feature"),
            AppError::StripeError(_) => Some("Retry payment after a short delay"),
        }
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

    fn is_sensitive(&self) -> bool {
        match self {
            AppError::Database(_) => true,
            AppError::S3(_) => true,
            AppError::ImageProcessing(_) => false,
            AppError::MediaConversionError(_) => true,
            AppError::InvalidInput(_) => false,
            AppError::BadRequest(_) => false,
            AppError::NotFound(_) => false,
            AppError::MetadataKeyNotFound(_) => false,
            AppError::OrganizationNotFound(_) => false,
            AppError::PayloadTooLarge(_) => false,
            AppError::Internal(_) => true,
            AppError::InternalWithSource { .. } => true,
            AppError::Unauthorized(_) => false,
            AppError::InvalidOAuthToken(_) => false,
            AppError::InsufficientDiskSpace { .. } => false,
            AppError::InsufficientMemory { .. } => false,
            AppError::HighCpuUsage { .. } => false,
            AppError::HighMemoryUsage { .. } => false,
            AppError::ResourceExceededDuringOperation { .. } => false,
            AppError::InvalidMetadataKey(_) => false,
            AppError::InvalidMetadataValue(_) => false,
            AppError::MetadataKeyLimitExceeded(_) => false,
            AppError::InvalidMetadataFilter(_) => false,
            AppError::MetadataFilterLimitExceeded(_) => false,
            AppError::UsageLimitExceeded { .. } => false,
            AppError::SubscriptionRequired(_) => false,
            AppError::StripeError(_) => true,
        }
    }

    fn log_level(&self) -> LogLevel {
        match self {
            AppError::Database(_) => LogLevel::Error,
            AppError::S3(_) => LogLevel::Error,
            AppError::ImageProcessing(_) => LogLevel::Warn,
            AppError::MediaConversionError(_) => LogLevel::Error,
            AppError::InvalidInput(_) => LogLevel::Debug,
            AppError::BadRequest(_) => LogLevel::Debug,
            AppError::NotFound(_) => LogLevel::Debug,
            AppError::MetadataKeyNotFound(_) => LogLevel::Debug,
            AppError::OrganizationNotFound(_) => LogLevel::Debug,
            AppError::PayloadTooLarge(_) => LogLevel::Debug,
            AppError::Internal(_) => LogLevel::Error,
            AppError::InternalWithSource { .. } => LogLevel::Error,
            AppError::Unauthorized(_) => LogLevel::Debug,
            AppError::InvalidOAuthToken(_) => LogLevel::Debug,
            AppError::InsufficientDiskSpace { .. } => LogLevel::Warn,
            AppError::InsufficientMemory { .. } => LogLevel::Warn,
            AppError::HighCpuUsage { .. } => LogLevel::Warn,
            AppError::HighMemoryUsage { .. } => LogLevel::Warn,
            AppError::ResourceExceededDuringOperation { .. } => LogLevel::Warn,
            AppError::InvalidMetadataKey(_) => LogLevel::Debug,
            AppError::InvalidMetadataValue(_) => LogLevel::Debug,
            AppError::MetadataKeyLimitExceeded(_) => LogLevel::Debug,
            AppError::InvalidMetadataFilter(_) => LogLevel::Debug,
            AppError::MetadataFilterLimitExceeded(_) => LogLevel::Debug,
            AppError::UsageLimitExceeded { .. } => LogLevel::Warn,
            AppError::SubscriptionRequired(_) => LogLevel::Debug,
            AppError::StripeError(_) => LogLevel::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_metadata_database() {
        // Create a database error using From trait
        let sqlx_err = sqlx::Error::PoolClosed;
        let err = AppError::from(sqlx_err);
        match err {
            AppError::Database(_) => {
                assert_eq!(err.http_status_code(), 500);
                assert_eq!(err.error_code(), "DATABASE_ERROR");
                assert!(err.is_recoverable());
                assert_eq!(err.client_message(), "Failed to access database");
                assert!(err.is_sensitive());
                assert_eq!(err.log_level(), LogLevel::Error);
            }
            _ => panic!("Expected Database variant"),
        }
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
        let err1 = AppError::Database(sqlx::Error::PoolClosed);
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
