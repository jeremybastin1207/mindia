//! MCP server using rmcp SDK
//!
//! Exposes Mindia API as MCP tools over stdio.

use crate::tools::*;
use mindia_api_client::ApiClient;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Parameters;
use rmcp::model::*;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use std::borrow::Cow;
use std::future::Future;
use std::sync::Arc;

fn text_content(s: impl Into<String>) -> Content {
    Content {
        raw: RawContent::Text(RawTextContent { text: s.into() }),
        annotations: None,
    }
}

#[derive(Debug, Clone)]
pub struct MindiaService {
    api_client: Arc<ApiClient>,
    tool_router: ToolRouter<MindiaService>,
}

#[tool_router]
impl MindiaService {
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client: Arc::new(api_client),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Upload an image file from local path or URL to Mindia")]
    async fn upload_image(
        &self,
        Parameters(req): Parameters<UploadImageRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = match (req.file_path.as_deref(), req.url.as_deref()) {
            (Some(path), _) => self.api_client.upload_image(path).await,
            (_, Some(url)) => self.api_client.upload_image_from_url(url).await,
            _ => {
                return Err(ErrorData {
                    code: ErrorCode(-32602),
                    message: Cow::from("Either file_path or url must be provided"),
                    data: None,
                })
            }
        };
        let body = result.map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(e.to_string()),
            data: None,
        })?;
        let text = serde_json::to_string(&body).map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(e.to_string()),
            data: None,
        })?;
        Ok(CallToolResult::success(vec![text_content(text)]))
    }

    #[tool(
        description = "List media files (images, videos, audio, documents) with optional filters"
    )]
    async fn list_media(
        &self,
        Parameters(req): Parameters<ListMediaRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let media_type = req.media_type.as_ref().map(|t| match t {
            MediaTypeParam::Image => "image",
            MediaTypeParam::Video => "video",
            MediaTypeParam::Audio => "audio",
            MediaTypeParam::Document => "document",
        });
        let result = self
            .api_client
            .list_media(media_type, req.limit, req.offset)
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(e.to_string()),
                data: None,
            })?;
        let text = serde_json::to_string(&result).map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(e.to_string()),
            data: None,
        })?;
        Ok(CallToolResult::success(vec![text_content(text)]))
    }

    #[tool(description = "Get metadata for a specific media item by ID")]
    async fn get_media(
        &self,
        Parameters(req): Parameters<GetMediaRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self
            .api_client
            .get_media(&req.media_id)
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(e.to_string()),
                data: None,
            })?;
        let text = serde_json::to_string(&result).map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(e.to_string()),
            data: None,
        })?;
        Ok(CallToolResult::success(vec![text_content(text)]))
    }

    #[tool(description = "Generate a transformed image URL with resize dimensions")]
    async fn transform_image(
        &self,
        Parameters(req): Parameters<TransformImageRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self
            .api_client
            .transform_image(&req.image_id, req.width, req.height)
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(e.to_string()),
                data: None,
            })?;
        let text =
            serde_json::to_string(&serde_json::json!({ "transformed_url": url })).map_err(|e| {
                ErrorData {
                    code: ErrorCode(-32603),
                    message: Cow::from(e.to_string()),
                    data: None,
                }
            })?;
        Ok(CallToolResult::success(vec![text_content(text)]))
    }

    #[tool(description = "Search media files using semantic search")]
    async fn search_media(
        &self,
        Parameters(req): Parameters<SearchMediaRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self
            .api_client
            .search_media(&req.query, req.limit)
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(e.to_string()),
                data: None,
            })?;
        let text = serde_json::to_string(&result).map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(e.to_string()),
            data: None,
        })?;
        Ok(CallToolResult::success(vec![text_content(text)]))
    }

    #[tool(description = "Delete a media item by ID")]
    async fn delete_media(
        &self,
        Parameters(req): Parameters<DeleteMediaRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.api_client
            .delete_media(&req.media_id)
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(e.to_string()),
                data: None,
            })?;
        let text = serde_json::to_string(&serde_json::json!({
            "success": true,
            "message": format!("Media {} deleted successfully", req.media_id)
        }))
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(e.to_string()),
            data: None,
        })?;
        Ok(CallToolResult::success(vec![text_content(text)]))
    }

    #[tool(description = "Create a new folder for organizing media")]
    async fn create_folder(
        &self,
        Parameters(req): Parameters<CreateFolderRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self
            .api_client
            .create_folder(&req.name, req.parent_id.as_deref())
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(e.to_string()),
                data: None,
            })?;
        let text = serde_json::to_string(&result).map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(e.to_string()),
            data: None,
        })?;
        Ok(CallToolResult::success(vec![text_content(text)]))
    }
}

#[tool_handler]
impl ServerHandler for MindiaService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mindia-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some(
                "Mindia MCP: upload, list, get, transform, search, and delete media; create folders. \
                 Set MINDIA_API_KEY and MINDIA_API_URL."
                    .to_string(),
            ),
        }
    }
}
