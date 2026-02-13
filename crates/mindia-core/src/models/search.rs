use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type, ToSchema)]
#[sqlx(type_name = "entity_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Image,
    Video,
    Document,
    Audio,
}

impl Display for EntityType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            EntityType::Image => write!(f, "image"),
            EntityType::Video => write!(f, "video"),
            EntityType::Document => write!(f, "document"),
            EntityType::Audio => write!(f, "audio"),
        }
    }
}

impl FromStr for EntityType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "image" => Ok(EntityType::Image),
            "video" => Ok(EntityType::Video),
            "document" => Ok(EntityType::Document),
            "audio" => Ok(EntityType::Audio),
            _ => Err(anyhow::anyhow!("Invalid entity type: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Embedding {
    pub id: i32,
    pub tenant_id: Uuid,
    pub entity_id: Uuid,
    pub entity_type: EntityType,
    pub description: String,
    #[cfg(feature = "semantic-search")]
    pub embedding: pgvector::Vector,
    #[cfg(not(feature = "semantic-search"))]
    pub embedding: Vec<f32>, // Fallback when pgvector is not available
    pub model_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Embedding {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Embedding {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            entity_id: row.get("entity_id"),
            entity_type: row.get("entity_type"),
            description: row.get("description"),
            embedding: row.get("embedding"),
            model_name: row.get("model_name"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResult {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub filename: String,
    pub url: String,
    pub description: String,
    pub similarity_score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
}

#[derive(Debug, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[serde(default)]
pub struct SearchQuery {
    /// Semantic search query text (optional if using metadata-only search)
    pub q: Option<String>,

    #[serde(rename = "type")]
    pub entity_type: Option<String>,

    /// Maximum number of results to return (default: 20, max: 100, min: 1)
    #[param(minimum = 1, maximum = 100, example = 20)]
    pub limit: Option<i64>,

    /// Offset for pagination (default: 0, min: 0). Applied in DB before min_similarity filtering; page boundaries are over the full ordered result set.
    #[param(minimum = 0, example = 0)]
    pub offset: Option<i64>,

    /// Search mode: "semantic", "metadata", or "both" (default: "both")
    /// Use "metadata" for metadata-only search, "semantic" for semantic-only, "both" for combined
    #[param(example = "both")]
    pub search_mode: Option<String>,

    /// Minimum similarity score for semantic search results (0.0 to 1.0, default: 0.3).
    /// Applied after limit/offset in the DB: results below this threshold are dropped, so count may be less than limit.
    #[param(minimum = 0.0, maximum = 1.0, example = 0.3)]
    pub min_similarity: Option<f32>,

    /// Filter results to a specific folder (folder UUID)
    /// If provided, only returns media within this folder and its subfolders
    pub folder_id: Option<Uuid>,
}

impl SearchQuery {
    /// Validate search query parameters
    pub fn validate(&self) -> Result<(), String> {
        // Validate limit
        if let Some(limit) = self.limit {
            if limit < 1 {
                return Err("Limit must be at least 1".to_string());
            }
            if limit > 100 {
                return Err("Limit cannot exceed 100".to_string());
            }
        }

        // Validate offset
        if let Some(offset) = self.offset {
            if offset < 0 {
                return Err("Offset must be non-negative".to_string());
            }
        }

        // search_mode and entity_type are validated by the handler via SearchStrategy::from_str and parse_entity_type

        // Validate min_similarity if provided
        if let Some(min_sim) = self.min_similarity {
            if !(0.0..=1.0).contains(&min_sim) {
                return Err("min_similarity must be between 0.0 and 1.0".to_string());
            }
        }

        // Validate query length for semantic search (DoS/cost guard)
        const MAX_QUERY_LEN: usize = 16 * 1024; // 16 KB
        if let Some(ref q) = self.q {
            if q.len() > MAX_QUERY_LEN {
                return Err(format!(
                    "Query parameter 'q' must not exceed {} characters",
                    MAX_QUERY_LEN
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type_display() {
        assert_eq!(EntityType::Image.to_string(), "image");
        assert_eq!(EntityType::Video.to_string(), "video");
        assert_eq!(EntityType::Document.to_string(), "document");
        assert_eq!(EntityType::Audio.to_string(), "audio");
    }

    #[test]
    fn test_entity_type_equality() {
        assert_eq!(EntityType::Image, EntityType::Image);
        assert_ne!(EntityType::Image, EntityType::Video);
        assert_ne!(EntityType::Video, EntityType::Document);
    }

    #[test]
    fn test_entity_type_clone() {
        let entity = EntityType::Image;
        let cloned = entity.clone();
        assert_eq!(entity, cloned);
    }
}
