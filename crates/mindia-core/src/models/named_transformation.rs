//! Named transformation models for reusable transformation presets

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Named transformation model for storing reusable transformation presets
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NamedTransformation {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub operations: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Named transformation response
#[derive(Debug, Serialize, ToSchema)]
pub struct NamedTransformationResponse {
    /// Unique identifier for the preset
    pub id: Uuid,
    /// Name of the preset (used in URLs as `-/preset/{name}/`)
    pub name: String,
    /// The transformation operations string (e.g., `-/resize/500x/-/format/webp/`)
    pub operations: String,
    /// Optional description of what this preset does
    pub description: Option<String>,
    /// When the preset was created
    pub created_at: DateTime<Utc>,
    /// When the preset was last updated
    pub updated_at: DateTime<Utc>,
}

/// Request DTO for creating a new named transformation
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateNamedTransformationRequest {
    /// Name of the preset (alphanumeric, hyphens, underscores; 1-100 chars)
    pub name: String,
    /// The transformation operations string (e.g., `-/resize/500x/-/format/webp/`)
    pub operations: String,
    /// Optional description of what this preset does
    #[serde(default)]
    pub description: Option<String>,
}

/// Request DTO for updating a named transformation
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateNamedTransformationRequest {
    /// New operations string (optional)
    #[serde(default)]
    pub operations: Option<String>,
    /// New description (optional, use null to clear)
    #[serde(default)]
    pub description: Option<Option<String>>,
}

impl From<NamedTransformation> for NamedTransformationResponse {
    fn from(nt: NamedTransformation) -> Self {
        NamedTransformationResponse {
            id: nt.id,
            name: nt.name,
            operations: nt.operations,
            description: nt.description,
            created_at: nt.created_at,
            updated_at: nt.updated_at,
        }
    }
}

/// Validation helpers for named transformations
impl CreateNamedTransformationRequest {
    /// Validate the preset name
    pub fn validate_name(name: &str) -> Result<(), String> {
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err("Preset name cannot be empty".to_string());
        }

        if trimmed.len() > 100 {
            return Err("Preset name cannot exceed 100 characters".to_string());
        }

        // Only allow alphanumeric, hyphens, and underscores
        if !trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(
                "Preset name can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            );
        }

        // Name cannot start with a number (to avoid confusion with image IDs)
        if trimmed
            .chars()
            .next()
            .map(|c| c.is_numeric())
            .unwrap_or(false)
        {
            return Err("Preset name cannot start with a number".to_string());
        }

        Ok(())
    }

    /// Validate that operations don't contain preset references (no recursion)
    pub fn validate_no_preset_reference(operations: &str) -> Result<(), String> {
        // Check for preset references in operations string
        if operations.contains("preset/") || operations.contains("preset\\") {
            return Err(
                "Operations cannot reference other presets (no recursion allowed)".to_string(),
            );
        }
        Ok(())
    }
}
