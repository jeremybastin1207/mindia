//! Storage location model: backend-agnostic reference to where a file is stored.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage_types::StorageBackend;

/// A reference to a file's physical location (S3, local, NFS, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLocation {
    pub id: Uuid,
    pub backend: StorageBackend,
    pub bucket: Option<String>,
    pub key: String,
    pub url: String,
}
