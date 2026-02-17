use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;
use uuid::Uuid;

#[cfg(feature = "sqlx")]
use sqlx::FromRow;

use super::storage::StorageLocation;
// Import actual domain models (for API response building)
use super::audio::Audio;
use super::document::Document;
use super::image::Image;
use super::video::Video;

// Re-export ProcessingStatus for convenience
pub use super::video::ProcessingStatus;

/// Media type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "media_type", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Document,
}

/// Unified media record (no storage details; use StorageLocation separately).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub media_type: MediaType,
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub folder_id: Option<Uuid>,
    pub metadata: Option<JsonValue>,
}

/// Type-specific metadata (stored in media.type_metadata JSONB).
/// Note: For API responses, this uses internal tagging with "kind" field.
/// For database storage, use `to_json_value()`/`from_json_value()` which handle flat JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TypeMetadata {
    Image {
        width: Option<i32>,
        height: Option<i32>,
    },
    Video {
        width: Option<i32>,
        height: Option<i32>,
        duration: Option<f64>,
        processing_status: Option<ProcessingStatus>,
        hls_master_playlist: Option<String>,
        variants: Option<JsonValue>,
    },
    Audio {
        duration: Option<f64>,
        bitrate: Option<i32>,
        sample_rate: Option<i32>,
        channels: Option<i32>,
    },
    Document {
        page_count: Option<i32>,
    },
}

