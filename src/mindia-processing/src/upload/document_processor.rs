//! Document upload processor (minimal metadata at upload).

use async_trait::async_trait;

use crate::upload::traits::UploadProcessor;

/// Document metadata at upload.
#[derive(Clone, Debug)]
pub struct UploadDocumentMetadata {}

/// Document upload processor.
pub struct DocumentUploadProcessor;

impl Default for DocumentUploadProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentUploadProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UploadProcessor for DocumentUploadProcessor {
    type Metadata = UploadDocumentMetadata;

    async fn extract_metadata(&self, _data: &[u8]) -> anyhow::Result<UploadDocumentMetadata> {
        Ok(UploadDocumentMetadata {})
    }

    async fn sanitize(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        Ok(data)
    }
}
