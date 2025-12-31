//! OpenAI ChatGPT API plugin for image description generation

use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::time::Duration;

use mindia_plugins::{Plugin, PluginContext, PluginExecutionStatus, PluginResult, PluginUsage};

/// OpenAI Image Description plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiImageDescriptionConfig {
    /// OpenAI API key (required)
    pub api_key: String,
    /// Model to use for image description (default: "gpt-4o")
    #[serde(default = "default_model")]
    pub model: String,
    /// Prompt for generating description (default: detailed description prompt)
    #[serde(default = "default_prompt")]
    pub prompt: String,
    /// Maximum tokens in response (default: 300)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_model() -> String {
    "gpt-4o".to_string()
}

fn default_prompt() -> String {
    "Generate a detailed description of this image, including key objects, colors, composition, and overall context.".to_string()
}

fn default_max_tokens() -> u32 {
    300
}

/// OpenAI Image Description plugin implementation
pub struct OpenAiImageDescriptionPlugin {
    http_client: reqwest::Client,
}

impl Debug for OpenAiImageDescriptionPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("OpenAiImageDescriptionPlugin").finish()
    }
}

impl Default for OpenAiImageDescriptionPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAiImageDescriptionPlugin {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "Failed to create HTTP client for OpenAI API, using default client");
                reqwest::Client::default()
            });

        Self { http_client }
    }

    /// Generate image description using OpenAI Chat Completions API
    async fn describe_image(
        &self,
        api_key: &str,
        model: &str,
        prompt: &str,
        max_tokens: u32,
        image_data: Vec<u8>,
        content_type: &str,
    ) -> Result<(String, Option<Usage>)> {
        let url = "https://api.openai.com/v1/chat/completions";

        // Encode image as base64
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(&image_data);

        // Build the request body with vision-capable format
        let request_body = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": prompt
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", content_type, image_base64)
                            }
                        }
                    ]
                }
            ],
            "max_tokens": max_tokens
        });

        tracing::debug!(
            model = model,
            image_size = image_data.len(),
            "Sending image description request to OpenAI API"
        );

        let response = self
            .http_client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse OpenAI error response for better error messages
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                if let Some(error_obj) = error_json.get("error") {
                    let error_message = error_obj
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown OpenAI error");
                    let error_type = error_obj
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("api_error");
                    return Err(anyhow::anyhow!(
                        "OpenAI API error ({}): {} - Status: {}",
                        error_type,
                        error_message,
                        status
                    ));
                }
            }

            return Err(anyhow::anyhow!(
                "OpenAI API request failed: {} - {}",
                status,
                error_text
            ));
        }

        let chat_response: ChatCompletionResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI API response")?;

        // Extract description from response
        let description = chat_response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .map(|s| s.trim().to_string())
            .context("No description in OpenAI API response")?;

        Ok((description, chat_response.usage))
    }
}

#[async_trait]
impl Plugin for OpenAiImageDescriptionPlugin {
    fn name(&self) -> &str {
        "openai_image_description"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _config: OpenAiImageDescriptionConfig = serde_json::from_value(config.clone())
            .context("Invalid OpenAI Image Description configuration: missing or invalid fields")?;

        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Executing OpenAI Image Description plugin"
        );

        // Parse configuration
        let config: OpenAiImageDescriptionConfig =
            serde_json::from_value(context.config.clone())
                .context("Failed to parse OpenAI Image Description configuration")?;

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

        tracing::info!(
            image_size = image_data.len(),
            model = %config.model,
            "Downloaded image, sending to OpenAI API for description"
        );

        // Generate description
        let (description, usage) = self
            .describe_image(
                &config.api_key,
                &config.model,
                &config.prompt,
                config.max_tokens,
                image_data,
                &image.content_type,
            )
            .await
            .context("Failed to generate image description")?;

        tracing::info!(
            description_length = description.len(),
            "Generated image description successfully"
        );

        // Build result with metadata
        let result_with_metadata = json!({
            "description": description,
            "model": config.model,
            "generated_at": Utc::now().to_rfc3339(),
            "max_tokens": config.max_tokens,
            "prompt": config.prompt,
        });

