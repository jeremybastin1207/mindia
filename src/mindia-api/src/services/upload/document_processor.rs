//! Document processor implementation

use async_trait::async_trait;
use mindia_core::AppError;

use super::traits::MediaProcessor;

/// Document metadata (empty for now)
pub struct DocumentMetadata {
    // Document metadata could include page count, etc., but it's not extracted at upload time
}

/// Document processor for handling document-specific operations
pub struct DocumentProcessorImpl;

impl DocumentProcessorImpl {
    /// Create a new document processor
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MediaProcessor for DocumentProcessorImpl {
    type Metadata = DocumentMetadata;

    /// Extract document metadata
    ///
    /// For documents, metadata extraction is not performed at upload time.
    async fn extract_metadata(&self, _data: &[u8]) -> Result<Self::Metadata, AppError> {
        Ok(DocumentMetadata {})
    }

    /// Sanitize document (no-op for document files)
    ///
    /// Documents don't require sanitization.
    async fn sanitize(&self, data: Vec<u8>) -> Result<Vec<u8>, AppError> {
        Ok(data)
    }
}
