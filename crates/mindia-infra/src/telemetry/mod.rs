//! OpenTelemetry telemetry initialization
//!
//! This module provides OpenTelemetry initialization for traces, metrics, and logs.

#[cfg(feature = "observability-opentelemetry")]
mod init_opentelemetry;

#[cfg(not(feature = "observability-opentelemetry"))]
mod init_basic;

#[cfg(feature = "observability-opentelemetry")]
pub use init_opentelemetry::{init_telemetry, shutdown_telemetry};

#[cfg(not(feature = "observability-opentelemetry"))]
pub use init_basic::{init_telemetry, shutdown_telemetry};
