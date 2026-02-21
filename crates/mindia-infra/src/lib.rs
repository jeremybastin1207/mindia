//! Mindia Infrastructure Library
//!
//! This crate provides shared infrastructure components used across all Mindia services.
//!
//! **Architectural boundary (intended split):**
//! - **Pure infrastructure** (middleware, telemetry, rate limiting, capacity): cross-cutting
//!   concerns with no domain logic; belong in this crate.
//! - **Business services** (webhook, analytics, cleanup, archive): domain-oriented services
//!   that coordinate storage/DB; could be moved to `mindia-services` in a future refactor
//!   so that this crate stays focused on HTTP/observability/resource gates only.
//!
//! Current modules:
//! - Middleware (request ID, security headers)
//! - Telemetry initialization (OpenTelemetry)
//! - Rate limiting
//! - Capacity checking
//!
//! Business services (webhook, analytics, cleanup, archive) are in mindia-services.

#[cfg(feature = "middleware")]
pub mod middleware;

#[cfg(feature = "observability-basic")]
pub mod telemetry;

#[cfg(feature = "rate-limit")]
pub mod rate_limit;

#[cfg(feature = "capacity")]
pub mod capacity;

// Re-export commonly used types
#[cfg(feature = "middleware")]
pub use middleware::{
    get_request_id, request_id_middleware, security_headers_middleware, RequestId,
};

#[cfg(feature = "observability-basic")]
pub use telemetry::{init_telemetry, shutdown_telemetry};

#[cfg(feature = "rate-limit")]
pub use rate_limit::RateLimiter;

#[cfg(feature = "capacity")]
pub use capacity::CapacityChecker;
