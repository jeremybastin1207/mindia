use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::storage::StorageLocation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub storage: StorageLocation,
    pub content_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Image {
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

#[derive(Debug, Serialize, ToSchema)]
pub struct ImageResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub uploaded_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_name: Option<String>,
}

impl From<Image> for ImageResponse {
    fn from(image: Image) -> Self {
        ImageResponse {
            id: image.id,
            filename: image.original_filename,
            url: image.storage.url,
            content_type: image.content_type,
            file_size: image.file_size,
            width: image.width,
            height: image.height,
            uploaded_at: image.uploaded_at,
            store_behavior: image.store_behavior,
            store_permanently: image.store_permanently,
            expires_at: image.expires_at,
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
    fn test_image_response_from_image() {
        let image_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();
        let expires_at = Some(Utc::now() + chrono::Duration::days(7));

        let image = Image {
            id: image_id,
            tenant_id,
            filename: "test_123.jpg".to_string(),
            original_filename: "test.jpg".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/test_123.jpg",
                "uploads/test_123.jpg",
            ),
            content_type: "image/jpeg".to_string(),
            file_size: 1024000,
            width: Some(1920),
            height: Some(1080),
            uploaded_at,
            updated_at,
            store_behavior: "temporary".to_string(),
            store_permanently: false,
            expires_at,
        };

        let response = ImageResponse::from(image.clone());

        assert_eq!(response.id, image_id);
        assert_eq!(response.filename, "test.jpg");
        assert_eq!(
            response.url,
            "https://s3.amazonaws.com/my-bucket/uploads/test_123.jpg"
        );
        assert_eq!(response.content_type, "image/jpeg");
        assert_eq!(response.file_size, 1024000);
        assert_eq!(response.width, Some(1920));
        assert_eq!(response.height, Some(1080));
        assert_eq!(response.uploaded_at, uploaded_at);
        assert_eq!(response.store_behavior, "temporary");
        assert!(!response.store_permanently);
        assert_eq!(response.expires_at, expires_at);
    }

    #[test]
    fn test_image_response_from_image_permanent() {
        let image_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let image = Image {
            id: image_id,
            tenant_id,
            filename: "permanent_456.png".to_string(),
            original_filename: "permanent.png".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/permanent_456.png",
                "uploads/permanent_456.png",
            ),
            content_type: "image/png".to_string(),
            file_size: 2048000,
            width: Some(3840),
            height: Some(2160),
            uploaded_at,
            updated_at,
            store_behavior: "permanent".to_string(),
            store_permanently: true,
            expires_at: None,
        };

        let response = ImageResponse::from(image.clone());

        assert_eq!(response.id, image_id);
        assert_eq!(response.filename, "permanent.png");
        assert!(response.store_permanently);
        assert_eq!(response.expires_at, None);
    }

    #[test]
    fn test_image_response_from_image_no_dimensions() {
        let image_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let image = Image {
            id: image_id,
            tenant_id,
            filename: "nodims_789.jpg".to_string(),
            original_filename: "nodims.jpg".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/nodims_789.jpg",
                "uploads/nodims_789.jpg",
            ),
            content_type: "image/jpeg".to_string(),
            file_size: 512000,
            width: None,
            height: None,
            uploaded_at,
            updated_at,
            store_behavior: "temporary".to_string(),
            store_permanently: false,
            expires_at: None,
        };

        let response = ImageResponse::from(image.clone());

        assert_eq!(response.width, None);
        assert_eq!(response.height, None);
    }
}
