//! VideoStorage implementations: S3-only and generic (Storage trait).

use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

use mindia_processing::VideoStorage;
use mindia_storage::Storage;

use crate::state::S3Config;

/// S3-backed VideoStorage for use by VideoOrchestrator (legacy; prefer GenericVideoStorage).
#[allow(dead_code)]
#[derive(Clone)]
pub struct S3VideoStorage {
    s3: Arc<S3Config>,
}

#[allow(dead_code)]
impl S3VideoStorage {
    pub fn new(s3: Arc<S3Config>) -> Self {
        Self { s3 }
    }
}

#[async_trait]
impl VideoStorage for S3VideoStorage {
    async fn get_file(&self, bucket: &str, key: &str) -> anyhow::Result<Vec<u8>> {
        let data = self
            .s3
            .service
            .get_file(bucket, key)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(data.to_vec())
    }

    async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> anyhow::Result<()> {
        self.s3
            .service
            .upload_file(bucket, key, Bytes::from(data), content_type)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }
}

/// Generic VideoStorage that uses the Storage trait (works with both local and S3 backends).
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
