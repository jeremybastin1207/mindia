use bytes::Bytes;
use http::Method;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::path::Path;
use object_store::signer::Signer;
use object_store::{Attribute, Attributes, ObjectStore, ObjectStoreExt, PutOptions, PutPayload};
use std::time::Duration;

#[derive(Clone)]
pub struct S3Service {
    store: AmazonS3,
    default_bucket: Option<String>,
    region: String,
    endpoint_url: Option<String>,
}

impl S3Service {
    /// Create a new S3Service with AWS client.
    ///
    /// Uses single-bucket semantics: when `default_bucket` is `Some`, all operations
    /// (upload, delete, get) use that bucket. The `bucket` parameter in method calls
    /// must match `default_bucket`; it is used for URL construction and validation.
    ///
    /// # Arguments
    /// * `default_bucket` - The S3 bucket to use for all operations (required for correctness)
    /// * `region` - AWS region (or region identifier for S3-compatible providers)
    /// * `endpoint_url` - Optional custom endpoint for S3-compatible providers (MinIO, DigitalOcean Spaces, etc.)
    pub async fn new(
        default_bucket: Option<String>,
        region: String,
        endpoint_url: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let mut builder = AmazonS3Builder::from_env().with_region(region.clone());
        if let Some(ref bucket) = default_bucket {
            builder = builder.with_bucket_name(bucket.clone());
        }
        if let Some(ref endpoint) = endpoint_url {
            let allow_http = endpoint.starts_with("http://");
            builder = builder
                .with_endpoint(endpoint.clone())
                .with_allow_http(allow_http);
        }

        let store = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build S3 object store: {}", e))?;

        Ok(S3Service {
            store,
            default_bucket,
            region,
            endpoint_url,
        })
    }

    /// Validates that the passed bucket matches the configured default (single-bucket semantics).
    fn ensure_bucket_matches(&self, bucket: &str) -> Result<(), anyhow::Error> {
        match &self.default_bucket {
            Some(expected) if expected != bucket => Err(anyhow::anyhow!(
                "Bucket mismatch: S3Service is configured for bucket '{}' but got '{}'. \
                 Use single-bucket semantics - pass the same bucket used at construction.",
                expected,
                bucket
            )),
            None if !bucket.is_empty() => Err(anyhow::anyhow!(
                "S3Service was built without a default bucket; bucket '{}' cannot be used",
                bucket
            )),
            _ => Ok(()),
        }
    }

