//! Mock Storage implementation for testing

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{self, Stream};
use mindia_services::{Storage, StorageBackend, StorageError, StorageResult};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncRead;
use uuid::Uuid;

/// Mock storage implementation that stores files in memory
pub struct MockStorage {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    backend_type: StorageBackend,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            backend_type: StorageBackend::Local,
        }
    }

    pub fn with_backend(backend_type: StorageBackend) -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            backend_type,
        }
    }

    /// Set a file in the mock storage
    pub fn set_file(&self, key: &str, data: Vec<u8>) {
        self.files.lock().unwrap().insert(key.to_string(), data);
    }

    /// Remove a file from the mock storage
    pub fn remove_file(&self, key: &str) {
        self.files.lock().unwrap().remove(key);
    }

    /// Check if a file exists in the mock storage
    pub fn has_file(&self, key: &str) -> bool {
        self.files.lock().unwrap().contains_key(key)
    }

    /// Get file data (for test assertions)
    pub fn get_file(&self, key: &str) -> Option<Vec<u8>> {
        self.files.lock().unwrap().get(key).cloned()
    }
}

impl Default for MockStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for MockStorage {
    async fn upload(
        &self,
        _tenant_id: Uuid,
        filename: &str,
        _content_type: &str,
        data: Vec<u8>,
    ) -> StorageResult<(String, String)> {
        let storage_key = format!("media/{}/{}", _tenant_id, filename);
        self.files.lock().unwrap().insert(storage_key.clone(), data);
        let storage_url = format!("https://example.com/{}", storage_key);
        Ok((storage_key, storage_url))
    }

    async fn upload_with_key(
        &self,
        storage_key: &str,
        data: Vec<u8>,
        _content_type: &str,
    ) -> StorageResult<String> {
        self.files
            .lock()
            .unwrap()
            .insert(storage_key.to_string(), data);
        Ok(format!("https://example.com/{}", storage_key))
    }

    async fn download(&self, storage_key: &str) -> StorageResult<Vec<u8>> {
        self.files
            .lock()
            .unwrap()
            .get(storage_key)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(storage_key.to_string()))
    }

    async fn delete(&self, storage_key: &str) -> StorageResult<()> {
        self.files
            .lock()
            .unwrap()
            .remove(storage_key)
            .ok_or_else(|| StorageError::NotFound(storage_key.to_string()))?;
        Ok(())
    }

    async fn get_presigned_url(
        &self,
        storage_key: &str,
        _expires_in: Duration,
    ) -> StorageResult<String> {
        if !self.files.lock().unwrap().contains_key(storage_key) {
            return Err(StorageError::NotFound(storage_key.to_string()));
        }
        Ok(format!("https://example.com/presigned/{}", storage_key))
    }

    async fn exists(&self, storage_key: &str) -> StorageResult<bool> {
        Ok(self.files.lock().unwrap().contains_key(storage_key))
    }

    async fn copy(&self, from_key: &str, to_key: &str) -> StorageResult<String> {
        let data = self.download(from_key).await?;
        self.files.lock().unwrap().insert(to_key.to_string(), data);
        Ok(format!("https://example.com/{}", to_key))
    }

    fn backend_type(&self) -> StorageBackend {
        self.backend_type
    }

    async fn upload_stream(
        &self,
        tenant_id: Uuid,
        filename: &str,
        _content_type: &str,
        _content_length: Option<u64>,
        mut reader: Pin<Box<dyn AsyncRead + Send + Unpin>>,
    ) -> StorageResult<(String, String)> {
        // Read the entire stream into memory (simplified for testing)
        use tokio::io::AsyncReadExt;
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .await
            .map_err(|e| StorageError::UploadFailed(e.to_string()))?;

        // Use the regular upload method
        self.upload(tenant_id, filename, _content_type, data).await
    }

    async fn download_stream(
        &self,
        storage_key: &str,
    ) -> StorageResult<Pin<Box<dyn Stream<Item = Result<Bytes, StorageError>> + Send>>> {
        // Download the file and wrap it in a stream
        let data = self.download(storage_key).await?;
        let byte_stream = stream::once(async move { Ok(Bytes::from(data)) });
        Ok(Box::pin(byte_stream))
    }
}
