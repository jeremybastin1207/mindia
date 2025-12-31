//! MCP Tool definitions and registry

use crate::api_client::ApiClient;
use anyhow::{Context, Result};
use serde_json::Value;

pub struct ToolRegistry;

impl Default for ToolRegistry {
    fn default() -> Self {
        Self
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn list_tools(&self) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "name": "upload_image",
                "description": "Upload an image file from local path or URL to Mindia",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Local file path to the image file"
                        },
                        "url": {
                            "type": "string",
                            "description": "URL to download and upload the image"
                        }
                    },
                    "required": []
                }
            }),
            serde_json::json!({
                "name": "list_media",
                "description": "List media files (images, videos, audio, documents) with optional filters",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "media_type": {
                            "type": "string",
                            "enum": ["image", "video", "audio", "document"],
                            "description": "Type of media to list"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of results to return"
                        },
                        "offset": {
                            "type": "number",
                            "description": "Number of results to skip"
                        }
                    }
                }
            }),
            serde_json::json!({
                "name": "get_media",
                "description": "Get metadata for a specific media item by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "media_id": {
                            "type": "string",
                            "description": "UUID of the media item"
                        }
                    },
                    "required": ["media_id"]
                }
            }),
            serde_json::json!({
                "name": "transform_image",
                "description": "Generate a transformed image URL with resize dimensions",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "image_id": {
                            "type": "string",
                            "description": "UUID of the image"
                        },
                        "width": {
                            "type": "number",
                            "description": "Target width in pixels"
                        },
                        "height": {
                            "type": "number",
                            "description": "Target height in pixels"
                        }
                    },
                    "required": ["image_id"]
                }
            }),
            serde_json::json!({
                "name": "search_media",
                "description": "Search media files using semantic search",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query text"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of results to return"
                        }
                    },
                    "required": ["query"]
                }
            }),
            serde_json::json!({
                "name": "delete_media",
                "description": "Delete a media item by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "media_id": {
                            "type": "string",
                            "description": "UUID of the media item to delete"
                        }
                    },
                    "required": ["media_id"]
                }
            }),
            serde_json::json!({
                "name": "create_folder",
                "description": "Create a new folder for organizing media",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the folder"
                        },
                        "parent_id": {
                            "type": "string",
                            "description": "UUID of parent folder (optional)"
                        }
                    },
                    "required": ["name"]
                }
            }),
        ]
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
        api_client: &ApiClient,
    ) -> Result<serde_json::Value> {
        match name {
            "upload_image" => {
                let file_path = arguments.get("file_path").and_then(|v| v.as_str());
                let url = arguments.get("url").and_then(|v| v.as_str());

                let result = if let Some(file_path) = file_path {
                    api_client.upload_image(file_path).await?
                } else if let Some(url) = url {
                    api_client.upload_image_from_url(url).await?
                } else {
                    return Err(anyhow::anyhow!("Either file_path or url must be provided"));
                };

                Ok(serde_json::to_value(result)?)
            }
            "list_media" => {
                let media_type = arguments.get("media_type").and_then(|v| v.as_str());
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let offset = arguments
                    .get("offset")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let result = api_client.list_media(media_type, limit, offset).await?;
                Ok(serde_json::to_value(result)?)
            }
            "get_media" => {
                let media_id = arguments
                    .get("media_id")
                    .and_then(|v| v.as_str())
                    .context("Missing media_id")?;

                let result = api_client.get_media(media_id).await?;
                Ok(serde_json::to_value(result)?)
            }
            "transform_image" => {
                let image_id = arguments
                    .get("image_id")
                    .and_then(|v| v.as_str())
                    .context("Missing image_id")?;
                let width = arguments
                    .get("width")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let height = arguments
                    .get("height")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let url = api_client.transform_image(image_id, width, height).await?;
                Ok(serde_json::json!({
                    "transformed_url": url
                }))
            }
            "search_media" => {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .context("Missing query")?;
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let result = api_client.search_media(query, limit).await?;
                Ok(serde_json::to_value(result)?)
            }
            "delete_media" => {
                let media_id = arguments
                    .get("media_id")
                    .and_then(|v| v.as_str())
                    .context("Missing media_id")?;

                api_client.delete_media(media_id).await?;
                Ok(serde_json::json!({
                    "success": true,
                    "message": format!("Media {} deleted successfully", media_id)
                }))
            }
            "create_folder" => {
                let name = arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("Missing name")?;
                let parent_id = arguments.get("parent_id").and_then(|v| v.as_str());

                let result = api_client.create_folder(name, parent_id).await?;
                Ok(serde_json::to_value(result)?)
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }
}
