use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt::{Display, Formatter, Result as FmtResult};
use utoipa::ToSchema;
use uuid::Uuid;

use super::storage::StorageLocation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "processing_status", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl Display for ProcessingStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ProcessingStatus::Pending => write!(f, "pending"),
            ProcessingStatus::Processing => write!(f, "processing"),
            ProcessingStatus::Completed => write!(f, "completed"),
            ProcessingStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub storage: StorageLocation,
    pub content_type: String,
    pub file_size: i64,
    pub duration: Option<f64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub processing_status: ProcessingStatus,
    pub hls_master_playlist: Option<String>,
    pub variants: Option<JsonValue>,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Video {
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
pub struct VideoResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub duration: Option<f64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub processing_status: ProcessingStatus,
    pub hls_url: Option<String>,
    pub variants: Option<JsonValue>,
    pub uploaded_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_name: Option<String>,
}

impl From<Video> for VideoResponse {
    fn from(video: Video) -> Self {
        let hls_url = video.hls_master_playlist.as_ref().map(|playlist| {
            let base = video
                .storage
                .url
                .trim_end_matches(&video.filename)
                .trim_end_matches('/');
            format!("{}/{}", base, playlist)
        });

        VideoResponse {
            id: video.id,
            filename: video.original_filename,
            url: video.storage.url,
            content_type: video.content_type,
            file_size: video.file_size,
            duration: video.duration,
            width: video.width,
            height: video.height,
            processing_status: video.processing_status,
            hls_url,
            variants: video.variants,
            uploaded_at: video.uploaded_at,
            store_behavior: video.store_behavior,
            store_permanently: video.store_permanently,
            expires_at: video.expires_at,
            folder_id: None,
            folder_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_types::StorageBackend;

    fn test_storage_location(url: &str, key: &str) -> StorageLocation {
        StorageLocation {
            id: Uuid::new_v4(),
            backend: StorageBackend::S3,
            bucket: Some("my-bucket".to_string()),
            key: key.to_string(),
            url: url.to_string(),
        }
    }

    #[test]
    fn test_processing_status_equality() {
        assert_eq!(ProcessingStatus::Pending, ProcessingStatus::Pending);
        assert_ne!(ProcessingStatus::Pending, ProcessingStatus::Processing);
        assert_ne!(ProcessingStatus::Processing, ProcessingStatus::Completed);
        assert_ne!(ProcessingStatus::Completed, ProcessingStatus::Failed);
    }

    #[test]
    fn test_processing_status_clone() {
        let status = ProcessingStatus::Processing;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_video_response_from_video_with_hls() {
        let video_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let variants = serde_json::json!({
            "360p": {"width": 640, "height": 360, "bitrate": "800k"},
            "720p": {"width": 1280, "height": 720, "bitrate": "2500k"}
        });

        let video = Video {
            id: video_id,
            tenant_id,
            filename: "video_123.mp4".to_string(),
            original_filename: "video.mp4".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/video_123.mp4",
                "uploads/video_123.mp4",
            ),
            content_type: "video/mp4".to_string(),
            file_size: 10240000,
            duration: Some(120.5),
            width: Some(1920),
            height: Some(1080),
            processing_status: ProcessingStatus::Completed,
            hls_master_playlist: Some("hls/master.m3u8".to_string()),
            variants: Some(variants.clone()),
            uploaded_at,
            updated_at,
            store_behavior: "permanent".to_string(),
            store_permanently: true,
            expires_at: None,
        };

        let response = VideoResponse::from(video.clone());

        assert_eq!(response.id, video_id);
        assert_eq!(response.filename, "video.mp4");
        assert_eq!(
            response.url,
            "https://s3.amazonaws.com/my-bucket/uploads/video_123.mp4"
        );
        assert_eq!(response.content_type, "video/mp4");
        assert_eq!(response.file_size, 10240000);
        assert_eq!(response.duration, Some(120.5));
        assert_eq!(response.width, Some(1920));
        assert_eq!(response.height, Some(1080));
        assert_eq!(response.processing_status, ProcessingStatus::Completed);
        assert_eq!(
            response.hls_url,
            Some("https://s3.amazonaws.com/my-bucket/uploads/hls/master.m3u8".to_string())
        );
        assert_eq!(response.variants, Some(variants));
        assert!(response.store_permanently);
        assert_eq!(response.expires_at, None);
    }

    #[test]
    fn test_video_response_from_video_without_hls() {
        let video_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let video = Video {
            id: video_id,
            tenant_id,
            filename: "video_456.mp4".to_string(),
            original_filename: "video2.mp4".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/video_456.mp4",
                "uploads/video_456.mp4",
            ),
            content_type: "video/mp4".to_string(),
            file_size: 5120000,
            duration: Some(60.0),
            width: Some(1280),
            height: Some(720),
            processing_status: ProcessingStatus::Pending,
            hls_master_playlist: None,
            variants: None,
            uploaded_at,
            updated_at,
            store_behavior: "temporary".to_string(),
            store_permanently: false,
            expires_at: Some(Utc::now() + chrono::Duration::days(7)),
        };

        let response = VideoResponse::from(video.clone());

        assert_eq!(response.id, video_id);
        assert_eq!(response.filename, "video2.mp4");
        assert_eq!(response.processing_status, ProcessingStatus::Pending);
        assert_eq!(response.hls_url, None);
        assert_eq!(response.variants, None);
    }

    #[test]
    fn test_video_response_hls_url_generation_edge_cases() {
        let video_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        // Test with filename at end of storage url
        let video = Video {
            id: video_id,
            tenant_id,
            filename: "test.mp4".to_string(),
            original_filename: "test.mp4".to_string(),
            storage: test_storage_location(
                "https://example.com/uploads/test.mp4",
                "uploads/test.mp4",
            ),
            content_type: "video/mp4".to_string(),
            file_size: 1000000,
            duration: Some(30.0),
            width: Some(640),
            height: Some(480),
            processing_status: ProcessingStatus::Completed,
            hls_master_playlist: Some("hls/playlist.m3u8".to_string()),
            variants: None,
            uploaded_at,
            updated_at,
            store_behavior: "permanent".to_string(),
            store_permanently: true,
            expires_at: None,
        };

        let response = VideoResponse::from(video);
        assert_eq!(
            response.hls_url,
            Some("https://example.com/uploads/hls/playlist.m3u8".to_string())
        );
    }

    #[test]
    fn test_video_response_processing_status_processing() {
        let video_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let video = Video {
            id: video_id,
            tenant_id,
            filename: "processing.mp4".to_string(),
            original_filename: "processing.mp4".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/processing.mp4",
                "uploads/processing.mp4",
            ),
            content_type: "video/mp4".to_string(),
            file_size: 20480000,
            duration: None,
            width: None,
            height: None,
            processing_status: ProcessingStatus::Processing,
            hls_master_playlist: None,
            variants: None,
            uploaded_at,
            updated_at,
            store_behavior: "permanent".to_string(),
            store_permanently: true,
            expires_at: None,
        };

        let response = VideoResponse::from(video);
        assert_eq!(response.processing_status, ProcessingStatus::Processing);
        assert_eq!(response.duration, None);
        assert_eq!(response.width, None);
        assert_eq!(response.height, None);
    }

    #[test]
    fn test_video_response_processing_status_failed() {
        let video_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let video = Video {
            id: video_id,
            tenant_id,
            filename: "failed.mp4".to_string(),
            original_filename: "failed.mp4".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/failed.mp4",
                "uploads/failed.mp4",
            ),
            content_type: "video/mp4".to_string(),
            file_size: 15360000,
            duration: None,
            width: None,
            height: None,
            processing_status: ProcessingStatus::Failed,
            hls_master_playlist: None,
            variants: None,
            uploaded_at,
            updated_at,
            store_behavior: "temporary".to_string(),
            store_permanently: false,
            expires_at: Some(Utc::now() + chrono::Duration::days(1)),
        };

        let response = VideoResponse::from(video);
        assert_eq!(response.processing_status, ProcessingStatus::Failed);
    }
}
