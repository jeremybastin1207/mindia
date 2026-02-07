//! MCP tool request types with JSON Schema for AI parameter generation

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct UploadImageRequest {
    #[schemars(description = "Local file path to the image file")]
    pub file_path: Option<String>,
    #[schemars(description = "URL to download and upload the image")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListMediaRequest {
    #[schemars(description = "Type of media to list")]
    pub media_type: Option<MediaTypeParam>,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
    #[schemars(description = "Number of results to skip")]
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaTypeParam {
    Image,
    Video,
    Audio,
    Document,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetMediaRequest {
    #[schemars(description = "UUID of the media item")]
    pub media_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TransformImageRequest {
    #[schemars(description = "UUID of the image")]
    pub image_id: String,
    #[schemars(description = "Target width in pixels")]
    pub width: Option<u32>,
    #[schemars(description = "Target height in pixels")]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SearchMediaRequest {
    #[schemars(description = "Search query text")]
    pub query: String,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DeleteMediaRequest {
    #[schemars(description = "UUID of the media item to delete")]
    pub media_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CreateFolderRequest {
    #[schemars(description = "Name of the folder")]
    pub name: String,
    #[schemars(description = "UUID of parent folder (optional)")]
    pub parent_id: Option<String>,
}
