//! Types for the upload pipeline.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Validated file input (before scan/process/store).
#[derive(Clone, Debug)]
pub struct ValidatedFile {
    pub data: Vec<u8>,
    pub original_filename: String,
    pub content_type: String,
    pub extension: String,
}

/// Data produced by the upload pipeline for entity creation.
#[derive(Clone, Debug)]
pub struct UploadData {
    pub tenant_id: Uuid,
    pub file_id: Uuid,
    pub uuid_filename: String,
    pub safe_original_filename: String,
    pub storage_key: String,
    pub bucket: String,
    pub storage_url: String,
    pub content_type: String,
    pub file_size: i64,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
}
