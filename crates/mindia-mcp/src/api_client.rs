//! API client for interacting with Mindia API

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("MINDIA_API_URL")
            .or_else(|_| std::env::var("API_URL"))
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let api_key = std::env::var("MINDIA_API_KEY")
            .or_else(|_| std::env::var("API_KEY"))
            .context("Missing API key. Set MINDIA_API_KEY or API_KEY environment variable")?;

        Self::new(base_url, api_key)
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    #[allow(dead_code)] // May be used in future
    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let url = self.build_url(path);
        let request_builder = self
            .client
            .request(method.parse().unwrap(), &url)
            .header("X-API-Key", &self.api_key);

        let request = if let Some(body) = body {
            request_builder.json(&body)
        } else {
            request_builder
        };

        let response = request.send().await.context("Failed to send request")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "API request failed with status {}: {}",
                status,
                error_text
            ));
        }

        let body: T = response
            .json()
            .await
            .context("Failed to parse response as JSON")?;

        Ok(body)
    }

    pub async fn upload_image(&self, file_path: &str) -> Result<ImageResponse> {
        use std::fs::File;
        use std::io::Read;

        // Prevent path traversal attacks by rejecting paths containing '..'
        let path = std::path::Path::new(file_path);
        if path.components().any(|c| c == std::path::Component::ParentDir) {
            return Err(anyhow::anyhow!("Invalid input: {}", path.display()));
        }
        let mut file =
            File::open(path).with_context(|| format!("Failed to open file: {}", file_path))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("Failed to read file: {}", file_path))?;

        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.jpg");

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(buffer).file_name(filename.to_string()),
        );

        let url = self.build_url("/api/v0/images");
        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .multipart(form)
            .send()
            .await
            .context("Failed to upload image")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Upload failed with status {}: {}",
                status,
                error_text
            ));
        }

        let body: ImageResponse = response.json().await.context("Failed to parse response")?;

        Ok(body)
    }

    pub async fn upload_image_from_url(&self, url: &str) -> Result<ImageResponse> {
        // Download image from URL first
        let image_data = self
            .client
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

        let api_url = self.build_url("/api/v0/images");
        let response = self
            .client
            .post(&api_url)
            .header("X-API-Key", &self.api_key)
            .multipart(form)
            .send()
            .await
            .context("Failed to upload image")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Upload failed with status {}: {}",
                status,
                error_text
            ));
        }

        let body: ImageResponse = response.json().await.context("Failed to parse response")?;

        Ok(body)
    }

    pub async fn list_media(
        &self,
        media_type: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<MediaListResponse> {
        let path = match media_type {
            Some("image") | Some("images") => "/api/v0/images".to_string(),
            Some("video") | Some("videos") => "/api/v0/videos".to_string(),
            Some("audio") | Some("audios") => "/api/v0/audios".to_string(),
            Some("document") | Some("documents") => "/api/v0/documents".to_string(),
            _ => "/api/v0/media".to_string(),
        };

        let mut query = Vec::new();
        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = offset {
            query.push(("offset", offset.to_string()));
        }

        let url = if query.is_empty() {
            self.build_url(&path)
        } else {
            format!(
                "{}?{}",
                self.build_url(&path),
                query
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("&")
            )
        };

        let response = self
            .client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .context("Failed to list media")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "List failed with status {}: {}",
                status,
                error_text
            ));
        }

        let body: MediaListResponse = response.json().await.context("Failed to parse response")?;

        Ok(body)
    }

    pub async fn get_media(&self, media_id: &str) -> Result<MediaResponse> {
        let url = self.build_url(&format!("/api/v0/media/{}", media_id));
        let response = self
            .client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .context("Failed to get media")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Get failed with status {}: {}",
                status,
                error_text
            ));
        }

        let body: MediaResponse = response.json().await.context("Failed to parse response")?;

        Ok(body)
    }

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

        let url = self.build_url(&format!(
            "/api/v0/images/{}/-/resize/{}/",
            image_id, dimensions
        ));
        Ok(url)
    }

    pub async fn search_media(&self, query: &str, limit: Option<u32>) -> Result<SearchResponse> {
        let mut path = format!("/api/v0/search?q={}", urlencoding::encode(query));
        if let Some(limit) = limit {
            path.push_str(&format!("&limit={}", limit));
        }

        let url = self.build_url(&path);
        let response = self
            .client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .context("Failed to search")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Search failed with status {}: {}",
                status,
                error_text
            ));
        }

        let body: SearchResponse = response.json().await.context("Failed to parse response")?;

        Ok(body)
    }

    pub async fn delete_media(&self, media_id: &str) -> Result<()> {
        let url = self.build_url(&format!("/api/v0/media/{}", media_id));
        let response = self
            .client
            .delete(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .context("Failed to delete media")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Delete failed with status {}: {}",
                status,
                error_text
            ));
        }

        Ok(())
    }

    pub async fn create_folder(
        &self,
        name: &str,
        parent_id: Option<&str>,
    ) -> Result<FolderResponse> {
        let mut body = serde_json::json!({
            "name": name,
        });
        if let Some(parent_id) = parent_id {
            body["parent_id"] = serde_json::Value::String(parent_id.to_string());
        }

        let url = self.build_url("/api/v0/folders");
        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .context("Failed to create folder")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Create folder failed with status {}: {}",
                status,
                error_text
            ));
        }

        let folder: FolderResponse = response.json().await.context("Failed to parse response")?;

        Ok(folder)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaListResponse {
    Images(Vec<ImageResponse>),
    Videos(Vec<VideoResponse>),
    Audios(Vec<AudioResponse>),
    Documents(Vec<DocumentResponse>),
    Media(Vec<MediaResponse>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaResponse {
    Image(ImageResponse),
    Video(VideoResponse),
    Audio(AudioResponse),
    Document(DocumentResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub score: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FolderResponse {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
