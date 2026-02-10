//! Claude Vision plugin for comprehensive image analysis using Anthropic's Claude API

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::time::Duration;

use crate::plugins::{Plugin, PluginContext, PluginExecutionStatus, PluginResult, PluginUsage};

const API_BASE: &str = "https://api.anthropic.com/v1";
const API_VERSION: &str = "2023-06-01";

/// Claude Vision plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeVisionConfig {
    /// Anthropic API key
    pub api_key: String,
    /// Claude model to use (default: claude-sonnet-4-20250514)
    #[serde(default = "default_model")]
    pub model: String,
    /// Maximum tokens for response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Analysis features to include
    #[serde(default = "default_features")]
    pub features: Vec<String>,
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_max_tokens() -> u32 {
    2048
}

fn default_features() -> Vec<String> {
    vec![
        "objects".to_string(),
        "text".to_string(),
        "colors".to_string(),
        "scene".to_string(),
        "content_moderation".to_string(),
    ]
}

/// Claude Vision plugin implementation
pub struct ClaudeVisionPlugin {
    http_client: reqwest::Client,
}

impl Debug for ClaudeVisionPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ClaudeVisionPlugin").finish()
    }
}

// Messages API request/response structures
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
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlockResponse {
    Text { text: String },
}

impl ClaudeVisionPlugin {
    pub fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .context("Failed to create HTTP client for Claude Vision")?;

        Ok(Self { http_client })
    }

    /// Build analysis prompt based on requested features
    fn build_analysis_prompt(features: &[String]) -> String {
        let mut parts = vec![
            "Analyze this image and provide a detailed JSON response with the following information:".to_string(),
        ];

        for feature in features {
            match feature.as_str() {
                "objects" => parts.push("- objects: List all objects, people, animals visible in the image with their approximate locations".to_string()),
                "text" => parts.push("- text: Extract any text visible in the image (OCR)".to_string()),
                "colors" => parts.push("- colors: Identify the dominant colors in the image".to_string()),
                "scene" => parts.push("- scene: Describe the overall scene, setting, and context".to_string()),
                "content_moderation" => parts.push("- content_safety: Assess if the image contains any inappropriate, offensive, or unsafe content (provide 'safe' or 'unsafe' with confidence score)".to_string()),
                "description" => parts.push("- description: Provide a comprehensive description of the image".to_string()),
                _ => {}
            }
        }

        parts.push("\nProvide the response in valid JSON format.".to_string());
        parts.join("\n")
    }

    /// Call Claude Messages API with image
    async fn analyze_image(
        &self,
        api_key: &str,
        model: &str,
        max_tokens: u32,
        image_data: Vec<u8>,
        prompt: &str,
    ) -> Result<(String, Option<AnthropicUsage>)> {
        use base64::Engine;
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_data);

        // Detect media type from image data (simple magic number check)
        let media_type = detect_media_type(&image_data);

        let body = MessagesRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![MessageParam {
                role: "user".to_string(),
                content: vec![
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
                ],
            }],
        };

        let response = self
            .http_client
            .post(format!("{}/messages", API_BASE))
            .header("x-api-key", api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Claude Vision API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Claude Vision API request failed: {} - {}",
                status,
                error_text
            ));
        }

        let parsed: MessagesResponse = response
            .json()
            .await
            .context("Failed to parse Claude Vision API response")?;

        let text = parsed
            .content
            .into_iter()
            .map(|b| match b {
                ContentBlockResponse::Text { text } => text,
            })
            .next()
            .unwrap_or_default();

        Ok((text, parsed.usage))
    }

    /// Parse Claude's JSON response into structured data
    fn parse_analysis_result(text: &str) -> Result<serde_json::Value> {
        // Try to extract JSON from markdown code blocks if present
        let json_text = if text.contains("```json") {
            text.split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(text)
                .trim()
        } else if text.contains("```") {
            text.split("```")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(text)
                .trim()
        } else {
            text.trim()
        };

        // Parse JSON
        serde_json::from_str(json_text).context("Failed to parse Claude analysis result as JSON")
    }
}

/// Detect media type from image data using magic numbers
fn detect_media_type(data: &[u8]) -> &'static str {
    if data.len() < 4 {
        return "image/jpeg"; // Default
    }

    // JPEG: FF D8 FF
    if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        return "image/jpeg";
    }

    // PNG: 89 50 4E 47
    if data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47 {
        return "image/png";
    }

    // GIF: 47 49 46
    if data[0] == 0x47 && data[1] == 0x49 && data[2] == 0x46 {
        return "image/gif";
    }

    // WebP: RIFF ... WEBP
    if data.len() >= 12
        && data[0] == 0x52
        && data[1] == 0x49
        && data[2] == 0x46
        && data[3] == 0x46
        && data[8] == 0x57
        && data[9] == 0x45
        && data[10] == 0x42
        && data[11] == 0x50
    {
        return "image/webp";
    }

    "image/jpeg" // Default
}

