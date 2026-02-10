//! Replicate DeOldify plugin for image colorization
//!
//! This plugin uses Replicate's DeOldify model to add color to old black and white images.
//! Model: https://replicate.com/arielreplicate/deoldify_image

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::time::Duration;
use tokio::time::sleep;

use crate::plugins::{Plugin, PluginContext, PluginExecutionStatus, PluginResult, PluginUsage};
use mindia_core::models::MediaType;
use uuid::Uuid;

const REPLICATE_API_BASE: &str = "https://api.replicate.com/v1";
const DEOLDIFY_IMAGE_MODEL: &str = "arielreplicate/deoldify_image";
const MAX_POLL_ATTEMPTS: u32 = 300; // 5 minutes with 1-second intervals
const POLL_INTERVAL_SECS: u64 = 1;

/// Replicate DeOldify plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicateDeoldifyConfig {
    /// Replicate API token
    pub api_token: String,
    /// Model version (default: latest)
    #[serde(default)]
    pub model_version: Option<String>,
    /// Rendering factor for image quality (1-40, higher = better quality but slower)
    /// Default is 35 for artistic results
    #[serde(default = "default_render_factor")]
    pub render_factor: u32,
}

fn default_render_factor() -> u32 {
    35 // Higher default for images (artistic quality)
}

/// Replicate DeOldify plugin implementation
pub struct ReplicateDeoldifyPlugin {
    http_client: reqwest::Client,
}

impl Debug for ReplicateDeoldifyPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ReplicateDeoldifyPlugin").finish()
    }
}

