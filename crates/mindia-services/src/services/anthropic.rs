//! Anthropic (Claude) semantic search provider.
//!
//! Uses Messages API for vision and document summarization, and Embeddings API
//! for vector generation.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::semantic_search::{normalize_embedding_dim, SemanticSearchProvider};

const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const VOYAGE_API_BASE: &str = "https://api.voyageai.com/v1";
const API_VERSION: &str = "2023-06-01";

#[derive(Clone)]
pub struct AnthropicService {
    anthropic_api_key: String,      // For Claude Vision
    voyage_api_key: Option<String>, // For embeddings (required for semantic search)
    vision_model: String,
    embedding_model: String,
    client: reqwest::Client,
}

// Messages API request/response
#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<MessageParam>,
}

#[derive(Debug, Serialize)]
struct MessageParam {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text { text: String },
    Image { source: ImageSource },
}

#[derive(Debug, Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlockResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlockResponse {
    Text { text: String },
}

// Voyage AI Embeddings API
// Note: Anthropic does not provide embeddings, so we use Voyage AI as recommended
#[derive(Debug, Serialize)]
struct VoyageEmbedRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_type: Option<String>, // "query" or "document"
}

#[derive(Debug, Deserialize)]
struct VoyageEmbedData {
    embedding: Vec<f32>,
    #[allow(dead_code)]
    index: usize, // Part of Voyage API response schema, preserved for completeness
}

#[derive(Debug, Deserialize)]
struct VoyageEmbedResponse {
    data: Vec<VoyageEmbedData>,
}

impl AnthropicService {
    pub fn new(api_key: String, vision_model: String, embedding_model: String) -> Self {
        Self::new_with_voyage(api_key, None, vision_model, embedding_model)
    }

    pub fn new_with_voyage(
        anthropic_api_key: String,
        voyage_api_key: Option<String>,
        vision_model: String,
        embedding_model: String,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");
        Self {
            anthropic_api_key,
            voyage_api_key,
            vision_model,
            embedding_model,
            client,
        }
    }

    fn messages_url(&self) -> String {
        format!("{}/messages", ANTHROPIC_API_BASE)
    }

    fn embeddings_url(&self) -> String {
        // Use Voyage AI since Anthropic doesn't provide embeddings
        format!("{}/embeddings", VOYAGE_API_BASE)
    }

    async fn call_messages(&self, content: Vec<ContentBlock>) -> Result<String> {
        let body = MessagesRequest {
            model: self.vision_model.clone(),
            max_tokens: 1024,
            messages: vec![MessageParam {
                role: "user".to_string(),
                content,
            }],
        };

        let response = self
            .client
            .post(self.messages_url())
            .header("x-api-key", &self.anthropic_api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send Messages API request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Anthropic Messages API failed with status {}: {}",
                status,
                error_text
            ));
        }

        let parsed: MessagesResponse = response
            .json()
            .await
            .context("Failed to parse Messages API response")?;

        let text = parsed
            .content
            .into_iter()
            .map(|b| match b {
                ContentBlockResponse::Text { text } => text,
            })
            .next()
            .unwrap_or_default();
        Ok(text)
    }
}

#[async_trait]
impl SemanticSearchProvider for AnthropicService {
    fn embedding_model_name(&self) -> &str {
        &self.embedding_model
    }

    async fn health_check(&self) -> Result<bool> {
        // Check if Voyage API key is configured
        let voyage_key = match &self.voyage_api_key {
            Some(key) => key,
            None => {
                tracing::warn!(
                    "Voyage API key not configured. Semantic search requires VOYAGE_API_KEY"
                );
                return Ok(false);
            }
        };

        // Test embeddings endpoint with minimal input
        let body = VoyageEmbedRequest {
            model: self.embedding_model.clone(),
            input: vec!["test".to_string()],
            input_type: Some("query".to_string()),
        };

        match self
            .client
            .post(self.embeddings_url())
            .header("Authorization", format!("Bearer {}", voyage_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!("Voyage AI embeddings health check passed");
                Ok(true)
            }
            Ok(resp) => {
                tracing::warn!(status = %resp.status(), "Voyage AI embeddings health check failed");
                Ok(false)
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to connect to Voyage AI");
                Ok(false)
            }
        }
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Anthropic doesn't provide embeddings, so we use Voyage AI
        let voyage_key = self.voyage_api_key.as_ref().ok_or_else(|| {
            anyhow!(
                "Voyage API key not configured. Semantic search requires VOYAGE_API_KEY environment variable. \
                Get your key at: https://www.voyageai.com/"
            )
        })?;

        let body = VoyageEmbedRequest {
            model: self.embedding_model.clone(),
            input: vec![text.to_string()],
            input_type: Some("document".to_string()),
        };

        let response = self
            .client
            .post(self.embeddings_url())
            .header("Authorization", format!("Bearer {}", voyage_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Voyage AI embeddings API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Voyage AI embeddings failed with status {}: {}",
                status,
                error_text
            ));
        }

        let embed_response: VoyageEmbedResponse = response
            .json()
            .await
            .context("Failed to parse Voyage AI response")?;

        let embedding = embed_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No embedding returned from Voyage AI"))?
            .embedding;

        Ok(normalize_embedding_dim(embedding))
    }

    async fn describe_image(&self, image_data: Bytes) -> Result<String> {
        let base64_image = STANDARD.encode(&image_data);
        let media_type = "image/jpeg"; // Assume JPEG; could detect from magic bytes
        let prompt = "Describe this image in detail. Focus on the main subjects, objects, colors, setting, and any text visible. Keep the description concise but informative.";

        let content = vec![
            ContentBlock::Image {
                source: ImageSource {
                    source_type: "base64".to_string(),
                    media_type: media_type.to_string(),
                    data: base64_image,
                },
            },
            ContentBlock::Text {
                text: prompt.to_string(),
            },
        ];

        self.call_messages(content).await
    }

    async fn describe_video_frame(&self, frame_data: Bytes) -> Result<String> {
        let base64_image = STANDARD.encode(&frame_data);
        let prompt = "Describe this video frame in detail. Focus on the scene, subjects, actions, setting, and any context. This is a still frame from a video.";

        let content = vec![
            ContentBlock::Image {
                source: ImageSource {
                    source_type: "base64".to_string(),
                    media_type: "image/jpeg".to_string(),
                    data: base64_image,
                },
            },
            ContentBlock::Text {
                text: prompt.to_string(),
            },
        ];

        self.call_messages(content).await
    }

    async fn summarize_document(&self, text: &str) -> Result<String> {
        if text.len() < 500 {
            return Ok(text.to_string());
        }
        let sample = &text[..text.len().min(2000)];
        let prompt = format!(
            "Summarize the following document text in 2-3 sentences. Focus on the main topic and key points:\n\n{}",
            sample
        );
        let content = vec![ContentBlock::Text { text: prompt }];
        self.call_messages(content).await
    }
}
