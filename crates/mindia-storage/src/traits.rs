//! Storage abstraction trait
//!
//! This module defines the Storage trait that all storage backends must implement.

use crate::StorageBackend;
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::time::Duration;
use thiserror::Error;
use tokio::io::AsyncRead;
use uuid::Uuid;

/// Storage operation errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Delete failed: {0}")]
    DeleteFailed(String),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Invalid storage key: {0}")]
    InvalidKey(String),

    #[error("Storage backend error: {0}")]
    BackendError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage abstraction trait
///
/// All storage backends (S3, local filesystem) must implement this trait.
/// This allows the media repository to work with any storage backend without
/// coupling to specific implementation details.
///
/// **Key format:** Keys are tenant-scoped: `media/{filename}` for the default tenant,
/// or `media/{tenant_id}/{filename}` otherwise. See the crate root documentation.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Upload a file and return (storage_key, storage_url)
    ///
    /// The storage_key is an internal identifier used to reference the file
    /// The storage_url is the publicly accessible URL to the file
    async fn upload(
        &self,
        tenant_id: Uuid,
        filename: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> StorageResult<(String, String)>;

    /// Download a file by its storage key
    async fn download(&self, storage_key: &str) -> StorageResult<Vec<u8>>;

    /// Upload data to a specific storage key (for processing workflows e.g. HLS segments).
    /// Returns the public URL for the uploaded file.
    async fn upload_with_key(
        &self,
        storage_key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> StorageResult<String>;

    /// Delete a file by its storage key
    async fn delete(&self, storage_key: &str) -> StorageResult<()>;

    /// Generate a presigned/temporary URL for direct access (GET)
    ///
    /// This is useful for giving clients temporary access to files
    /// without going through the application server
    async fn get_presigned_url(
        &self,
        storage_key: &str,
        expires_in: Duration,
    ) -> StorageResult<String>;

    /// Generate a presigned PUT URL for direct uploads.
    ///
    /// Clients can upload with HTTP PUT to the returned URL. Only supported by S3 backends;
    /// other backends return a `ConfigError`.
    async fn presigned_put_url(
        &self,
        storage_key: &str,
        _content_type: &str,
        expires_in: Duration,
    ) -> StorageResult<String>;

    /// Check if a file exists
    async fn exists(&self, storage_key: &str) -> StorageResult<bool>;

    /// Get the size in bytes of an object, if it exists.
    async fn content_length(&self, storage_key: &str) -> StorageResult<u64>;

    /// Copy a file from one key to another
    ///
    /// This is useful for processing workflows (e.g., copying original
    /// video before transcoding)
    async fn copy(&self, from_key: &str, to_key: &str) -> StorageResult<String>;

    /// Get the storage backend type
    fn backend_type(&self) -> StorageBackend;

    /// Upload a file from a stream/reader (for large files)
    ///
    /// This method allows uploading large files without loading them entirely into memory.
    /// The reader will be consumed until EOF.
    ///
    /// # Arguments
    /// * `tenant_id` - Tenant identifier
    /// * `filename` - Filename for the uploaded file
    /// * `content_type` - MIME type of the content
    /// * `content_length` - Expected size of the content (may be used for optimization)
    /// * `reader` - Async reader that provides the file content
    ///
    /// # Returns
    /// A tuple of (storage_key, storage_url) on success
    async fn upload_stream(
        &self,
        tenant_id: Uuid,
        filename: &str,
        content_type: &str,
        content_length: Option<u64>,
        reader: Pin<Box<dyn AsyncRead + Send + Unpin>>,
    ) -> StorageResult<(String, String)>;

    /// Download a file as a stream (for large files)
    ///
    /// This method allows downloading large files without loading them entirely into memory.
    /// The stream yields `Bytes` chunks as they become available.
    ///
    /// # Arguments
    /// * `storage_key` - The storage key of the file to download
    ///
    /// # Returns
    /// A stream of `Bytes` chunks on success
    async fn download_stream(
        &self,
        storage_key: &str,
    ) -> StorageResult<Pin<Box<dyn Stream<Item = Result<Bytes, StorageError>> + Send>>>;
}
