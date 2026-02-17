//! Shared HTTP client for the Mindia API.
//!
//! Provides a minimal client with configurable auth (Bearer token or X-API-Key),
//! generic GET/POST helpers, and domain methods (upload, list, search, etc.).
//! CLI and MCP crates use this client directly.

pub mod api;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::Duration;

/// Authentication strategy for the API.
#[derive(Clone, Debug)]
pub enum Auth {
    /// `Authorization: Bearer {token}`
    Bearer(String),
    /// `X-API-Key: {key}`
    XApiKey(String),
}

/// API version prefix (e.g. "/api/v0"). Set MINDIA_API_VERSION to match the server.
pub fn api_prefix() -> String {
    let version = std::env::var("MINDIA_API_VERSION").unwrap_or_else(|_| "v0".to_string());
    format!("/api/{}", version)
}

/// HTTP client for the Mindia API with configurable auth.
#[derive(Clone, Debug)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    auth: Auth,
}

impl ApiClient {
    pub fn new(base_url: String, auth: Auth) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth,
        })
    }

    /// Create client from environment: MINDIA_API_URL (or API_URL), MINDIA_API_KEY (or API_KEY).
    /// Uses X-API-Key auth by default.
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("MINDIA_API_URL")
            .or_else(|_| std::env::var("API_URL"))
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let api_key = std::env::var("MINDIA_API_KEY")
            .or_else(|_| std::env::var("API_KEY"))
            .context("Missing API key. Set MINDIA_API_KEY or API_KEY")?;

        Self::new(base_url, Auth::XApiKey(api_key))
    }

    /// Create client from environment using Bearer token: JWT_TOKEN or MINDIA_API_KEY.
    pub fn from_env_bearer() -> Result<Self> {
        let base_url = std::env::var("MINDIA_API_URL")
            .or_else(|_| std::env::var("API_URL"))
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let token = std::env::var("MINDIA_API_KEY")
            .or_else(|_| std::env::var("API_KEY"))
            .or_else(|_| std::env::var("JWT_TOKEN"))
            .context("Missing token. Set MINDIA_API_KEY, API_KEY, or JWT_TOKEN")?;

        Self::new(base_url, Auth::Bearer(token))
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn apply_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            Auth::Bearer(token) => request.header("Authorization", format!("Bearer {}", token)),
            Auth::XApiKey(key) => request.header("X-API-Key", key.as_str()),
        }
    }

    /// GET request with optional query parameters. Deserializes JSON response.
    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = self.build_url(path);
        let mut request = self.client.get(&url);
        request = self.apply_auth(request);

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

    /// POST JSON body and deserialize response.
    pub async fn post_json<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.build_url(path);
        let request = self.client.post(&url).json(body);
        let request = self.apply_auth(request);

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

    /// POST multipart form and deserialize response.
    pub async fn post_multipart<T: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<T> {
        let url = self.build_url(path);
        let request = self.client.post(&url).multipart(form);
        let request = self.apply_auth(request);

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

    /// DELETE request. Returns Ok(()) on success.
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = self.build_url(path);
        let request = self.client.delete(&url);
        let request = self.apply_auth(request);

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

        Ok(())
    }

    /// Raw client for custom requests. Caller must apply auth via build_url and headers.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

// Re-export domain response types for convenience.
pub use api::{MediaListResponse, MediaResponse, SearchResponse};
pub use mindia_core::models::{
    AudioResponse, DocumentResponse, FolderResponse, ImageResponse, SearchResult, StorageSummary,
    VideoResponse,
};