/// Internal flat structs for database JSONB serialization (no discriminator tag).
/// These mirror TypeMetadata variants but serialize without the "kind" field.
mod flat_metadata {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub struct ImageMetadata {
        pub width: Option<i32>,
        pub height: Option<i32>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct VideoMetadata {
        pub width: Option<i32>,
        pub height: Option<i32>,
        pub duration: Option<f64>,
        pub processing_status: Option<ProcessingStatus>,
        pub hls_master_playlist: Option<String>,
        pub variants: Option<JsonValue>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct AudioMetadata {
        pub duration: Option<f64>,
        pub bitrate: Option<i32>,
        pub sample_rate: Option<i32>,
        pub channels: Option<i32>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct DocumentMetadata {
        pub page_count: Option<i32>,
    }
}

impl TypeMetadata {
    /// Parse type_metadata JSONB using media_type to select the variant.
    /// Uses serde for field parsing, ensuring consistency with struct definitions.
    pub fn from_json_value(v: &JsonValue, media_type: MediaType) -> Option<TypeMetadata> {
        match media_type {
            MediaType::Image => {
                let flat: flat_metadata::ImageMetadata = serde_json::from_value(v.clone()).ok()?;
                Some(TypeMetadata::Image {
                    width: flat.width,
                    height: flat.height,
                })
            }
            MediaType::Video => {
                let flat: flat_metadata::VideoMetadata = serde_json::from_value(v.clone()).ok()?;
                Some(TypeMetadata::Video {
                    width: flat.width,
                    height: flat.height,
                    duration: flat.duration,
                    processing_status: flat.processing_status,
                    hls_master_playlist: flat.hls_master_playlist,
                    variants: flat.variants,
                })
            }
            MediaType::Audio => {
                let flat: flat_metadata::AudioMetadata = serde_json::from_value(v.clone()).ok()?;
                Some(TypeMetadata::Audio {
                    duration: flat.duration,
                    bitrate: flat.bitrate,
                    sample_rate: flat.sample_rate,
                    channels: flat.channels,
                })
            }
            MediaType::Document => {
                let flat: flat_metadata::DocumentMetadata =
                    serde_json::from_value(v.clone()).ok()?;
                Some(TypeMetadata::Document {
                    page_count: flat.page_count,
                })
            }
        }
    }

    /// Serialize to JSONB for insert/update (flat JSON without discriminator tag).
    /// Uses serde for serialization, ensuring consistency with struct definitions.
    pub fn to_json_value(&self) -> JsonValue {
        match self {
            TypeMetadata::Image { width, height } => {
                let flat = flat_metadata::ImageMetadata {
                    width: *width,
                    height: *height,
                };
                serde_json::to_value(flat).unwrap_or_default()
            }
            TypeMetadata::Video {
                width,
                height,
                duration,
                processing_status,
                hls_master_playlist,
                variants,
            } => {
                let flat = flat_metadata::VideoMetadata {
                    width: *width,
                    height: *height,
                    duration: *duration,
                    processing_status: processing_status.clone(),
                    hls_master_playlist: hls_master_playlist.clone(),
                    variants: variants.clone(),
                };
                serde_json::to_value(flat).unwrap_or_default()
            }
            TypeMetadata::Audio {
                duration,
                bitrate,
                sample_rate,
                channels,
            } => {
                let flat = flat_metadata::AudioMetadata {
                    duration: *duration,
                    bitrate: *bitrate,
                    sample_rate: *sample_rate,
                    channels: *channels,
                };
                serde_json::to_value(flat).unwrap_or_default()
            }
            TypeMetadata::Document { page_count } => {
                let flat = flat_metadata::DocumentMetadata {
                    page_count: *page_count,
                };
                serde_json::to_value(flat).unwrap_or_default()
            }
        }
    }
}

/// Polymorphic media enum (Image, Video, Audio, Document) for response building.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "media_type", rename_all = "lowercase")]
pub enum Media {
    Image(Image),
    Video(Video),
    Audio(Audio),
    Document(Document),
}

impl Media {
    pub fn id(&self) -> Uuid {
        match self {
            Media::Image(i) => i.id,
            Media::Video(v) => v.id,
            Media::Audio(a) => a.id,
            Media::Document(d) => d.id,
        }
    }

    pub fn tenant_id(&self) -> Uuid {
        match self {
            Media::Image(i) => i.tenant_id,
            Media::Video(v) => v.tenant_id,
            Media::Audio(a) => a.tenant_id,
            Media::Document(d) => d.tenant_id,
        }
    }

    pub fn media_type(&self) -> MediaType {
        match self {
            Media::Image(_) => MediaType::Image,
            Media::Video(_) => MediaType::Video,
            Media::Audio(_) => MediaType::Audio,
            Media::Document(_) => MediaType::Document,
        }
    }
}

/// Database row for media table (storage_id + type_metadata; storage is in storage_locations).
#[derive(Debug)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct MediaRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub storage_id: Uuid,
    pub media_type: MediaType,
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub folder_id: Option<Uuid>,
    pub metadata: Option<JsonValue>,
    pub type_metadata: Option<JsonValue>,
}

impl MediaRow {
    /// Build MediaItem (no storage) from this row.
    pub fn to_media_item(&self) -> MediaItem {
        MediaItem {
            id: self.id,
            tenant_id: self.tenant_id,
            media_type: self.media_type,
            filename: self.filename.clone(),
            original_filename: self.original_filename.clone(),
            content_type: self.content_type.clone(),
            file_size: self.file_size,
            uploaded_at: self.uploaded_at,
            updated_at: self.updated_at,
            store_behavior: self.store_behavior.clone(),
            store_permanently: self.store_permanently,
            expires_at: self.expires_at,
            folder_id: self.folder_id,
            metadata: self.metadata.clone(),
        }
    }

