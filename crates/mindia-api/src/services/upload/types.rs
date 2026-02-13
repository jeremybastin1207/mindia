//! Types used by the media upload service

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

/// Extracted and validated file data
pub struct ValidatedFile {
    pub data: Vec<u8>,
    pub original_filename: String,
    pub content_type: String,
    pub extension: String,
}

/// Storage information for uploaded file
#[allow(dead_code)]
pub struct UploadedFile {
    pub storage_key: String,
    pub storage_url: String,
    pub file_size: usize,
}

/// Result of a successful upload operation
///
/// Contains both the database entity and storage information
#[allow(dead_code)]
pub struct UploadResult<E> {
    /// The created entity (Image, Audio, Video, or Document)
    pub entity: E,
    /// Storage information
    pub uploaded_file: UploadedFile,
}

/// Data needed to create an entity after upload
///
/// This struct contains all the information needed to create a database entity
/// after a file has been uploaded to storage. This avoids the need for complex
/// closure patterns that cause type inference issues with Axum.
#[derive(Clone)]
pub struct UploadData {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Generated file UUID
    pub file_id: Uuid,
    /// UUID-based filename (e.g., "uuid.jpg")
    pub uuid_filename: String,
    /// Sanitized original filename
    pub safe_original_filename: String,
    /// Storage key
    pub storage_key: String,
    /// Storage URL
    pub storage_url: String,
    /// Content type
    pub content_type: String,
    /// File size in bytes
    pub file_size: i64,
    /// Store behavior string
    pub store_behavior: String,
    /// Whether to store permanently
    pub store_permanently: bool,
    /// Expiration time if not permanent
    pub expires_at: Option<DateTime<Utc>>,
}
