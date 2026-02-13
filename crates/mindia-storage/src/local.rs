use crate::traits::{Storage, StorageError, StorageResult};
use crate::StorageBackend;
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use futures::StreamExt;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncRead, AsyncWriteExt};
use uuid::Uuid;

/// Local filesystem storage implementation
#[derive(Clone)]
pub struct LocalStorage {
    base_path: PathBuf,
    base_url: String,
}

impl LocalStorage {
    /// Create a new LocalStorage instance
    ///
    /// # Arguments
    /// * `base_path` - Root directory for file storage (e.g., "/var/lib/mindia/media")
    /// * `base_url` - Base URL for serving files (e.g., "http://localhost:3000/media")
    pub async fn new(base_path: impl Into<PathBuf>, base_url: String) -> StorageResult<Self> {
        let base_path = base_path.into();

        fs::create_dir_all(&base_path).await.map_err(|e| {
            StorageError::ConfigError(format!(
                "Failed to create storage directory {}: {}",
                base_path.display(),
                e
            ))
        })?;

        Ok(LocalStorage {
            base_path,
            base_url,
        })
    }

    /// Convert storage key to filesystem path with security validation
    ///
    /// This function validates that the storage key doesn't contain path traversal
    /// sequences that could escape the base storage directory.
    fn key_to_path(&self, storage_key: &str) -> StorageResult<PathBuf> {
        if storage_key.contains("..") || storage_key.starts_with('/') {
            return Err(StorageError::InvalidKey(
                "Storage key contains invalid characters".to_string(),
            ));
        }

        let path = self.base_path.join(storage_key);

        let base_canonical = self.base_path.canonicalize().map_err(|e| {
            StorageError::ConfigError(format!("Failed to canonicalize base path: {}", e))
        })?;

        if let Ok(canonical) = path.canonicalize() {
            if canonical.strip_prefix(&base_canonical).is_err() {
                return Err(StorageError::InvalidKey(
                    "Storage key resolves outside storage directory".to_string(),
                ));
            }
        } else {
            let mut current = path.clone();
            loop {
                if current == self.base_path {
                    break;
                }
                if let Some(parent) = current.parent() {
                    let parent_buf = parent.to_path_buf();
                    if parent_buf.strip_prefix(&self.base_path).is_err() && parent_buf != self.base_path {
                        return Err(StorageError::InvalidKey(
                            "Storage key resolves outside storage directory".to_string(),
                        ));
                    }
                    current = parent_buf;
                } else {
                    break;
                }
            }
        }

        Ok(path)
    }

    /// Generate storage key from tenant and filename
    /// For default tenant, uses shorter path: media/{filename}
    fn generate_key(tenant_id: Uuid, filename: &str) -> String {
        if tenant_id == mindia_core::constants::DEFAULT_TENANT_ID {
            format!("media/{}", filename)
        } else {
            format!("media/{}/{}", tenant_id, filename)
        }
    }

