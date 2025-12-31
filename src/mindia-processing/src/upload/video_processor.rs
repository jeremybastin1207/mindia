//! Video upload processor (minimal metadata at upload; full metadata during transcode).

use async_trait::async_trait;

use crate::upload::traits::UploadProcessor;

/// Video metadata at upload (full metadata from transcode).
#[derive(Clone, Debug)]
pub struct UploadVideoMetadata {}

/// Video upload processor.
pub struct VideoUploadProcessor;

impl Default for VideoUploadProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoUploadProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UploadProcessor for VideoUploadProcessor {
    type Metadata = UploadVideoMetadata;

    async fn extract_metadata(&self, _data: &[u8]) -> anyhow::Result<UploadVideoMetadata> {
        Ok(UploadVideoMetadata {})
    }

    async fn sanitize(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        Ok(data)
    }
}
