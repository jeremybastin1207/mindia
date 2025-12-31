//! Traits for the upload pipeline.

use async_trait::async_trait;

/// Configuration for a media type in the upload pipeline.
pub trait UploadConfig: Send + Sync {
    fn max_file_size(&self) -> usize;
    fn allowed_extensions(&self) -> &[String];
    fn allowed_content_types(&self) -> &[String];
    fn media_type_name(&self) -> &'static str;
}

/// Media-specific processing (extract metadata, sanitize).
#[async_trait]
pub trait UploadProcessor: Send + Sync {
    type Metadata: Send;

    async fn extract_metadata(&self, data: &[u8]) -> anyhow::Result<Self::Metadata>;
    async fn sanitize(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>>;
}

/// Optional virus scanner (e.g. ClamAV). Implemented by the API.
#[async_trait]
pub trait VirusScanner: Send + Sync {
    async fn scan(&self, data: &[u8]) -> anyhow::Result<()>;
}
