//! Audio upload processor (minimal metadata at upload).

use async_trait::async_trait;

use crate::upload::traits::UploadProcessor;

/// Audio metadata at upload.
#[derive(Clone, Debug)]
pub struct UploadAudioMetadata {}

/// Audio upload processor.
pub struct AudioUploadProcessor;

impl Default for AudioUploadProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioUploadProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UploadProcessor for AudioUploadProcessor {
    type Metadata = UploadAudioMetadata;

    async fn extract_metadata(&self, _data: &[u8]) -> anyhow::Result<UploadAudioMetadata> {
        Ok(UploadAudioMetadata {})
    }

    async fn sanitize(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        Ok(data)
    }
}
