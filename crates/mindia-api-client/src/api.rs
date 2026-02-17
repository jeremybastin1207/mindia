//! Domain methods for the Mindia API client.
//!
//! Response types are re-exported from `mindia_core::models` where possible.
//! Wrapper types (SearchResponse, MediaListResponse, MediaResponse) are defined here.

use crate::{api_prefix, ApiClient};
use anyhow::{Context, Result};
use mindia_core::models::{
    AudioResponse, DocumentResponse, FolderResponse, ImageResponse, SearchResult, StorageSummary,
    VideoResponse,
};

/// Search API response (query, results, count). Matches API handler shape.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SearchResponse {
    pub query: Option<String>,
    pub results: Vec<SearchResult>,
    pub count: usize,
}

/// List media response: one of the typed arrays depending on endpoint.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MediaListResponse {
    Images(Vec<ImageResponse>),
    Videos(Vec<VideoResponse>),
    Audios(Vec<AudioResponse>),
    Documents(Vec<DocumentResponse>),
    Media(Vec<MediaResponse>),
}

/// Single media item (image, video, audio, or document). Matches GET /media/{id}.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MediaResponse {
    Image(ImageResponse),
    Video(VideoResponse),
    Audio(AudioResponse),
    Document(DocumentResponse),
}

impl ApiClient {
    /// Upload an image from a local file path.
    pub async fn upload_image(&self, file_path: &str) -> Result<ImageResponse> {
        use std::io::Read;

        let path = std::path::Path::new(file_path);
        if path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(anyhow::anyhow!("Invalid input: {}", path.display()));
        }
        let mut file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open file: {}", file_path))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("Failed to read file: {}", file_path))?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.jpg");

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(buffer).file_name(filename.to_string()),
        );

        self.post_multipart(&format!("{}/images", api_prefix()), form)
            .await
    }

    /// Upload an image from a URL (downloads then uploads to Mindia).
    pub async fn upload_image_from_url(&self, url: &str) -> Result<ImageResponse> {
        let image_data = self
            .client()
            .get(url)
            .send()
            .await
            .context("Failed to download image from URL")?
            .bytes()
            .await
            .context("Failed to read image data")?;

        let filename = std::path::Path::new(url)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.jpg");

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(image_data.to_vec()).file_name(filename.to_string()),
        );

        self.post_multipart(&format!("{}/images", api_prefix()), form)
            .await
    }

    /// List media with optional type filter and pagination.
    pub async fn list_media(
        &self,
        media_type: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<MediaListResponse> {
        let path = match media_type {
            Some("image") | Some("images") => format!("{}/images", api_prefix()),
            Some("video") | Some("videos") => format!("{}/videos", api_prefix()),
            Some("audio") | Some("audios") => format!("{}/audios", api_prefix()),
            Some("document") | Some("documents") => format!("{}/documents", api_prefix()),
            _ => format!("{}/media", api_prefix()),
        };

        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(l) = limit {
            query.push(("limit", l.to_string()));
        }
        if let Some(o) = offset {
            query.push(("offset", o.to_string()));
        }

        self.get(&path, &query).await
    }

    /// List images with pagination and optional folder filter.
    pub async fn list_images(
        &self,
        limit: i64,
        offset: i64,
        folder_id: Option<uuid::Uuid>,
    ) -> Result<Vec<ImageResponse>> {
        let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        if let Some(fid) = folder_id {
            query.push(("folder_id", fid.to_string()));
        }
        self.get(&format!("{}/images", api_prefix()), &query).await
    }

    /// List videos with pagination and optional folder filter.
    pub async fn list_videos(
        &self,
        limit: i64,
        offset: i64,
        folder_id: Option<uuid::Uuid>,
    ) -> Result<Vec<VideoResponse>> {
        let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        if let Some(fid) = folder_id {
            query.push(("folder_id", fid.to_string()));
        }
        self.get(&format!("{}/videos", api_prefix()), &query).await
    }

    /// List audios with pagination and optional folder filter.
    pub async fn list_audios(
        &self,
        limit: i64,
        offset: i64,
        folder_id: Option<uuid::Uuid>,
    ) -> Result<Vec<AudioResponse>> {
        let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        if let Some(fid) = folder_id {
            query.push(("folder_id", fid.to_string()));
        }
        self.get(&format!("{}/audios", api_prefix()), &query).await
    }

    /// List documents with pagination and optional folder filter.
    pub async fn list_documents(
        &self,
        limit: i64,
        offset: i64,
        folder_id: Option<uuid::Uuid>,
    ) -> Result<Vec<DocumentResponse>> {
        let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        if let Some(fid) = folder_id {
            query.push(("folder_id", fid.to_string()));
        }
        self.get(&format!("{}/documents", api_prefix()), &query)
            .await
    }

    /// Get a single media item by ID (image, video, audio, or document).
    pub async fn get_media(&self, media_id: &str) -> Result<MediaResponse> {
        self.get(&format!("{}/media/{}", api_prefix(), media_id), &[])
            .await
    }

    /// Build a transformed image URL with resize dimensions (does not call the API).
    pub async fn transform_image(
        &self,
        image_id: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<String> {
        let dimensions = match (width, height) {
            (Some(w), Some(h)) => format!("{}x{}", w, h),
            (Some(w), None) => format!("{}x", w),
            (None, Some(h)) => format!("x{}", h),
            (None, None) => {
                return Err(anyhow::anyhow!(
                    "At least width or height must be specified"
                ))
            }
        };

        Ok(self.build_url(&format!(
            "{}/images/{}/-/resize/{}/",
            api_prefix(),
            image_id,
            dimensions
        )))
    }

    /// Semantic (and metadata) search.
    pub async fn search_media(&self, query: &str, limit: Option<u32>) -> Result<SearchResponse> {
        let mut query_params = vec![("q", urlencoding::encode(query).to_string())];
        if let Some(l) = limit {
            query_params.push(("limit", l.to_string()));
        }
        self.get(&format!("{}/search", api_prefix()), &query_params)
            .await
    }

    /// Delete a media item by ID.
    pub async fn delete_media(&self, media_id: &str) -> Result<()> {
        self.delete(&format!("{}/media/{}", api_prefix(), media_id))
            .await
    }

    /// Create a folder with optional parent.
    pub async fn create_folder(
        &self,
        name: &str,
        parent_id: Option<&str>,
    ) -> Result<FolderResponse> {
        let mut body = serde_json::json!({ "name": name });
        if let Some(pid) = parent_id {
            body["parent_id"] = serde_json::Value::String(pid.to_string());
        }
        self.post_json(&format!("{}/folders", api_prefix()), &body)
            .await
    }

    /// Get storage summary (analytics).
    pub async fn get_storage_summary(&self) -> Result<StorageSummary> {
        self.get(&format!("{}/analytics/storage", api_prefix()), &[])
            .await
    }
}
