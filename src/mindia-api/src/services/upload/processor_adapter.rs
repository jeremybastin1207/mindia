//! Adapters from API traits to processing pipeline types.

use async_trait::async_trait;
use std::sync::Arc;

use mindia_core::AppError;
use mindia_processing::{UploadConfig as ProcessingUploadConfig, UploadProcessor};

use super::traits::MediaProcessor;
use super::traits::MediaUploadConfig;

/// Wraps MediaUploadConfig and implements processing's UploadConfig.
#[allow(dead_code)]
pub struct UploadConfigAdapter<'a>(pub &'a dyn MediaUploadConfig);

impl ProcessingUploadConfig for UploadConfigAdapter<'_> {
    fn max_file_size(&self) -> usize {
        self.0.max_file_size()
    }
    fn allowed_extensions(&self) -> &[String] {
        self.0.allowed_extensions()
    }
    fn allowed_content_types(&self) -> &[String] {
        self.0.allowed_content_types()
    }
    fn media_type_name(&self) -> &'static str {
        self.0.media_type_name()
    }
}

/// Wraps a MediaProcessor and implements UploadProcessor.
#[allow(dead_code)]
pub struct ProcessorAdapter<M> {
    inner: Arc<dyn MediaProcessor<Metadata = M> + Send + Sync>,
}

impl<M> ProcessorAdapter<M> {
    #[allow(dead_code)]
    pub fn new(inner: Box<dyn MediaProcessor<Metadata = M> + Send + Sync + 'static>) -> Self {
        Self {
            inner: Arc::from(inner),
        }
    }
}

#[async_trait]
impl<M: Send + 'static> UploadProcessor for ProcessorAdapter<M> {
    type Metadata = M;

    async fn extract_metadata(&self, data: &[u8]) -> anyhow::Result<M> {
        self.inner
            .extract_metadata(data)
            .await
            .map_err(|e: AppError| anyhow::anyhow!("{}", e))
    }

    async fn sanitize(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        self.inner
            .sanitize(data)
            .await
            .map_err(|e: AppError| anyhow::anyhow!("{}", e))
    }
}
