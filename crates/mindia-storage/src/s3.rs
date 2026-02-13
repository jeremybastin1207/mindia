use crate::traits::{Storage, StorageError, StorageResult};
use crate::StorageBackend;
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use futures::StreamExt;
use http::Method;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::path::Path;
use object_store::signer::Signer;
use object_store::Error as ObjectStoreError;
use object_store::{ObjectStoreExt, PutPayload, Result as ObjectResult};
use std::pin::Pin;
use std::time::Duration;
use tokio::io::AsyncRead;
use uuid::Uuid;

/// S3 storage implementation
#[derive(Clone)]
pub struct S3Storage {
    store: AmazonS3,
    bucket: String,
    region: String,
    endpoint_url: Option<String>, // Custom endpoint for S3-compatible providers
}

impl S3Storage {
    /// Create a new S3Storage instance
    ///
    /// # Arguments
    /// * `bucket` - S3 bucket name
    /// * `region` - AWS region (or region identifier for S3-compatible providers)
    /// * `endpoint_url` - Optional custom endpoint URL for S3-compatible providers
    ///   (e.g., "http://localhost:9000" for MinIO, "https://nyc3.digitaloceanspaces.com" for DigitalOcean Spaces)
    pub async fn new(
        bucket: String,
        region: String,
        endpoint_url: Option<String>,
    ) -> StorageResult<Self> {
        // Build AmazonS3 object store from environment and explicit settings.
        let mut builder = AmazonS3Builder::from_env()
            .with_region(region.clone())
            .with_bucket_name(bucket.clone());

        if let Some(ref endpoint) = endpoint_url {
            let allow_http = endpoint.starts_with("http://");
            builder = builder
                .with_endpoint(endpoint.clone())
                .with_allow_http(allow_http);
        }

        let store = builder
            .build()
            .map_err(|e| StorageError::ConfigError(e.to_string()))?;

        Ok(S3Storage {
            store,
            bucket,
            region,
            endpoint_url,
        })
    }

    /// Generate S3 key path for a tenant and filename
    /// For default tenant, uses shorter path: media/{filename}
    fn generate_key(tenant_id: Uuid, filename: &str) -> String {
        if tenant_id == mindia_core::constants::DEFAULT_TENANT_ID {
            format!("media/{}", filename)
        } else {
            format!("media/{}/{}", tenant_id, filename)
        }
    }

    /// Generate public URL for S3 object
    ///
    /// For AWS S3, uses the standard format: https://{bucket}.s3.{region}.amazonaws.com/{key}
    /// For S3-compatible providers, uses the endpoint URL if provided
    fn generate_url(&self, key: &str) -> String {
        if let Some(ref endpoint) = self.endpoint_url {
            // For S3-compatible providers, construct URL from endpoint
            // Remove trailing slash if present
            let base_url = endpoint.trim_end_matches('/');
            // Some providers use path-style, others use virtual-hosted-style
            // We'll use path-style for compatibility: {endpoint}/{bucket}/{key}
            format!("{}/{}/{}", base_url, self.bucket, key)
        } else {
            // Standard AWS S3 URL format
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                self.bucket, self.region, key
            )
        }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn upload(
        &self,
        tenant_id: Uuid,
        filename: &str,
        _content_type: &str,
        data: Vec<u8>,
    ) -> StorageResult<(String, String)> {
        let key = Self::generate_key(tenant_id, filename);
        let size = data.len() as u64;
        let bytes = Bytes::from(data);
        let location = Path::from(key.clone());

        let start = std::time::Instant::now();

        let result: ObjectResult<_> = self
            .store
            .put(&location, PutPayload::from(bytes.clone()))
            .await;

        result.map_err(|e| {
            tracing::error!(
                error = %e,
                bucket = %self.bucket,
                key = %key,
                size_bytes = size,
                duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                "S3 upload failed"
            );
            StorageError::UploadFailed(e.to_string())
        })?;

        let url = self.generate_url(&key);

        tracing::info!(
            bucket = %self.bucket,
            key = %key,
            size_bytes = size,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "S3 upload successful"
        );

        Ok((key, url))
    }

    async fn upload_with_key(
        &self,
        storage_key: &str,
        data: Vec<u8>,
        _content_type: &str,
    ) -> StorageResult<String> {
        let size = data.len() as u64;
        let bytes = Bytes::from(data);
        let location = Path::from(storage_key.to_string());
        let start = std::time::Instant::now();

        let result: ObjectResult<_> = self.store.put(&location, PutPayload::from(bytes)).await;

        result.map_err(|e| {
            tracing::error!(
                error = %e,
                bucket = %self.bucket,
                key = %storage_key,
                size_bytes = size,
                duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                "S3 upload_with_key failed"
            );
            StorageError::UploadFailed(e.to_string())
        })?;

        let url = self.generate_url(storage_key);

        tracing::info!(
            bucket = %self.bucket,
            key = %storage_key,
            size_bytes = size,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "S3 upload_with_key successful"
        );

        Ok(url)
    }

