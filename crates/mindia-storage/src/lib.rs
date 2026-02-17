//! Mindia Storage Library
//!
//! This crate provides storage abstraction and implementations for Mindia.
//! It includes the Storage trait and implementations for S3 and local filesystem.
//!
//! # Storage key format
//!
//! Storage keys are tenant-scoped. All backends use the same key layout for consistency:
//!
//! - **Default tenant**: `media/{filename}`
//! - **Other tenants**: `media/{tenant_id}/{filename}`
//!
//! Keys must not contain `..` or a leading `/`. Key generation is centralized in the
//! `keys` module so all backends stay consistent.

pub mod factory;
pub(crate) mod keys;
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