    /// Parse type_metadata JSONB into TypeMetadata.
    pub fn type_metadata_parsed(&self) -> TypeMetadata {
        let empty = &JsonValue::Object(serde_json::Map::new());
        let v = self.type_metadata.as_ref().unwrap_or(empty);
        TypeMetadata::from_json_value(v, self.media_type).unwrap_or(match self.media_type {
            MediaType::Image => TypeMetadata::Image {
                width: None,
                height: None,
            },
            MediaType::Video => TypeMetadata::Video {
                width: None,
                height: None,
                duration: None,
                processing_status: None,
                hls_master_playlist: None,
                variants: None,
            },
            MediaType::Audio => TypeMetadata::Audio {
                duration: None,
                bitrate: None,
                sample_rate: None,
                channels: None,
            },
            MediaType::Document => TypeMetadata::Document { page_count: None },
        })
    }
}

/// Build Image response DTO from MediaItem + StorageLocation + TypeMetadata.
pub fn to_image(item: &MediaItem, storage: &StorageLocation, meta: &TypeMetadata) -> Image {
    let (width, height) = match meta {
        TypeMetadata::Image { width, height } => (*width, *height),
        _ => (None, None),
    };
    Image {
        id: item.id,
        tenant_id: item.tenant_id,
        filename: item.filename.clone(),
        original_filename: item.original_filename.clone(),
        storage: storage.clone(),
        content_type: item.content_type.clone(),
        file_size: item.file_size,
        width,
        height,
        uploaded_at: item.uploaded_at,
        updated_at: item.updated_at,
        store_behavior: item.store_behavior.clone(),
        store_permanently: item.store_permanently,
        expires_at: item.expires_at,
    }
}

/// Build Video response DTO from MediaItem + StorageLocation + TypeMetadata.
pub fn to_video(item: &MediaItem, storage: &StorageLocation, meta: &TypeMetadata) -> Video {
    let (width, height, duration, processing_status, hls_master_playlist, variants) = match meta {
        TypeMetadata::Video {
            width,
            height,
            duration,
            processing_status,
            hls_master_playlist,
            variants,
        } => (
            *width,
            *height,
            *duration,
            processing_status
                .clone()
                .unwrap_or(ProcessingStatus::Pending),
            hls_master_playlist.clone(),
            variants.clone(),
        ),
        _ => (None, None, None, ProcessingStatus::Pending, None, None),
    };
    Video {
        id: item.id,
        tenant_id: item.tenant_id,
        filename: item.filename.clone(),
        original_filename: item.original_filename.clone(),
        storage: storage.clone(),
        content_type: item.content_type.clone(),
        file_size: item.file_size,
        width,
        height,
        duration,
        processing_status,
        hls_master_playlist,
        variants,
        uploaded_at: item.uploaded_at,
        updated_at: item.updated_at,
        store_behavior: item.store_behavior.clone(),
        store_permanently: item.store_permanently,
        expires_at: item.expires_at,
    }
}

/// Build Audio response DTO from MediaItem + StorageLocation + TypeMetadata.
pub fn to_audio(item: &MediaItem, storage: &StorageLocation, meta: &TypeMetadata) -> Audio {
    let (duration, bitrate, sample_rate, channels) = match meta {
        TypeMetadata::Audio {
            duration,
            bitrate,
            sample_rate,
            channels,
        } => (*duration, *bitrate, *sample_rate, *channels),
        _ => (None, None, None, None),
    };
    Audio {
        id: item.id,
        tenant_id: item.tenant_id,
        filename: item.filename.clone(),
        original_filename: item.original_filename.clone(),
        storage: storage.clone(),
        content_type: item.content_type.clone(),
        file_size: item.file_size,
        duration,
        bitrate,
        sample_rate,
        channels,
        uploaded_at: item.uploaded_at,
        updated_at: item.updated_at,
        store_behavior: item.store_behavior.clone(),
        store_permanently: item.store_permanently,
        expires_at: item.expires_at,
    }
}

/// Build Document response DTO from MediaItem + StorageLocation + TypeMetadata.
pub fn to_document(item: &MediaItem, storage: &StorageLocation, meta: &TypeMetadata) -> Document {
    let page_count = match meta {
        TypeMetadata::Document { page_count } => *page_count,
        _ => None,
    };
    Document {
        id: item.id,
        tenant_id: item.tenant_id,
        filename: item.filename.clone(),
        original_filename: item.original_filename.clone(),
        storage: storage.clone(),
        content_type: item.content_type.clone(),
        file_size: item.file_size,
        page_count,
        uploaded_at: item.uploaded_at,
        updated_at: item.updated_at,
        store_behavior: item.store_behavior.clone(),
        store_permanently: item.store_permanently,
        expires_at: item.expires_at,
    }
}
