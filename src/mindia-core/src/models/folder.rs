use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Folder model for organizing media files hierarchically
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Folder {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Folder response with additional metadata
#[derive(Debug, Serialize, ToSchema)]
pub struct FolderResponse {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub media_count: Option<i64>, // Optional: count of media items in this folder
    pub subfolder_count: Option<i64>, // Optional: count of subfolders
}

/// Request DTO for creating a new folder
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateFolderRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Folder name must be between 1 and 255 characters"
    ))]
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
}

/// Request DTO for updating a folder
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateFolderRequest {
    #[serde(default)]
    #[validate(length(
        min = 1,
        max = 255,
        message = "Folder name must be between 1 and 255 characters"
    ))]
    pub name: Option<String>,
    #[serde(default)]
    pub parent_id: Option<Option<Uuid>>, // Option<Option> to distinguish between None (no change) and Some(None) (set to root)
}

/// Folder tree node for hierarchical representation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FolderTreeNode {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub media_count: Option<i64>,
    #[serde(default)]
    pub children: Vec<FolderTreeNode>,
}

impl From<Folder> for FolderResponse {
    fn from(folder: Folder) -> Self {
        FolderResponse {
            id: folder.id,
            name: folder.name,
            parent_id: folder.parent_id,
            created_at: folder.created_at,
            updated_at: folder.updated_at,
            media_count: None,
            subfolder_count: None,
        }
    }
}

impl Folder {
    /// Create a folder response with counts
    pub fn to_response_with_counts(
        self,
        media_count: Option<i64>,
        subfolder_count: Option<i64>,
    ) -> FolderResponse {
        FolderResponse {
            id: self.id,
            name: self.name,
            parent_id: self.parent_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            media_count,
            subfolder_count,
        }
    }
}