    /// Generate public URL for file
    fn generate_url(&self, key: &str) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), key)
    }

    /// Ensure parent directory exists
    async fn ensure_parent_dir(&self, path: &Path) -> StorageResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn upload(
        &self,
        tenant_id: Uuid,
        filename: &str,
        _content_type: &str,
        data: Vec<u8>,
    ) -> StorageResult<(String, String)> {
        let key = Self::generate_key(tenant_id, filename);
        let path = self.key_to_path(&key)?;
        let size = data.len();

        self.ensure_parent_dir(&path).await?;

        let start = std::time::Instant::now();

        let mut file = fs::File::create(&path).await.map_err(|e| {
            StorageError::UploadFailed(format!("Failed to create file {}: {}", path.display(), e))
        })?;

        file.write_all(&data).await.map_err(|e| {
            StorageError::UploadFailed(format!("Failed to write file {}: {}", path.display(), e))
        })?;

        file.sync_all().await.map_err(|e| {
            StorageError::UploadFailed(format!("Failed to sync file {}: {}", path.display(), e))
        })?;

        let url = self.generate_url(&key);

        tracing::info!(
            path = %path.display(),
            key = %key,
            size_bytes = size,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "Local storage upload successful"
        );

        Ok((key, url))
    }

    async fn download(&self, storage_key: &str) -> StorageResult<Vec<u8>> {
        let path = self.key_to_path(storage_key)?;
        let start = std::time::Instant::now();

        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Err(StorageError::NotFound(storage_key.to_string()));
        }

        let data = fs::read(&path).await.map_err(|e| {
            StorageError::DownloadFailed(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        let size = data.len();

        tracing::info!(
            path = %path.display(),
            key = %storage_key,
            size_bytes = size,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "Local storage download successful"
        );

        Ok(data)
    }

    async fn upload_with_key(
        &self,
        storage_key: &str,
        data: Vec<u8>,
        _content_type: &str,
    ) -> StorageResult<String> {
        let path = self.key_to_path(storage_key)?;
        let size = data.len();

        self.ensure_parent_dir(&path).await?;

        let start = std::time::Instant::now();

        let mut file = fs::File::create(&path).await.map_err(|e| {
            StorageError::UploadFailed(format!("Failed to create file {}: {}", path.display(), e))
        })?;

        file.write_all(&data).await.map_err(|e| {
            StorageError::UploadFailed(format!("Failed to write file {}: {}", path.display(), e))
        })?;

        file.sync_all().await.map_err(|e| {
            StorageError::UploadFailed(format!("Failed to sync file {}: {}", path.display(), e))
        })?;

        let url = self.generate_url(storage_key);

        tracing::info!(
            path = %path.display(),
            key = %storage_key,
            size_bytes = size,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "Local storage upload_with_key successful"
        );

        Ok(url)
    }

    async fn delete(&self, storage_key: &str) -> StorageResult<()> {
        let path = self.key_to_path(storage_key)?;
        let start = std::time::Instant::now();

        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Ok(());
        }

        fs::remove_file(&path).await.map_err(|e| {
            StorageError::DeleteFailed(format!("Failed to delete file {}: {}", path.display(), e))
        })?;

        tracing::info!(
            path = %path.display(),
            key = %storage_key,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "Local storage delete successful"
        );

        Ok(())
    }

    async fn get_presigned_url(
        &self,
        storage_key: &str,
        _expires_in: Duration,
    ) -> StorageResult<String> {
        self.key_to_path(storage_key)?;
        Ok(self.generate_url(storage_key))
    }

    async fn exists(&self, storage_key: &str) -> StorageResult<bool> {
        let path = self.key_to_path(storage_key)?;
        Ok(tokio::fs::try_exists(&path).await.unwrap_or(false))
    }

    async fn content_length(&self, storage_key: &str) -> StorageResult<u64> {
        let path = self.key_to_path(storage_key)?;
        let meta = fs::metadata(&path)
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        Ok(meta.len())
    }

    async fn copy(&self, from_key: &str, to_key: &str) -> StorageResult<String> {
        let from_path = self.key_to_path(from_key)?;
        let to_path = self.key_to_path(to_key)?;

        if !tokio::fs::try_exists(&from_path).await.unwrap_or(false) {
            return Err(StorageError::NotFound(from_key.to_string()));
        }

        self.ensure_parent_dir(&to_path).await?;

        fs::copy(&from_path, &to_path).await.map_err(|e| {
            StorageError::BackendError(format!(
                "Failed to copy {} to {}: {}",
                from_path.display(),
                to_path.display(),
                e
            ))
        })?;

        let url = self.generate_url(to_key);

        tracing::info!(
            from_key = %from_key,
            to_key = %to_key,
            from_path = %from_path.display(),
            to_path = %to_path.display(),
            "Local storage copy successful"
        );

        Ok(url)
    }

    fn backend_type(&self) -> StorageBackend {
        StorageBackend::Local
    }

    async fn upload_stream(
        &self,
        tenant_id: Uuid,
        filename: &str,
        _content_type: &str,
        _content_length: Option<u64>,
        mut reader: Pin<Box<dyn AsyncRead + Send + Unpin>>,
    ) -> StorageResult<(String, String)> {
        let key = Self::generate_key(tenant_id, filename);
        let path = self.key_to_path(&key)?;
        let start = std::time::Instant::now();

        self.ensure_parent_dir(&path).await?;

        let mut file = fs::File::create(&path).await.map_err(|e| {
            StorageError::UploadFailed(format!("Failed to create file {}: {}", path.display(), e))
        })?;

        let bytes_copied = tokio::io::copy(&mut reader, &mut file).await.map_err(|e| {
            StorageError::UploadFailed(format!(
                "Failed to write stream to file {}: {}",
                path.display(),
                e
            ))
        })?;

        file.sync_all().await.map_err(|e| {
            StorageError::UploadFailed(format!("Failed to sync file {}: {}", path.display(), e))
        })?;

        let url = self.generate_url(&key);

        tracing::info!(
            path = %path.display(),
            key = %key,
            size_bytes = bytes_copied,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "Local storage stream upload successful"
        );

        Ok((key, url))
    }

    async fn download_stream(
        &self,
        storage_key: &str,
    ) -> StorageResult<Pin<Box<dyn Stream<Item = Result<Bytes, StorageError>> + Send>>> {
        let path = self.key_to_path(storage_key)?;
        let start = std::time::Instant::now();

        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Err(StorageError::NotFound(storage_key.to_string()));
        }

        let file = fs::File::open(&path).await.map_err(|e| {
            StorageError::DownloadFailed(format!("Failed to open file {}: {}", path.display(), e))
        })?;

        let reader = tokio_util::io::ReaderStream::new(file);

        let stream = reader.map(|result| {
            result.map_err(|e| StorageError::DownloadFailed(format!("Failed to read chunk: {}", e)))
        });

        let key = storage_key.to_string();
        let path_display = path.display().to_string();
        let logged_stream = stream.map(move |item| {
            if item.is_err() {
                tracing::error!(
                    path = %path_display,
                    key = %key,
                    duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                    "Local storage stream download error"
                );
            }
            item
        });

        Ok(Box::pin(logged_stream))
    }
}

