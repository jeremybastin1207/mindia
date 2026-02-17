use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::storage::StorageLocation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub storage: StorageLocation,
    pub content_type: String,
    pub file_size: i64,
    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Audio {
    pub fn storage_key(&self) -> &str {
        &self.storage.key
    }
    pub fn storage_url(&self) -> &str {
        &self.storage.url
    }
    pub fn storage_bucket(&self) -> Option<&str> {
        self.storage.bucket.as_deref()
    }
    /// Backend type (S3, Local, NFS) for this file's storage location.
    pub fn storage_backend(&self) -> crate::StorageBackend {
        self.storage.backend
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AudioResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<i32>,
    pub uploaded_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_name: Option<String>,
}

impl From<Audio> for AudioResponse {
    fn from(audio: Audio) -> Self {
        AudioResponse {
            id: audio.id,
            filename: audio.original_filename,
            url: audio.storage.url,
            content_type: audio.content_type,
            file_size: audio.file_size,
            duration: audio.duration,
            bitrate: audio.bitrate,
            sample_rate: audio.sample_rate,
            channels: audio.channels,
            uploaded_at: audio.uploaded_at,
            store_behavior: audio.store_behavior,
            store_permanently: audio.store_permanently,
            expires_at: audio.expires_at,
            folder_id: None,
            folder_name: None,
        }
    }
}
