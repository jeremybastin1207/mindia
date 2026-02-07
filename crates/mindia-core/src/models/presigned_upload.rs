use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Request to generate a presigned URL for direct S3 upload
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct PresignedUploadRequest {
    /// Original filename
    #[validate(length(
        min = 1,
        max = 255,
        message = "Filename must be between 1 and 255 characters"
    ))]
    pub filename: String,
    /// Content type (MIME type)
    #[validate(length(
        min = 1,
        max = 255,
        message = "Content type must be between 1 and 255 characters"
    ))]
    pub content_type: String,
    /// File size in bytes
    #[validate(range(min = 1, message = "File size must be at least 1 byte"))]
    pub file_size: u64,
    /// Media type (image, video, audio, document)
    #[validate(length(
        min = 1,
        max = 50,
        message = "Media type must be between 1 and 50 characters"
    ))]
    pub media_type: String,
    /// Storage behavior: "0" (temporary), "1" (permanent), "auto"
    #[serde(default = "default_store")]
    #[validate(length(
        min = 1,
        max = 10,
        message = "Store value must be between 1 and 10 characters"
    ))]
    pub store: String,
    /// Optional custom metadata (key-value pairs)
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_store() -> String {
    "auto".to_string()
}

/// Response containing presigned URL and upload information
#[derive(Debug, Serialize, ToSchema)]
pub struct PresignedUploadResponse {
    /// Upload ID (used to complete the upload)
    pub upload_id: Uuid,
    /// Presigned URL for direct S3 upload
    pub presigned_url: String,
    /// S3 key where the file will be stored
    pub s3_key: String,
    /// URL expiration time
    pub expires_at: DateTime<Utc>,
    /// Fields to include in POST request (for POST form uploads)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
}

/// Request to complete a direct upload
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteUploadRequest {
    /// Upload ID from presigned URL response
    pub upload_id: Uuid,
    /// Optional metadata to update after upload
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Response after completing upload
#[derive(Debug, Serialize, ToSchema)]
pub struct CompleteUploadResponse {
    /// Media ID (created after upload completion)
    pub id: Uuid,
    /// File URL
    pub url: String,
    /// Content type
    pub content_type: String,
    /// File size
    pub file_size: i64,
    /// Upload timestamp
    pub uploaded_at: DateTime<Utc>,
}