#[cfg(all(test, feature = "storage-local"))]
mod tests {
    use super::*;
    use futures::StreamExt;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_local_storage_upload_download() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000/media".to_string())
            .await
            .unwrap();

        let tenant_id = Uuid::new_v4();
        let data = b"test data".to_vec();

        let (key, url) = storage
            .upload(tenant_id, "test.txt", "text/plain", data.clone())
            .await
            .unwrap();

        assert!(key.contains("test.txt"));
        assert!(url.contains("test.txt"));

        let downloaded = storage.download(&key).await.unwrap();
        assert_eq!(data, downloaded);
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000/media".to_string())
            .await
            .unwrap();

        let result = storage.download("../../../etc/passwd").await;
        assert!(matches!(result, Err(StorageError::InvalidKey(_))));

        let result = storage.delete("../etc/passwd").await;
        assert!(matches!(result, Err(StorageError::InvalidKey(_))));

        let result = storage.exists("/etc/passwd").await;
        assert!(matches!(result, Err(StorageError::InvalidKey(_))));
    }

    #[tokio::test]
    async fn test_local_storage_delete_nonexistent() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000/media".to_string())
            .await
            .unwrap();

        let result = storage.delete("nonexistent/file.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_local_storage_exists() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000/media".to_string())
            .await
            .unwrap();

        let tenant_id = Uuid::new_v4();
        let data = b"test".to_vec();

        let (key, _) = storage
            .upload(tenant_id, "exists.txt", "text/plain", data)
            .await
            .unwrap();

        assert!(storage.exists(&key).await.unwrap());
        assert!(!storage.exists("nonexistent.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_local_storage_copy() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000/media".to_string())
            .await
            .unwrap();

        let tenant_id = Uuid::new_v4();
        let data = b"original content".to_vec();

        let (from_key, _) = storage
            .upload(tenant_id, "original.txt", "text/plain", data.clone())
            .await
            .unwrap();

        let to_key = format!("media/{}/copied.txt", tenant_id);
        let url = storage.copy(&from_key, &to_key).await.unwrap();

        assert!(url.contains("copied.txt"));

        let copied_data = storage.download(&to_key).await.unwrap();
        assert_eq!(data, copied_data);
    }

    #[tokio::test]
    async fn test_local_storage_stream_upload() {
        use std::pin::Pin;

        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000/media".to_string())
            .await
            .unwrap();

        let tenant_id = Uuid::new_v4();
        let data = b"stream test data".to_vec();
        let cursor = std::io::Cursor::new(data.clone());
        let reader = Box::pin(cursor) as Pin<Box<dyn tokio::io::AsyncRead + Send + Unpin>>;

        let (key, _) = storage
            .upload_stream(
                tenant_id,
                "stream.txt",
                "text/plain",
                Some(data.len() as u64),
                reader,
            )
            .await
            .unwrap();

        let downloaded = storage.download(&key).await.unwrap();
        assert_eq!(data, downloaded);
    }

    #[tokio::test]
    async fn test_local_storage_stream_download() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000/media".to_string())
            .await
            .unwrap();

        let tenant_id = Uuid::new_v4();
        let data = b"stream download test".to_vec();

        let (key, _) = storage
            .upload(tenant_id, "stream_dl.txt", "text/plain", data.clone())
            .await
            .unwrap();

        let mut stream = storage.download_stream(&key).await.unwrap();
        let mut downloaded = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.unwrap();
            downloaded.extend_from_slice(&chunk);
        }

        assert_eq!(data, downloaded);
    }
}
