//! Abstraction for video file download/upload (e.g. S3).
//!
//! The API implements this trait using its S3 service. Orchestration uses
//! it to fetch source video and upload HLS outputs.

use async_trait::async_trait;

/// Storage operations for video orchestration (download source, upload HLS).
#[async_trait]
pub trait VideoStorage: Send + Sync {
    /// Download file bytes by bucket and key.
    async fn get_file(&self, bucket: &str, key: &str) -> anyhow::Result<Vec<u8>>;

    /// Upload file bytes to bucket/key with content type.
    async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> anyhow::Result<()>;
}
