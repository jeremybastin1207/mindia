#[cfg(feature = "storage-local")]
use crate::LocalStorage;
#[cfg(feature = "storage-s3")]
use crate::S3Storage;
use crate::{Storage, StorageBackend, StorageError, StorageResult};
use mindia_core::Config;
use std::sync::Arc;

/// Create a storage backend based on configuration
pub async fn create_storage(config: &Config) -> StorageResult<Arc<dyn Storage>> {
    let backend = config.storage_backend().unwrap_or(StorageBackend::S3);

    match backend {
        #[cfg(feature = "storage-s3")]
        StorageBackend::S3 => {
            let bucket = config
                .s3_bucket()
                .map(String::from)
                .ok_or_else(|| StorageError::ConfigError("S3_BUCKET not configured".to_string()))?;
            let region = config
                .s3_region()
                .map(String::from)
                .or_else(|| config.aws_region().map(String::from))
                .ok_or_else(|| {
                    StorageError::ConfigError("S3_REGION or AWS_REGION not configured".to_string())
                })?;
            let endpoint = config.s3_endpoint().map(String::from);

            let storage = S3Storage::new(bucket, region, endpoint).await?;
            Ok(Arc::new(storage))
        }

        #[cfg(not(feature = "storage-s3"))]
        StorageBackend::S3 => Err(StorageError::ConfigError(
            "S3 storage backend not available (storage-s3 feature not enabled)".to_string(),
        )),

        #[cfg(feature = "storage-local")]
        StorageBackend::Local => {
            let base_path = config
                .local_storage_path()
                .map(String::from)
                .ok_or_else(|| {
                    StorageError::ConfigError("LOCAL_STORAGE_PATH not configured".to_string())
                })?;
            let base_url = config
                .local_storage_base_url()
                .map(String::from)
                .ok_or_else(|| {
                    StorageError::ConfigError("LOCAL_STORAGE_BASE_URL not configured".to_string())
                })?;

            let storage = LocalStorage::new(base_path, base_url).await?;
            Ok(Arc::new(storage))
        }

        #[cfg(not(feature = "storage-local"))]
        StorageBackend::Local => Err(StorageError::ConfigError(
            "Local storage backend not available (storage-local feature not enabled)".to_string(),
        )),

        StorageBackend::Nfs => Err(StorageError::ConfigError(
            "NFS storage backend not yet implemented".to_string(),
        )),
    }
}

/// Create a storage backend for testing (uses local storage)
#[cfg(all(test, feature = "storage-local"))]
pub async fn create_test_storage() -> StorageResult<Arc<dyn Storage>> {
    use std::env;

    let temp_dir = env::temp_dir().join("mindia-test-storage");
    let base_url = "http://localhost:3000/media".to_string();

    let storage = LocalStorage::new(temp_dir, base_url).await?;
    Ok(Arc::new(storage))
}