// Replicate API structures
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct CreatePredictionRequest {
    version: Option<String>,
    input: PredictionInput,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct PredictionInput {
    image: String, // URL to the image file
    render_factor: u32,
}

#[derive(Debug, Deserialize)]
struct PredictionResponse {
    id: String,
    status: String,
    output: Option<serde_json::Value>,
    error: Option<String>,
    #[serde(default)]
    metrics: Option<PredictionMetrics>,
}

#[derive(Debug, Deserialize)]
struct PredictionMetrics {
    predict_time: Option<f64>,
}

impl ReplicateDeoldifyPlugin {
    pub fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minutes timeout
            .build()
            .context("Failed to create HTTP client for Replicate DeOldify")?;

        Ok(Self { http_client })
    }

    /// Create a prediction on Replicate
    async fn create_prediction(
        &self,
        api_token: &str,
        image_url: &str,
        render_factor: u32,
        model_version: Option<&str>,
    ) -> Result<String> {
        let url = format!("{}/predictions", REPLICATE_API_BASE);

        // Replicate requires a non-empty `version` field. If no explicit model
        // version is provided in the config, fall back to the default DeOldify
        // model slug so the API request is valid.
        let version_to_use = model_version
            .map(|v| v.to_string())
            .unwrap_or_else(|| DEOLDIFY_IMAGE_MODEL.to_string());

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Token {}", api_token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "version": version_to_use,
                "input": {
                    "image": image_url,
                    "render_factor": render_factor,
                },
            }))
            .send()
            .await
            .context("Failed to send request to Replicate API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Replicate API request failed: {} - {}",
                status,
                error_text
            ));
        }

        let prediction: PredictionResponse = response
            .json()
            .await
            .context("Failed to parse Replicate API response")?;

        Ok(prediction.id)
    }

    /// Get prediction status
    async fn get_prediction(
        &self,
        api_token: &str,
        prediction_id: &str,
    ) -> Result<PredictionResponse> {
        let url = format!("{}/predictions/{}", REPLICATE_API_BASE, prediction_id);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Token {}", api_token))
            .send()
            .await
            .context("Failed to get prediction status from Replicate API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Failed to get prediction status: {} - {}",
                status,
                error_text
            ));
        }

        let prediction = response
            .json()
            .await
            .context("Failed to parse prediction response")?;

        Ok(prediction)
    }

    /// Wait for prediction to complete
    async fn wait_for_prediction(
        &self,
        api_token: &str,
        prediction_id: &str,
    ) -> Result<PredictionResponse> {
        for attempt in 0..MAX_POLL_ATTEMPTS {
            let prediction = self.get_prediction(api_token, prediction_id).await?;

            match prediction.status.as_str() {
                "succeeded" => {
                    tracing::info!(
                        prediction_id = %prediction_id,
                        attempts = attempt + 1,
                        "Replicate prediction completed successfully"
                    );
                    return Ok(prediction);
                }
                "failed" => {
                    let error_msg = prediction
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string());
                    return Err(anyhow::anyhow!(
                        "Replicate prediction failed: {}",
                        error_msg
                    ));
                }
                "canceled" => {
                    return Err(anyhow::anyhow!("Replicate prediction was canceled"));
                }
                "starting" | "processing" => {
                    tracing::debug!(
                        prediction_id = %prediction_id,
                        attempt = attempt + 1,
                        status = %prediction.status,
                        "Waiting for Replicate prediction to complete"
                    );
                    sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                }
                _ => {
                    tracing::warn!(
                        prediction_id = %prediction_id,
                        status = %prediction.status,
                        "Unknown prediction status"
                    );
                    sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                }
            }
        }

        Err(anyhow::anyhow!(
            "Replicate prediction timed out after {} attempts",
            MAX_POLL_ATTEMPTS
        ))
    }

    /// Download output image, store it, and create database entry
    async fn download_and_store_output(
        &self,
        output_url: &str,
        context: &PluginContext,
        original_filename: &str,
        original_width: Option<i32>,
        original_height: Option<i32>,
    ) -> Result<(Uuid, String, String)> {
        tracing::info!(
            output_url = %output_url,
            "Downloading colorized image from Replicate"
        );

        // Download the image
        let response = self
            .http_client
            .get(output_url)
            .send()
            .await
            .context("Failed to download output image from Replicate")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download output image: {}",
                response.status()
            ));
        }

        let image_data = response
            .bytes()
            .await
            .context("Failed to read output image data")?;

        let file_size = image_data.len() as i64;

        // Generate filename for the colorized image
        // Preserve original extension or default to jpg
        let extension = original_filename.rsplit('.').next().unwrap_or("jpg");
        let base_name = original_filename.trim_end_matches(&format!(".{}", extension));
        let new_filename = format!("{}_colorized.{}", base_name, extension);

        // Determine content type
        let content_type = match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "webp" => "image/webp",
            _ => "image/jpeg",
        };

        // Upload to storage - this returns (storage_key, storage_url)
        let (storage_key, storage_url) = context
            .storage
            .upload(
                context.tenant_id,
                &new_filename,
                content_type,
                image_data.to_vec(),
            )
            .await
            .context("Failed to upload colorized image to storage")?;

        tracing::info!(
            storage_key = %storage_key,
            storage_url = %storage_url,
            "Colorized image uploaded to storage"
        );

        // Create a media database entry for the colorized image
        let colorized_id = Uuid::new_v4();
        let _colorized_media = context
            .media_repo
            .create_media_entry(
                context.tenant_id,
                colorized_id,
                MediaType::Image,
                new_filename.clone(),
                storage_key.clone(),
                storage_url.clone(),
                content_type.to_string(),
                file_size,
                original_width,
                original_height,
                None, // duration (not applicable for images)
            )
            .await
            .context("Failed to create media entry for colorized image")?;

        tracing::info!(
            colorized_id = %colorized_id,
            "Created media database entry for colorized image"
        );

        Ok((colorized_id, storage_key, storage_url))
    }
}

#[async_trait]
impl Plugin for ReplicateDeoldifyPlugin {
    fn name(&self) -> &str {
        "replicate_deoldify"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let config: ReplicateDeoldifyConfig = serde_json::from_value(config.clone())
            .context("Invalid Replicate DeOldify configuration: missing or invalid fields")?;

        // Validate API token is present and not a placeholder
        if config.api_token.is_empty() {
            anyhow::bail!("Replicate API token is required but not provided");
        }

        if config.api_token == "your-api-token"
            || config.api_token == "r8_"
            || config.api_token.len() < 10
        {
            anyhow::bail!("Replicate API token appears to be invalid or a placeholder. Please provide a valid API token.");
        }

        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Executing Replicate DeOldify plugin"
        );

        // Parse configuration
        let config: ReplicateDeoldifyConfig = serde_json::from_value(context.config.clone())
            .context("Failed to parse Replicate DeOldify configuration")?;

