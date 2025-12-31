use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// API version used for paths (e.g. "v0"). Set MINDIA_API_VERSION to match the server.
fn api_prefix() -> String {
    let version = std::env::var("MINDIA_API_VERSION").unwrap_or_else(|_| "v0".to_string());
    format!("/api/{}", version)
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    auth_token: String,
}

impl ApiClient {
    pub fn new(base_url: String, auth_token: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token,
        })
    }

    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("MINDIA_API_URL")
            .or_else(|_| std::env::var("API_URL"))
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let auth_token = std::env::var("MINDIA_API_KEY")
            .or_else(|_| std::env::var("API_KEY"))
            .or_else(|_| std::env::var("JWT_TOKEN"))
            .context("Missing authentication token. Set MINDIA_API_KEY, API_KEY, or JWT_TOKEN environment variable")?;

        Self::new(base_url, auth_token)
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = self.build_url(path);
        let mut request = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token));

        if !query.is_empty() {
            request = request.query(query);
        }

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

    pub async fn list_images(
        &self,
        limit: i64,
        offset: i64,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<ImageResponse>> {
        let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        if let Some(fid) = folder_id {
            query.push(("folder_id", fid.to_string()));
        }
        self.get(&format!("{}/images", api_prefix()), &query).await
    }

    pub async fn list_videos(
        &self,
        limit: i64,
        offset: i64,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<VideoResponse>> {
        let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        if let Some(fid) = folder_id {
            query.push(("folder_id", fid.to_string()));
        }
        self.get(&format!("{}/videos", api_prefix()), &query).await
    }

    pub async fn list_audios(
        &self,
        limit: i64,
        offset: i64,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<AudioResponse>> {
        let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        if let Some(fid) = folder_id {
            query.push(("folder_id", fid.to_string()));
        }
        self.get(&format!("{}/audios", api_prefix()), &query).await
    }

    pub async fn list_documents(
        &self,
        limit: i64,
        offset: i64,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<DocumentResponse>> {
        let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        if let Some(fid) = folder_id {
            query.push(("folder_id", fid.to_string()));
        }
        self.get(&format!("{}/documents", api_prefix()), &query)
            .await
    }

    pub async fn get_storage_summary(&self) -> Result<StorageSummaryResponse> {
        self.get(&format!("{}/analytics/storage", api_prefix()), &[])
            .await
    }
}

// Response types that can be deserialized from API responses
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
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub folder_id: Option<Uuid>,
    pub folder_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub duration: Option<f64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub processing_status: ProcessingStatus,
    pub hls_url: Option<String>,
    pub variants: Option<serde_json::Value>,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub folder_id: Option<Uuid>,
    pub folder_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub folder_id: Option<Uuid>,
    pub folder_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub page_count: Option<i32>,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub folder_id: Option<Uuid>,
    pub folder_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageSummaryResponse {
    pub total_files: i64,
    pub total_storage_bytes: i64,
    pub image_count: i64,
    pub image_bytes: i64,
    pub video_count: i64,
    pub video_bytes: i64,
    pub audio_count: i64,
    pub audio_bytes: i64,
    pub by_content_type: HashMap<String, ContentTypeStatsResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentTypeStatsResponse {
    pub count: i64,
    pub bytes: i64,
}
