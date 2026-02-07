use aws_config::meta::region::RegionProviderChain;
use aws_config::retry::{RetryConfig, RetryMode};
use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use bytes::Bytes;
use std::env;

#[derive(Clone)]
pub struct S3Service {
    client: Client,
    default_bucket: Option<String>,
}

impl S3Service {
    /// Create a new S3Service with AWS client
    /// default_bucket is optional - for multi-tenancy, bucket is passed per-request
    pub async fn new(
        default_bucket: Option<String>,
        region: String,
    ) -> Result<Self, anyhow::Error> {
        env::set_var("AWS_REGION", &region);

        let region_provider =
            RegionProviderChain::first_try(aws_config::Region::new(region.clone()));

        let retry_config = RetryConfig::standard()
            .with_max_attempts(5)
            .with_retry_mode(RetryMode::Adaptive);

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .retry_config(retry_config)
            .load()
            .await;

        let client = Client::new(&config);

        Ok(S3Service {
            client,
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

        let body = ByteStream::from(data);

        let result = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await;

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

        let result = self
            .client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await;

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

        let result = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await;

        match result {
            Ok(response) => {
                let data = response.body.collect().await?;
                let bytes = data.into_bytes();
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
        use aws_sdk_s3::types::{BucketLocationConstraint, CreateBucketConfiguration};

        let constraint = BucketLocationConstraint::from(region);
        let cfg = CreateBucketConfiguration::builder()
            .location_constraint(constraint)
            .build();

        self.client
            .create_bucket()
            .bucket(bucket)
            .create_bucket_configuration(cfg)
            .send()
            .await?;

        tracing::info!("S3 bucket created successfully");
        Ok(())
    }

    /// Check if a bucket exists
    #[tracing::instrument(skip(self), fields(
        aws.service.name = "s3",
        aws.s3.bucket = %bucket,
        aws.s3.operation = "HeadBucket",
        s3.bucket = %bucket
    ))]
    pub async fn bucket_exists(&self, bucket: &str) -> Result<bool, anyhow::Error> {
        match self.client.head_bucket().bucket(bucket).send().await {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("404") || err_str.contains("NotFound") {
                    Ok(false)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Get the default bucket
    pub fn default_bucket(&self) -> Option<&str> {
        self.default_bucket.as_deref()
    }

    /// Get the S3 client (for advanced operations like multipart uploads)
    pub fn client(&self) -> &Client {
        &self.client
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
        use aws_sdk_s3::presigning::PresigningConfig;
        use std::time::Duration;

        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in_seconds))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create presigning config: {}", e))?;

        let presigned_request = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate presigned URL: {}", e))?;

        let url = presigned_request.uri().to_string();

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
        use aws_sdk_s3::presigning::PresigningConfig;
        use std::time::Duration;

        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in_seconds))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create presigning config: {}", e))?;

        // For POST, we need to create a presigned POST request
        // AWS SDK doesn't have direct POST presigning, so we'll use PUT for now
        // and document that clients should use PUT method
        let presigned_request = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate presigned URL: {}", e))?;

        let url = presigned_request.uri().to_string();

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
