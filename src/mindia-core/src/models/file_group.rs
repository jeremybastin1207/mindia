use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// File group entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGroup {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// File group item (represents a file in a group)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGroupItem {
    pub group_id: Uuid,
    pub media_id: Uuid,
    pub index: i32,
    pub created_at: DateTime<Utc>,
}

/// Request to create a file group
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFileGroupRequest {
    /// Array of media UUIDs to include in the group
    #[serde(rename = "files")]
    pub files: Vec<Uuid>,
}

/// File group info (metadata only)
#[derive(Debug, Serialize, ToSchema)]
pub struct FileGroupInfo {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub file_count: i64,
}

/// File item in a group response
#[derive(Debug, Serialize, ToSchema)]
pub struct FileGroupFileItem {
    pub id: Uuid,
    pub url: String,
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub media_type: String,
}

/// Full file group response (includes list of files)
#[derive(Debug, Serialize, ToSchema)]
pub struct FileGroupResponse {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub files: Vec<FileGroupFileItem>,
}