        // Validate API token
        if config.api_token.is_empty() || config.api_token == "your-api-token" {
            return Ok(PluginResult {
                status: PluginExecutionStatus::Failed,
                data: json!({}),
                error: Some(
                    "Replicate API token not configured. Please set a valid Replicate API token."
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

        // Validate image size to prevent issues
        crate::validation::validate_image_size(&vec![0u8; image.file_size as usize])
            .context("Image file too large for processing")?;

        tracing::info!(
            image_size = image.file_size,
            render_factor = config.render_factor,
            "Image validated, preparing for colorization"
        );

        // For Replicate API, we need a publicly accessible URL to the image
        // Generate a presigned URL from storage (valid for 1 hour)
        let image_url = context
            .storage
            .get_presigned_url(image.storage_key(), Duration::from_secs(3600))
            .await
            .context("Failed to get presigned URL for image")?;

        // Create prediction
        let prediction_id = self
            .create_prediction(
                &config.api_token,
                &image_url,
                config.render_factor,
                config.model_version.as_deref(),
            )
            .await
            .context("Failed to create Replicate prediction")?;

        tracing::info!(
            prediction_id = %prediction_id,
            "Replicate prediction created, waiting for completion"
        );

        // Wait for prediction to complete
        let prediction = self
            .wait_for_prediction(&config.api_token, &prediction_id)
            .await
            .context("Failed to wait for prediction completion")?;

        // Extract output URL
        let output_url = prediction
            .output
            .as_ref()
            .and_then(|o| o.as_str())
            .context("No output URL in prediction response")?;

        // Download and store the colorized image
        let (colorized_media_id, colorized_storage_key, colorized_storage_url) = self
            .download_and_store_output(
                output_url,
                &context,
                &image.filename,
                image.width,
                image.height,
            )
            .await
            .context("Failed to download and store colorized image")?;

        // Create a file group linking the original and colorized images
        let file_group = context
            .file_group_repo
            .create_group(
                context.tenant_id,
                vec![context.media_id, colorized_media_id],
            )
            .await
            .context("Failed to create file group")?;

        tracing::info!(
            file_group_id = %file_group.id,
            original_id = %context.media_id,
            colorized_id = %colorized_media_id,
            "Created file group linking original and colorized images"
        );

        // Prepare result data
        let result_data = json!({
            "prediction_id": prediction_id,
            "replicate_output_url": output_url,
            "colorized_media_id": colorized_media_id.to_string(),
            "colorized_storage_key": colorized_storage_key,
            "colorized_storage_url": colorized_storage_url,
            "file_group_id": file_group.id.to_string(),
            "render_factor": config.render_factor,
            "processed_at": Utc::now().to_rfc3339(),
        });

        // Update metadata in database
        context
            .media_repo
            .merge_plugin_metadata(
                context.tenant_id,
                context.media_id,
                "replicate_deoldify",
                result_data.clone(),
            )
            .await
            .context("Failed to update metadata: media not found or unauthorized")?;

        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            colorized_media_id = %colorized_media_id,
            colorized_storage_key = %colorized_storage_key,
            file_group_id = %file_group.id,
            "Replicate DeOldify colorization completed successfully"
        );

        // Calculate usage if metrics are available
        let plugin_usage = prediction.metrics.map(|m| {
            let predict_time_secs = m.predict_time.unwrap_or(0.0);
            PluginUsage {
                unit_type: "seconds".to_string(),
                input_units: None,
                output_units: None,
                total_units: predict_time_secs.ceil() as i64,
                raw_usage: Some(json!({
                    "predict_time": predict_time_secs,
                })),
            }
        });

        Ok(PluginResult {
            status: PluginExecutionStatus::Success,
            data: result_data,
            error: None,
            metadata: Some(json!({
                "model": DEOLDIFY_IMAGE_MODEL,
                "render_factor": config.render_factor,
                "prediction_id": prediction_id,
            })),
            usage: plugin_usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_config_valid() {
        let plugin = ReplicateDeoldifyPlugin::new().unwrap();
        let config = json!({
            "api_token": "test-token",
            "render_factor": 25
        });
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_token() {
        let plugin = ReplicateDeoldifyPlugin::new().unwrap();
        let config = json!({
            "render_factor": 25
        });
        assert!(plugin.validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_default_render_factor() {
        let config_json = json!({
            "api_token": "test-token"
        });
        let config: ReplicateDeoldifyConfig = serde_json::from_value(config_json).unwrap();
        assert_eq!(config.render_factor, 35);
    }
}
