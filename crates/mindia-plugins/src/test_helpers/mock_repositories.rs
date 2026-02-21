//! Mock repository implementations for testing
//!
//! These mocks allow testing plugins without database dependencies.

use anyhow::Result;
use async_trait::async_trait;
use mindia_core::models::{Audio, FileGroup, Image, Media, MediaType};
use mindia_db::{PluginFileGroupRepository, PluginMediaRepository};
use mindia_storage::Storage;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Mock media repository for testing without database
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct MockMediaRepository {
    audios: Arc<Mutex<HashMap<(Uuid, Uuid), Audio>>>,
    images: Arc<Mutex<HashMap<(Uuid, Uuid), Image>>>,
    media: Arc<Mutex<HashMap<(Uuid, Uuid), Media>>>,
    metadata: Arc<Mutex<HashMap<(Uuid, Uuid), JsonValue>>>,
    plugin_metadata: Arc<Mutex<HashMap<(Uuid, Uuid, String), JsonValue>>>,
}

impl MockMediaRepository {
    pub fn new() -> Self {
        Self {
            audios: Arc::new(Mutex::new(HashMap::new())),
            images: Arc::new(Mutex::new(HashMap::new())),
            media: Arc::new(Mutex::new(HashMap::new())),
            metadata: Arc::new(Mutex::new(HashMap::new())),
            plugin_metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_audio(&self, tenant_id: Uuid, audio: Audio) {
        self.audios
            .lock()
            .unwrap()
            .insert((tenant_id, audio.id), audio);
    }

    pub fn add_image(&self, tenant_id: Uuid, image: Image) {
        self.images
            .lock()
            .unwrap()
            .insert((tenant_id, image.id), image);
    }

    pub fn add_media(&self, tenant_id: Uuid, media: Media) {
        let media_id = media.id();
        self.media
            .lock()
            .unwrap()
            .insert((tenant_id, media_id), media);
    }

    pub fn add_metadata(&self, tenant_id: Uuid, media_id: Uuid, metadata: JsonValue) {
        self.metadata
            .lock()
            .unwrap()
            .insert((tenant_id, media_id), metadata);
    }
}

#[async_trait]
impl PluginMediaRepository for MockMediaRepository {
    async fn get_audio(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Audio>> {
        Ok(self.audios.lock().unwrap().get(&(tenant_id, id)).cloned())
    }

    async fn get_image(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Image>> {
        Ok(self.images.lock().unwrap().get(&(tenant_id, id)).cloned())
    }

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Media>> {
        Ok(self.media.lock().unwrap().get(&(tenant_id, id)).cloned())
    }

    async fn get_metadata(&self, tenant_id: Uuid, media_id: Uuid) -> Result<Option<JsonValue>> {
        Ok(self
            .metadata
            .lock()
            .unwrap()
            .get(&(tenant_id, media_id))
            .cloned())
    }

    async fn merge_plugin_metadata(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        plugin_name: &str,
        metadata: JsonValue,
    ) -> Result<Option<JsonValue>> {
        self.plugin_metadata.lock().unwrap().insert(
            (tenant_id, media_id, plugin_name.to_string()),
            metadata.clone(),
        );
        Ok(Some(metadata))
    }

    async fn create_media_entry(
        &self,
        _tenant_id: Uuid,
        _id: Uuid,
        _media_type: MediaType,
        _original_filename: String,
        _storage_key: String,
        _storage_url: String,
        _content_type: String,
        _file_size: i64,
        _width: Option<i32>,
        _height: Option<i32>,
        _duration: Option<f64>,
    ) -> Result<Media> {
        Err(anyhow::anyhow!(
            "create_media_entry is not implemented for MockMediaRepository"
        ))
    }
}

impl Default for MockMediaRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock file group repository for testing without database
#[derive(Clone)]
pub struct MockFileGroupRepository;

impl MockFileGroupRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PluginFileGroupRepository for MockFileGroupRepository {
    async fn create_group(&self, _tenant_id: Uuid, _media_ids: Vec<Uuid>) -> Result<FileGroup> {
        Err(anyhow::anyhow!(
            "create_group is not implemented for MockFileGroupRepository"
        ))
    }
}

impl Default for MockFileGroupRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a PluginContext for testing without database
pub struct TestPluginContextBuilder {
    pub tenant_id: Uuid,
    pub media_id: Uuid,
    pub config: serde_json::Value,
    pub storage: Option<Arc<dyn Storage>>,
    pub media_repo: Option<Arc<dyn PluginMediaRepository>>,
    pub file_group_repo: Option<Arc<dyn PluginFileGroupRepository>>,
}

impl TestPluginContextBuilder {
    pub fn new(tenant_id: Uuid, media_id: Uuid, config: serde_json::Value) -> Self {
        Self {
            tenant_id,
            media_id,
            config,
            storage: None,
            media_repo: None,
            file_group_repo: None,
        }
    }

    pub fn with_storage(mut self, storage: Arc<dyn Storage>) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn with_media_repo(mut self, repo: Arc<dyn PluginMediaRepository>) -> Self {
        self.media_repo = Some(repo);
        self
    }

    pub fn with_file_group_repo(mut self, repo: Arc<dyn PluginFileGroupRepository>) -> Self {
        self.file_group_repo = Some(repo);
        self
    }

    /// Build a PluginContext with mock repositories and storage
    pub fn build_with_mocks(self, storage: Arc<dyn Storage>) -> crate::plugin::PluginContext {
        let media_repo = self
            .media_repo
            .unwrap_or_else(|| Arc::new(MockMediaRepository::new()));
        let file_group_repo = self
            .file_group_repo
            .unwrap_or_else(|| Arc::new(MockFileGroupRepository::new()));

        crate::plugin::PluginContext {
            tenant_id: self.tenant_id,
            media_id: self.media_id,
            storage: self.storage.unwrap_or(storage),
            media_repo,
            file_group_repo,
            get_public_file_url: None,
            config: self.config,
        }
    }
}