#[async_trait]
impl Plugin for ClaudeVisionPlugin {
    fn name(&self) -> &str {
        "claude_vision"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let config: ClaudeVisionConfig = serde_json::from_value(config.clone())
            .context("Invalid Claude Vision configuration: missing or invalid fields")?;

        // Validate API key is present and not a placeholder
        if config.api_key.is_empty() {
            anyhow::bail!("Claude Vision API key is required but not provided");
        }

        if config.api_key == "your-api-key"
            || config.api_key == "sk-ant-"
            || config.api_key.len() < 10
        {
            anyhow::bail!("Claude Vision API key appears to be invalid or a placeholder. Please provide a valid Anthropic API key.");
        }

        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Executing Claude Vision plugin"
        );

        // Parse configuration
        let config: ClaudeVisionConfig = serde_json::from_value(context.config.clone())
            .context("Failed to parse Claude Vision configuration")?;

        // Validate API key
        if config.api_key.is_empty() || config.api_key == "your-api-key" {
            return Ok(PluginResult {
                status: PluginExecutionStatus::Failed,
                data: json!({}),
                error: Some(
                    "Claude Vision API key not configured. Please set a valid Anthropic API key."
                        .to_string(),
                ),
                metadata: None,
                usage: None,
            });
        }

        // Get image from media repository
        let image = context
            .media_repo
            .get_image(context.tenant_id, context.media_id)
            .await
            .context("Failed to get image")?
            .context("Image not found")?;

        // Download image from storage
        let image_data = context
            .storage
            .download(image.storage_key())
            .await
            .context("Failed to download image from storage")?;

        // Validate image size to prevent memory issues
        crate::validation::validate_image_size(&image_data)
            .context("Image file too large for processing")?;

        tracing::info!(
            image_size = image_data.len(),
            model = %config.model,
            features = ?config.features,
            "Downloaded image (size validated), sending to Claude Vision API"
        );

        // Build analysis prompt
        let prompt = Self::build_analysis_prompt(&config.features);

        // Analyze image
        let (analysis_text, usage) = self
            .analyze_image(
                &config.api_key,
                &config.model,
                config.max_tokens,
                image_data,
                &prompt,
            )
            .await
            .context("Failed to analyze image with Claude Vision")?;

        // Parse result
        let analysis_result = Self::parse_analysis_result(&analysis_text).unwrap_or_else(|_| {
            // If JSON parsing fails, return the raw text
            json!({
                "raw_analysis": analysis_text,
                "parsing_note": "Could not parse as JSON, returning raw text"
            })
        });

        // Add metadata
        let mut result_with_metadata = analysis_result.clone();
        result_with_metadata["analyzed_at"] = json!(Utc::now().to_rfc3339());
        result_with_metadata["config"] = json!({
            "model": config.model,
            "features": config.features,
        });

        // Update metadata in database
        context
            .media_repo
            .merge_plugin_metadata(
                context.tenant_id,
                context.media_id,
                "claude_vision",
                result_with_metadata.clone(),
            )
            .await
            .context("Failed to update metadata: media not found or unauthorized")?;

        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Claude Vision analysis completed successfully"
        );

        let plugin_usage = usage.map(|u| PluginUsage {
            unit_type: "tokens".to_string(),
            input_units: Some(u.input_tokens as i64),
            output_units: Some(u.output_tokens as i64),
            total_units: (u.input_tokens + u.output_tokens) as i64,
            raw_usage: Some(json!({
                "input_tokens": u.input_tokens,
                "output_tokens": u.output_tokens,
            })),
        });

        Ok(PluginResult {
            status: PluginExecutionStatus::Success,
            data: result_with_metadata,
            error: None,
            metadata: Some(json!({
                "model": config.model,
                "features": config.features,
            })),
            usage: plugin_usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_media_type_jpeg() {
        let jpeg_magic = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_media_type(&jpeg_magic), "image/jpeg");
    }

    #[test]
    fn test_detect_media_type_png() {
        let png_magic = vec![0x89, 0x50, 0x4E, 0x47];
        assert_eq!(detect_media_type(&png_magic), "image/png");
    }

    #[test]
    fn test_detect_media_type_gif() {
        let gif_magic = vec![0x47, 0x49, 0x46, 0x38];
        assert_eq!(detect_media_type(&gif_magic), "image/gif");
    }

    #[test]
    fn test_build_analysis_prompt() {
        let features = vec!["objects".to_string(), "text".to_string()];
        let prompt = ClaudeVisionPlugin::build_analysis_prompt(&features);
        assert!(prompt.contains("objects"));
        assert!(prompt.contains("text"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_parse_analysis_result_plain_json() {
        let json_text = r#"{"objects": ["cat", "dog"], "colors": ["brown", "white"]}"#;
        let result = ClaudeVisionPlugin::parse_analysis_result(json_text);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.get("objects").is_some());
    }

    #[test]
    fn test_parse_analysis_result_markdown() {
        let markdown_json = r#"
Here's the analysis:
```json
{"objects": ["cat", "dog"], "colors": ["brown", "white"]}
```
"#;
        let result = ClaudeVisionPlugin::parse_analysis_result(markdown_json);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.get("objects").is_some());
    }
}
