//! VideoStorage implementation using the unified Storage trait.

use async_trait::async_trait;
use std::sync::Arc;

use mindia_processing::VideoStorage;
use mindia_storage::Storage;

/// VideoStorage that uses the Storage trait (works with both local and S3 backends).
#[derive(Clone)]
pub struct GenericVideoStorage {
    storage: Arc<dyn Storage>,
}

impl GenericVideoStorage {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl VideoStorage for GenericVideoStorage {
    async fn get_file(&self, _bucket: &str, key: &str) -> anyhow::Result<Vec<u8>> {
        self.storage
            .download(key)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn upload_file(
        &self,
        _bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> anyhow::Result<()> {
        self.storage
            .upload_with_key(key, data, content_type)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }
}
