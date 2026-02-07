//! Mindia Storage Library
//!
//! This crate provides storage abstraction and implementations for Mindia.
//! It includes the Storage trait and implementations for S3 and local filesystem.

pub mod factory;
#[cfg(feature = "storage-local")]
pub mod local;
#[cfg(feature = "storage-s3")]
pub mod s3;
pub mod traits;

// Re-export commonly used types
pub use factory::create_storage;
#[cfg(feature = "storage-local")]
pub use local::LocalStorage;
pub use mindia_core::StorageBackend;
#[cfg(feature = "storage-s3")]
pub use s3::S3Storage;
pub use traits::{Storage, StorageError, StorageResult};
