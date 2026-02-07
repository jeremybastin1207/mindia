use crate::traits::{Storage, StorageError, StorageResult};
use crate::StorageBackend;
use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_config::retry::{RetryConfig, RetryMode};
use aws_config::BehaviorVersion;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::Client;
use bytes::Bytes;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

/// S3 storage implementation
#[derive(Clone)]
pub struct S3Storage {
    client: Client,
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
        let region_provider =
            RegionProviderChain::first_try(aws_config::Region::new(region.clone()));

        let retry_config = RetryConfig::standard()
            .with_max_attempts(5)
            .with_retry_mode(RetryMode::Adaptive);

        let config_builder = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .retry_config(retry_config.clone());

        let config = config_builder.load().await;

        // Configure S3 client with custom endpoint if provided (for S3-compatible providers)
        let client = if let Some(ref endpoint) = endpoint_url {
            // Build S3 config with custom endpoint
            // For S3-compatible providers, we may need path-style addressing
            let mut s3_config_builder = aws_sdk_s3::Config::builder()
                .endpoint_url(endpoint)
                .region(config.region().cloned())
                .retry_config(retry_config);
            if let Some(provider) = config.credentials_provider().into_iter().next() {
                s3_config_builder = s3_config_builder.credentials_provider(provider);
            }
            // Use path-style addressing for S3-compatible providers (required for MinIO, etc.)
            s3_config_builder = s3_config_builder.force_path_style(true);

            let s3_config = s3_config_builder.build();
            Client::from_conf(s3_config)
        } else {
            // Use default AWS S3 client
            Client::new(&config)
        };

        Ok(S3Storage {
            client,
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
        content_type: &str,
        data: Vec<u8>,
    ) -> StorageResult<(String, String)> {
        let key = Self::generate_key(tenant_id, filename);
        let size = data.len() as u64;

        let body = ByteStream::from(Bytes::from(data));

        let start = std::time::Instant::now();

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| {
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
        content_type: &str,
    ) -> StorageResult<String> {
        let size = data.len() as u64;
        let body = ByteStream::from(Bytes::from(data));
        let start = std::time::Instant::now();

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(storage_key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| {
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

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(storage_key)
            .send()
            .await
            .map_err(|e| match &e {
                SdkError::ServiceError(service_err) => match service_err.err() {
                    GetObjectError::NoSuchKey(_) => StorageError::NotFound(storage_key.to_string()),
                    _ => {
                        tracing::error!(
                            error = %e,
                            bucket = %self.bucket,
                            key = %storage_key,
                            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                            "S3 download failed"
                        );
                        StorageError::DownloadFailed(e.to_string())
                    }
                },
                _ => {
                    tracing::error!(
                        error = %e,
                        bucket = %self.bucket,
                        key = %storage_key,
                        duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                        "S3 download failed"
                    );
                    StorageError::DownloadFailed(e.to_string())
                }
            })?;

        let data = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::DownloadFailed(e.to_string()))?;

        let bytes = data.into_bytes().to_vec();
        let size = bytes.len() as u64;

        tracing::info!(
            bucket = %self.bucket,
            key = %storage_key,
            size_bytes = size,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "S3 download successful"
        );

        Ok(bytes)
    }

    async fn delete(&self, storage_key: &str) -> StorageResult<()> {
        let start = std::time::Instant::now();

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(storage_key)
            .send()
            .await
            .map_err(|e| {
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
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::builder()
            .expires_in(expires_in)
            .build()
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(storage_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        Ok(presigned_request.uri().to_string())
    }

    async fn exists(&self, storage_key: &str) -> StorageResult<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(storage_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => match &e {
                SdkError::ServiceError(service_err) => match service_err.err() {
                    HeadObjectError::NotFound(_) => Ok(false),
                    _ => Err(StorageError::BackendError(e.to_string())),
                },
                _ => Err(StorageError::BackendError(e.to_string())),
            },
        }
    }

    async fn copy(&self, from_key: &str, to_key: &str) -> StorageResult<String> {
        let start = std::time::Instant::now();

        // URL-encode the copy source per AWS S3 API requirements
        let encoded_key = urlencoding::encode(from_key);
        let copy_source = format!("{}/{}", self.bucket, encoded_key);

        self.client
            .copy_object()
            .bucket(&self.bucket)
            .copy_source(&copy_source)
            .key(to_key)
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

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
        content_type: &str,
        content_length: Option<u64>,
        mut reader: Pin<Box<dyn AsyncRead + Send + Unpin>>,
    ) -> StorageResult<(String, String)> {
        let key = Self::generate_key(tenant_id, filename);
        let start = std::time::Instant::now();

        // Use multipart upload for files larger than 5MB, otherwise use regular upload
        const MULTIPART_THRESHOLD: u64 = 5 * 1024 * 1024; // 5MB
        const PART_SIZE: usize = 5 * 1024 * 1024; // 5MB per part (minimum is 5MB except last part)

        let use_multipart = content_length
            .map(|len| len > MULTIPART_THRESHOLD)
            .unwrap_or(true);

        if use_multipart {
            // Use multipart upload for large files
            let create_result = self
                .client
                .create_multipart_upload()
                .bucket(&self.bucket)
                .key(&key)
                .content_type(content_type)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(
                        error = %e,
                        bucket = %self.bucket,
                        key = %key,
                        "Failed to create multipart upload"
                    );
                    StorageError::UploadFailed(e.to_string())
                })?;

            let upload_id = create_result.upload_id().ok_or_else(|| {
                StorageError::UploadFailed("No upload ID returned from S3".to_string())
            })?;

            let mut part_number = 1u32;
            let mut parts = Vec::new();
            let mut part_buffer = vec![0u8; PART_SIZE];
            let mut total_size = 0u64;

            loop {
                // Read a part's worth of data
                let mut bytes_in_part = 0usize;
                while bytes_in_part < PART_SIZE {
                    let bytes_read = reader
                        .read(&mut part_buffer[bytes_in_part..])
                        .await
                        .map_err(|e| {
                            StorageError::UploadFailed(format!("Failed to read from stream: {}", e))
                        })?;

                    if bytes_read == 0 {
                        break; // EOF
                    }

                    bytes_in_part += bytes_read;
                }

                if bytes_in_part == 0 {
                    break; // No more data
                }

                total_size += bytes_in_part as u64;

                // Upload this part
                let part_data = Bytes::from(part_buffer[..bytes_in_part].to_vec());
                let part_body = ByteStream::from(part_data.clone());

                let upload_part_result = self
                    .client
                    .upload_part()
                    .bucket(&self.bucket)
                    .key(&key)
                    .upload_id(upload_id)
                    .part_number(part_number as i32)
                    .body(part_body)
                    .send()
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            error = %e,
                            bucket = %self.bucket,
                            key = %key,
                            part_number = part_number,
                            "Failed to upload part"
                        );
                        StorageError::UploadFailed(e.to_string())
                    })?;

                let etag = upload_part_result
                    .e_tag()
                    .ok_or_else(|| {
                        StorageError::UploadFailed(format!(
                            "No ETag returned for part {}",
                            part_number
                        ))
                    })?
                    .to_string();

                let completed_part = CompletedPart::builder()
                    .part_number(part_number as i32)
                    .e_tag(etag)
                    .build();
                parts.push(completed_part);

                part_number += 1;

                // If we read less than PART_SIZE, we've reached EOF
                if bytes_in_part < PART_SIZE {
                    break;
                }
            }

            // Complete multipart upload
            let completed_parts = CompletedMultipartUpload::builder()
                .set_parts(Some(parts))
                .build();

            self.client
                .complete_multipart_upload()
                .bucket(&self.bucket)
                .key(&key)
                .upload_id(upload_id)
                .multipart_upload(completed_parts)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(
                        error = %e,
                        bucket = %self.bucket,
                        key = %key,
                        "Failed to complete multipart upload"
                    );
                    StorageError::UploadFailed(e.to_string())
                })?;

