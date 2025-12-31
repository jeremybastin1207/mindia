//! OpenTelemetry telemetry module
//!
//! This module provides OpenTelemetry initialization and helper utilities
//! for metrics, traces, and logs.

pub mod init;
pub mod metrics;
pub mod pool_metrics;
pub mod wide_event;

pub use init::{init_telemetry, shutdown_telemetry};
