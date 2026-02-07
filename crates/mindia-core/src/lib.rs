//! Mindia Core Library
//!
//! This crate provides core domain models, error types, configuration, and validation
//! that are shared across all Mindia components.

pub mod capacity_gate;
pub mod config;
pub mod constants;
pub mod encryption;
pub mod error;
pub mod hooks;
pub mod models;
pub mod storage_types;
pub mod task_error;
pub mod transform_url;
pub mod validation;

// Re-export commonly used types
pub use capacity_gate::CapacityGate;
pub use config::{validate_env, BaseConfig, Config, MediaProcessorConfig};
pub use encryption::EncryptionService;
pub use error::{AppError, ErrorMetadata, LogLevel};
pub use storage_types::StorageBackend;
pub use task_error::{TaskError, TaskResultExt};
// Note: Storage, StorageError, StorageResult are now in mindia-storage crate
// Import them directly from mindia-storage instead of mindia-core
pub use hooks::{NoOpUsageReporter, TenantContextInfo, UsageInfo, UsageReporter};
pub use transform_url::{ImageTransformUrlBuilder, ImageTransformUrlParser, ParsedTransformUrl};
