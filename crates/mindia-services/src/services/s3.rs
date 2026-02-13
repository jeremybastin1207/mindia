use bytes::Bytes;
use http::Method;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::path::Path;
use object_store::signer::Signer;
use object_store::ObjectStoreExt;
use std::env;
use std::time::Duration;

#[derive(Clone)]
pub struct S3Service {
    store: AmazonS3,
    default_bucket: Option<String>,
}

impl S3Service {
    /// Create a new S3Service with AWS client
    /// default_bucket is optional - for multi-tenancy, bucket is passed per-request
    pub async fn new(
        default_bucket: Option<String>,
        region: String,
    ) -> Result<Self, anyhow::Error> {
        // Keep AWS_REGION for compatibility with existing tooling if not already set.
        if env::var("AWS_REGION").is_err() {
            env::set_var("AWS_REGION", &region);
        }

        let mut builder = AmazonS3Builder::from_env().with_region(region.clone());
        if let Some(ref bucket) = default_bucket {
            builder = builder.with_bucket_name(bucket.clone());
        }

        let store = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build S3 object store: {}", e))?;

        Ok(S3Service {
            store,
            default_bucket,
        })
    }

    /// Upload a file to S3 with explicit bucket parameter
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
        let start = std::time::Instant::now();
        let size = data.len() as u64;

        let location = Path::from(key.to_string());
        let result = self.store.put(&location, data.into()).await;

        let duration = start.elapsed().as_secs_f64();

        match result {
            Ok(_) => {
                let url = format!("https://{}.s3.amazonaws.com/{}", bucket, key);
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

    /// Delete a file from S3 with explicit bucket parameter
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.key = %key,
        aws.s3.operation = "DeleteObject",
        s3.bucket = %bucket,
        s3.key = %key
    ))]
    pub async fn delete_file(&self, bucket: &str, key: &str) -> Result<(), anyhow::Error> {
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

    /// Get a file from S3 with explicit bucket parameter
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.key = %key,
        aws.s3.operation = "GetObject",
        s3.bucket = %bucket,
        s3.key = %key
    ))]
    pub async fn get_file(&self, bucket: &str, key: &str) -> Result<Bytes, anyhow::Error> {
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

    /// Create a new S3 bucket (for tenant provisioning)
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.operation = "CreateBucket",
        s3.bucket = %bucket
    ))]
    pub async fn create_bucket(&self, bucket: &str, region: &str) -> Result<(), anyhow::Error> {
        // Bucket provisioning is no longer handled by this service when using object_store.
        // Callers should manage bucket creation externally (e.g., via infrastructure tooling).
        Err(anyhow::anyhow!(
            "create_bucket is not supported; provision S3 buckets via infrastructure"
        ))
    }

    /// Check if a bucket exists
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.operation = "HeadBucket",
        s3.bucket = %bucket
    ))]
    pub async fn bucket_exists(&self, bucket: &str) -> Result<bool, anyhow::Error> {
        // object_store does not expose bucket-level existence checks directly.
        // Treat this as unsupported; callers should handle this via infrastructure.
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

    /// Generate a presigned PUT URL for direct S3 uploads
    ///
    /// This allows clients to upload directly to S3 without going through the API server.
    /// The URL is time-limited and tenant-scoped.
    #[tracing::instrument(skip(self), fields(s3.bucket = %bucket, s3.key = %key))]
    pub async fn generate_presigned_put_url(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        expires_in_seconds: u64,
    ) -> Result<String, anyhow::Error> {
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

    /// Generate a presigned POST URL for direct S3 uploads using POST form
    ///
    /// This is an alternative to PUT that uses POST with form data.
    /// Useful for browser-based uploads with additional form fields.
    #[tracing::instrument(skip(self), fields(s3.bucket = %bucket, s3.key = %key))]
    pub async fn generate_presigned_post_url(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        expires_in_seconds: u64,
    ) -> Result<PresignedPostData, anyhow::Error> {
        // Reuse PUT-style presigned URL; clients should use PUT.
        let url = self
            .generate_presigned_put_url(bucket, key, content_type, expires_in_seconds)
            .await?;

        tracing::info!(
            expires_in_seconds = expires_in_seconds,
            "Generated presigned POST URL (using PUT method)"
        );

        // Return as POST data structure (for compatibility with POST form uploads)
        Ok(PresignedPostData {
            url,
            fields: serde_json::json!({
                "key": key,
                "Content-Type": content_type,
            }),
        })
    }
}

/// Presigned POST form data structure
#[derive(Debug, Clone, serde::Serialize)]
pub struct PresignedPostData {
    pub url: String,
    pub fields: serde_json::Value,
}