    async fn download(&self, storage_key: &str) -> StorageResult<Vec<u8>> {
        let start = std::time::Instant::now();
        let location = Path::from(storage_key.to_string());

        let result: ObjectResult<_> = self.store.get(&location).await;

        let result = result.map_err(|e| match e {
            ObjectStoreError::NotFound { .. } => StorageError::NotFound(storage_key.to_string()),
            other => {
                tracing::error!(
                    error = %other,
                    bucket = %self.bucket,
                    key = %storage_key,
                    duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                    "S3 download failed"
                );
                StorageError::DownloadFailed(other.to_string())
            }
        })?;

        let bytes = result
            .bytes()
            .await
            .map_err(|e| StorageError::DownloadFailed(e.to_string()))?;
        let size = bytes.len() as u64;

        tracing::info!(
            bucket = %self.bucket,
            key = %storage_key,
            size_bytes = size,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "S3 download successful"
        );

        Ok(bytes.to_vec())
    }

    async fn delete(&self, storage_key: &str) -> StorageResult<()> {
        let start = std::time::Instant::now();
        let location = Path::from(storage_key.to_string());

        let result: ObjectResult<_> = self.store.delete(&location).await;

        result.map_err(|e| {
            tracing::error!(
                error = %e,
                bucket = %self.bucket,
                key = %storage_key,
                duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                "S3 delete failed"
            );
            StorageError::DeleteFailed(e.to_string())
        })?;

        tracing::info!(
            bucket = %self.bucket,
            key = %storage_key,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "S3 delete successful"
        );

        Ok(())
    }

    async fn get_presigned_url(
        &self,
        storage_key: &str,
        expires_in: Duration,
    ) -> StorageResult<String> {
        let location = Path::from(storage_key.to_string());
        let url_result: ObjectResult<_> = self
            .store
            .signed_url(Method::GET, &location, expires_in)
            .await;

        let url = url_result
            .map_err(|e| StorageError::BackendError(e.to_string()))?
            .to_string();

        Ok(url)
    }

    async fn exists(&self, storage_key: &str) -> StorageResult<bool> {
        let location = Path::from(storage_key.to_string());
        match self.store.head(&location).await {
            Ok(_) => Ok(true),
            Err(ObjectStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(StorageError::BackendError(e.to_string())),
        }
    }

    async fn copy(&self, from_key: &str, to_key: &str) -> StorageResult<String> {
        let start = std::time::Instant::now();
        let from = Path::from(from_key.to_string());
        let to = Path::from(to_key.to_string());

        let copy_result: ObjectResult<_> = self.store.copy(&from, &to).await;

        copy_result.map_err(|e| StorageError::BackendError(e.to_string()))?;

        let url = self.generate_url(to_key);

        tracing::info!(
            from_key = %from_key,
            to_key = %to_key,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "S3 copy successful"
        );

        Ok(url)
    }

    fn backend_type(&self) -> StorageBackend {
        StorageBackend::S3
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
        let start = std::time::Instant::now();
        // For simplicity, read the entire stream into memory and upload in a single put.
        // This is less optimal for very large files but significantly simplifies the
        // implementation while still benefiting from object_store's S3 integration.
        let mut buffer = Vec::new();
        let mut temp_buf = vec![0u8; 8192];

        loop {
            let bytes_read = tokio::io::AsyncReadExt::read(&mut reader, &mut temp_buf)
                .await
                .map_err(|e| {
                    StorageError::UploadFailed(format!("Failed to read from stream: {}", e))
                })?;

            if bytes_read == 0 {
                break;
            }

            buffer.extend_from_slice(&temp_buf[..bytes_read]);
        }

        let size = buffer.len() as u64;
        let bytes = Bytes::from(buffer);
        let location = Path::from(key.clone());

        let result: ObjectResult<_> = self.store.put(&location, PutPayload::from(bytes)).await;

        result.map_err(|e| {
            tracing::error!(
                error = %e,
                bucket = %self.bucket,
                key = %key,
                size_bytes = size,
                duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                "S3 stream upload failed"
            );
            StorageError::UploadFailed(e.to_string())
        })?;

        let url = self.generate_url(&key);

        tracing::info!(
            bucket = %self.bucket,
            key = %key,
            size_bytes = size,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "S3 stream upload successful"
        );

        Ok((key, url))
    }

    async fn download_stream(
        &self,
        storage_key: &str,
    ) -> StorageResult<Pin<Box<dyn Stream<Item = Result<Bytes, StorageError>> + Send>>> {
        let start = std::time::Instant::now();
        let location = Path::from(storage_key.to_string());

        let result: ObjectResult<_> = self.store.get(&location).await;

        let result = result.map_err(|e| match e {
            ObjectStoreError::NotFound { .. } => StorageError::NotFound(storage_key.to_string()),
            other => StorageError::DownloadFailed(other.to_string()),
        })?;

        let bucket = self.bucket.clone();
        let key = storage_key.to_string();

        let stream = result.into_stream().map(move |res| match res {
            Ok(bytes) => Ok(bytes),
            Err(e) => {
                tracing::error!(
                    bucket = %bucket,
                    key = %key,
                    duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                    "S3 stream download error"
                );
                Err(StorageError::DownloadFailed(e.to_string()))
            }
        });

        Ok(Box::pin(stream))
    }
}