            let url = self.generate_url(&key);

            tracing::info!(
                bucket = %self.bucket,
                key = %key,
                size_bytes = total_size,
                parts = part_number - 1,
                duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                "S3 multipart stream upload successful"
            );

            Ok((key, url))
        } else {
            // For smaller files, use regular upload but still stream
            // Read in chunks and stream directly to S3
            let mut buffer = Vec::new();
            let mut temp_buf = vec![0u8; 8192]; // 8KB buffer

            loop {
                let bytes_read = reader.read(&mut temp_buf).await.map_err(|e| {
                    StorageError::UploadFailed(format!("Failed to read from stream: {}", e))
                })?;

                if bytes_read == 0 {
                    break; // EOF
                }

                buffer.extend_from_slice(&temp_buf[..bytes_read]);
            }

            let size = buffer.len() as u64;
            let body = ByteStream::from(Bytes::from(buffer));

            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(&key)
                .body(body)
                .content_type(content_type)
                .send()
                .await
                .map_err(|e| {
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
    }

    async fn download_stream(
        &self,
        storage_key: &str,
    ) -> StorageResult<Pin<Box<dyn Stream<Item = Result<Bytes, StorageError>> + Send>>> {
        let start = std::time::Instant::now();

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(storage_key)
            .send()
            .await
            .map_err(|e| match &e {
                SdkError::ServiceError(service_err) => match service_err.err() {
                    GetObjectError::NoSuchKey(_) => StorageError::NotFound(storage_key.to_string()),
                    _ => StorageError::DownloadFailed(e.to_string()),
                },
                _ => StorageError::DownloadFailed(e.to_string()),
            })?;

        // Convert ByteStream to Stream<Item = Result<Bytes, StorageError>> via AsyncRead + ReaderStream
        let async_read = response.body.into_async_read();
        let stream = ReaderStream::new(async_read)
            .map(|result| result.map_err(|e| StorageError::DownloadFailed(e.to_string())));

        // Wrap with logging
        let bucket = self.bucket.clone();
        let key = storage_key.to_string();
        let logged_stream = stream.map(move |item| {
            if item.is_err() {
                tracing::error!(
                    bucket = %bucket,
                    key = %key,
                    duration_ms = start.elapsed().as_secs_f64() * 1000.0,
                    "S3 stream download error"
                );
            }
            item
        });

        Ok(Box::pin(logged_stream))
    }
}