    /// Upload a file to S3.
    ///
    /// `bucket` must match the bucket configured at construction (single-bucket semantics).
    /// `content_type` is set as the S3 object's Content-Type metadata for correct HTTP serving.
    #[tracing::instrument(skip(self, data), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.key = %key,
        aws.s3.operation = "PutObject",
        s3.bucket = %bucket,
        s3.key = %key,
        s3.size = %data.len()
    ))]
    pub async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> Result<String, anyhow::Error> {
        self.ensure_bucket_matches(bucket)?;
        let start = std::time::Instant::now();
        let size = data.len() as u64;

        let location = Path::from(key.to_string());
        let mut attrs = Attributes::new();
        attrs.insert(Attribute::ContentType, content_type.to_string().into());
        let opts = PutOptions::from(attrs);
        let result = self
            .store
            .put_opts(&location, PutPayload::from(data), opts)
            .await;

        let duration = start.elapsed().as_secs_f64();

        match result {
            Ok(_) => {
                let url = self.build_object_url(bucket, key);
                tracing::info!(
                    size_bytes = size,
                    duration_ms = duration * 1000.0,
                    "S3 upload successful"
                );
                Ok(url)
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    size_bytes = size,
                    duration_ms = duration * 1000.0,
                    "S3 upload failed"
                );
                Err(e.into())
            }
        }
    }

    /// Build the public URL for an object (supports AWS and S3-compatible endpoints).
    fn build_object_url(&self, bucket: &str, key: &str) -> String {
        if let Some(ref endpoint) = self.endpoint_url {
            let base_url = endpoint.trim_end_matches('/');
            format!("{}/{}/{}", base_url, bucket, key)
        } else {
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                bucket, self.region, key
            )
        }
    }

    /// Delete a file from S3.
    /// `bucket` must match the bucket configured at construction.
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.key = %key,
        aws.s3.operation = "DeleteObject",
        s3.bucket = %bucket,
        s3.key = %key
    ))]
    pub async fn delete_file(&self, bucket: &str, key: &str) -> Result<(), anyhow::Error> {
        self.ensure_bucket_matches(bucket)?;
        let start = std::time::Instant::now();

        let location = Path::from(key.to_string());
        let result = self.store.delete(&location).await;

        let duration = start.elapsed().as_secs_f64();

        match result {
            Ok(_) => {
                tracing::info!(duration_ms = duration * 1000.0, "S3 delete successful");
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    duration_ms = duration * 1000.0,
                    "S3 delete failed"
                );
                Err(e.into())
            }
        }
    }

    /// Get a file from S3.
    /// `bucket` must match the bucket configured at construction.
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.key = %key,
        aws.s3.operation = "GetObject",
        s3.bucket = %bucket,
        s3.key = %key
    ))]
    pub async fn get_file(&self, bucket: &str, key: &str) -> Result<Bytes, anyhow::Error> {
        self.ensure_bucket_matches(bucket)?;
        let start = std::time::Instant::now();

        let location = Path::from(key.to_string());
        let result = self.store.get(&location).await;

        match result {
            Ok(response) => {
                let bytes = response.bytes().await?;
                let size = bytes.len() as u64;
                let duration = start.elapsed().as_secs_f64();

                tracing::info!(
                    size_bytes = size,
                    duration_ms = duration * 1000.0,
                    "S3 download successful"
                );
                Ok(bytes)
            }
            Err(e) => {
                let duration = start.elapsed().as_secs_f64();
                tracing::error!(
                    error = %e,
                    duration_ms = duration * 1000.0,
                    "S3 download failed"
                );
                Err(e.into())
            }
        }
    }

    /// Create a new S3 bucket (for tenant provisioning).
    ///
    /// Deprecated: object_store does not expose bucket creation. Provision buckets
    /// via infrastructure tooling (Terraform, CloudFormation, etc.).
    #[deprecated(
        since = "0.1.0",
        note = "Provision S3 buckets via infrastructure tooling instead"
    )]
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.operation = "CreateBucket",
        s3.bucket = %bucket
    ))]
    pub async fn create_bucket(&self, bucket: &str, region: &str) -> Result<(), anyhow::Error> {
        let _ = (bucket, region);
        Err(anyhow::anyhow!(
            "create_bucket is not supported; provision S3 buckets via infrastructure"
        ))
    }

    /// Check if a bucket exists.
    ///
    /// Deprecated: object_store does not expose bucket-level existence checks.
    /// Use infrastructure tooling or AWS CLI for bucket checks.
    #[deprecated(
        since = "0.1.0",
        note = "Check bucket existence via infrastructure or AWS CLI instead"
    )]
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.operation = "HeadBucket",
        s3.bucket = %bucket
    ))]
    pub async fn bucket_exists(&self, bucket: &str) -> Result<bool, anyhow::Error> {
        let _ = bucket;
        Err(anyhow::anyhow!(
            "bucket_exists is not supported; check bucket existence via infrastructure"
        ))
    }

    /// Get the default bucket
    pub fn default_bucket(&self) -> Option<&str> {
        self.default_bucket.as_deref()
    }

    /// Get the S3 client (for advanced operations like multipart uploads)
    pub fn client(&self) -> &AmazonS3 {
        &self.store
    }

    /// Generate a presigned PUT URL for direct S3 uploads.
    ///
    /// This allows clients to upload directly to S3 without going through the API server.
    /// The URL is time-limited. `bucket` must match the bucket configured at construction.
    #[tracing::instrument(skip(self), fields(s3.bucket = %bucket, s3.key = %key))]
    pub async fn generate_presigned_put_url(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        expires_in_seconds: u64,
    ) -> Result<String, anyhow::Error> {
        self.ensure_bucket_matches(bucket)?;
        let location = Path::from(key.to_string());
        let url = self
            .store
            .signed_url(
                Method::PUT,
                &location,
                Duration::from_secs(expires_in_seconds),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate presigned URL: {}", e))?;

        let url = url.to_string();

        tracing::info!(
            expires_in_seconds = expires_in_seconds,
            "Generated presigned PUT URL"
        );

        Ok(url)
    }

    /// Generate presigned data for direct S3 uploads.
    ///
    /// Returns a PUT-based presigned URL with metadata. Clients should use HTTP PUT
    /// to the returned URL (this is NOT standard S3 POST form upload with policy/signature).
    /// `bucket` must match the bucket configured at construction.
    #[tracing::instrument(skip(self), fields(s3.bucket = %bucket, s3.key = %key))]
    pub async fn generate_presigned_post_url(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        expires_in_seconds: u64,
    ) -> Result<PresignedPutData, anyhow::Error> {
        let url = self
            .generate_presigned_put_url(bucket, key, content_type, expires_in_seconds)
            .await?;

        tracing::info!(
            expires_in_seconds = expires_in_seconds,
            "Generated presigned PUT URL"
        );

        Ok(PresignedPutData {
            url,
            key: key.to_string(),
            content_type: content_type.to_string(),
        })
    }
}

/// Presigned PUT upload data.
///
/// Use HTTP PUT to `url` with the given Content-Type. The key is included for client reference
/// (the presigned URL already encodes the object path). This is not S3 POST form upload.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PresignedPutData {
    /// The presigned PUT URL; clients upload with HTTP PUT to this URL
    pub url: String,
    /// Object key (for client reference)
    pub key: String,
    /// Content-Type to send with the PUT request
    pub content_type: String,
}

/// Presigned POST form data structure.
///
/// Alias for backwards compatibility. Prefer [`PresignedPutData`] - this struct wraps
/// a PUT-based presigned URL, not standard S3 POST form upload.
#[deprecated(
    since = "0.1.0",
    note = "Use PresignedPutData; this was always PUT-based"
)]
pub type PresignedPostData = PresignedPutData;