        // Update metadata in database
        context
            .media_repo
            .merge_plugin_metadata(
                context.tenant_id,
                context.media_id,
                "openai_image_description",
                result_with_metadata.clone(),
            )
            .await
            .context("Failed to update metadata: media not found or unauthorized")?;

        let plugin_usage = usage.map(|u| {
            let input = u.prompt_tokens.unwrap_or(0) as i64;
            let output = u.completion_tokens.unwrap_or(0) as i64;
            let total = u
                .total_tokens
                .map(|t| t as i64)
                .unwrap_or_else(|| input + output);
            PluginUsage {
                unit_type: "tokens".to_string(),
                input_units: Some(input),
                output_units: Some(output),
                total_units: total,
                raw_usage: Some(json!({
                    "prompt_tokens": u.prompt_tokens,
                    "completion_tokens": u.completion_tokens,
                    "total_tokens": u.total_tokens,
                })),
            }
        });

        Ok(PluginResult {
            status: PluginExecutionStatus::Success,
            data: result_with_metadata.clone(),
            error: None,
            metadata: Some(json!({
                "description_length": description.len(),
                "model": config.model,
            })),
            usage: plugin_usage,
        })
    }
}

// OpenAI Chat Completions API response types
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    #[allow(dead_code)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[allow(dead_code)]
    role: String,
    content: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Usage {
    #[allow(dead_code)]
    prompt_tokens: Option<u32>,
    #[allow(dead_code)]
    completion_tokens: Option<u32>,
    #[allow(dead_code)]
    total_tokens: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_config_success() {
        let plugin = OpenAiImageDescriptionPlugin::new();
        let config = json!({
            "api_key": "sk-test-api-key",
            "model": "gpt-4o",
            "prompt": "Describe this image in detail",
            "max_tokens": 300
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_validate_config_defaults() {
        let plugin = OpenAiImageDescriptionPlugin::new();
        // Test with minimal config - should use defaults
        let config = json!({
            "api_key": "sk-test-api-key"
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Minimal config should use defaults");

        // Verify defaults are applied
        let config_obj: OpenAiImageDescriptionConfig = serde_json::from_value(config).unwrap();
        assert_eq!(config_obj.model, "gpt-4o", "Default model should be gpt-4o");
        assert_eq!(
            config_obj.max_tokens, 300,
            "Default max_tokens should be 300"
        );
        assert!(
            !config_obj.prompt.is_empty(),
            "Default prompt should not be empty"
        );
    }

    #[test]
    fn test_validate_config_missing_api_key() {
        let plugin = OpenAiImageDescriptionPlugin::new();
        let config = json!({
            "model": "gpt-4o"
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_err(),
            "Config without api_key should fail validation"
        );
        let error_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            error_msg.contains("api_key") || error_msg.contains("missing"),
            "Error should mention api_key. Got: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_config_custom_model() {
        let plugin = OpenAiImageDescriptionPlugin::new();
        let config = json!({
            "api_key": "sk-test-api-key",
            "model": "gpt-4-vision-preview"
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Config with custom model should be valid");
    }

    #[test]
    fn test_validate_config_custom_prompt() {
        let plugin = OpenAiImageDescriptionPlugin::new();
        let config = json!({
            "api_key": "sk-test-api-key",
            "prompt": "Give a brief summary of this image"
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Config with custom prompt should be valid");
    }

    #[test]
    fn test_validate_config_custom_max_tokens() {
        let plugin = OpenAiImageDescriptionPlugin::new();
        let config = json!({
            "api_key": "sk-test-api-key",
            "max_tokens": 500
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_ok(),
            "Config with custom max_tokens should be valid"
        );

        let config_obj: OpenAiImageDescriptionConfig = serde_json::from_value(config).unwrap();
        assert_eq!(config_obj.max_tokens, 500);
    }

    #[test]
    fn test_default_prompt() {
        let prompt = default_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("description"));
    }
}
