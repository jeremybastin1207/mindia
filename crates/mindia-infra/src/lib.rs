//! Mindia Infrastructure Library
//!
//! This crate provides shared infrastructure components used across all Mindia services:
//! - Middleware (request ID, security headers)
//! - Telemetry initialization (OpenTelemetry)
//! - Error handling
//! - Webhook delivery
//! - Analytics collection
//! - Rate limiting
//! - Cleanup services
//! - Capacity checking
//! - Archive creation

#[cfg(feature = "middleware")]
pub mod middleware;

#[cfg(feature = "observability-basic")]
pub mod telemetry;

pub mod error;

#[cfg(feature = "webhook")]
pub mod webhook;

#[cfg(feature = "analytics")]
pub mod analytics;

#[cfg(feature = "rate-limit")]
pub mod rate_limit;

#[cfg(feature = "cleanup")]
pub mod cleanup;

#[cfg(feature = "capacity")]
pub mod capacity;

#[cfg(feature = "archive")]
pub mod archive;

// Re-export commonly used types
#[cfg(feature = "middleware")]
pub use middleware::{
    get_request_id, request_id_middleware, security_headers_middleware, RequestId,
};

#[cfg(feature = "observability-basic")]
pub use telemetry::{init_telemetry, shutdown_telemetry};

pub use error::ErrorResponse;

#[cfg(feature = "webhook")]
pub use webhook::{
    WebhookRetryService, WebhookRetryServiceConfig, WebhookService, WebhookServiceConfig,
};

#[cfg(feature = "analytics")]
pub use analytics::{start_storage_metrics_refresh, AnalyticsService};

#[cfg(feature = "rate-limit")]
pub use rate_limit::RateLimiter;

#[cfg(feature = "cleanup")]
pub use cleanup::CleanupService;

#[cfg(feature = "capacity")]
pub use capacity::CapacityChecker;

#[cfg(feature = "archive")]
pub use archive::{create_archive, ArchiveFormat};
