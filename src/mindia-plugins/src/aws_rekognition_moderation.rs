//! AWS Rekognition content moderation plugin
//!
//! This plugin uses AWS Rekognition to detect explicit or inappropriate content
//! in images and videos, helping ensure content safety.

use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_rekognition::Client as RekognitionClient;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter, Result as FmtResult};

use crate::plugin::{Plugin, PluginContext, PluginExecutionStatus, PluginResult};

/// AWS Rekognition content moderation plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsRekognitionModerationConfig {
    /// AWS region (e.g., "us-east-1")
    pub region: String,
    /// Minimum confidence threshold (0-100) for moderation labels
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,
    /// Safety threshold - content is considered unsafe if confidence exceeds this (0-100)
    #[serde(default = "default_safety_threshold")]
    pub safety_threshold: f32,
    /// S3 bucket (required for video moderation)
    #[serde(default)]
    pub s3_bucket: Option<String>,
    /// S3 key (required for video moderation, or use media filename)
    #[serde(default)]
    pub s3_key: Option<String>,
    /// Job ID for checking status of existing video moderation job
    #[serde(default)]
    pub job_id: Option<String>,
}

fn default_min_confidence() -> f32 {
    50.0
}

fn default_safety_threshold() -> f32 {
    70.0
}

/// AWS Rekognition content moderation plugin implementation
pub struct AwsRekognitionModerationPlugin;

impl Default for AwsRekognitionModerationPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for AwsRekognitionModerationPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("AwsRekognitionModerationPlugin").finish()
    }
}

impl AwsRekognitionModerationPlugin {
    pub fn new() -> Self {
        Self
    }

    /// Create Rekognition client for the given region
    async fn create_client(region: &str) -> Result<RekognitionClient> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Ok(RekognitionClient::new(&config))
    }

    /// Moderate an image using AWS Rekognition
    async fn moderate_image(
        client: &RekognitionClient,
        image_data: &[u8],
        config: &AwsRekognitionModerationConfig,
    ) -> Result<serde_json::Value> {
        use aws_sdk_rekognition::types::Image;

        let rekognition_image = Image::builder()
            .bytes(aws_sdk_rekognition::primitives::Blob::new(image_data))
            .build();

        let response = client
            .detect_moderation_labels()
            .image(rekognition_image)
            .min_confidence(config.min_confidence)
            .send()
            .await
            .context("Failed to detect moderation labels")?;

        let mut labels = Vec::new();
        let mut max_confidence: f32 = 0.0;

        for label in response.moderation_labels() {
            let name = label.name().unwrap_or("Unknown").to_string();
            let confidence = label.confidence().unwrap_or(0.0);
            max_confidence = max_confidence.max(confidence);

            labels.push(json!({
                "name": name,
                "confidence": confidence,
            }));
        }

        // Content is safe if no moderation labels found or all labels have low confidence
        let is_safe = labels.is_empty() || max_confidence < config.safety_threshold;

        Ok(json!({
            "is_safe": is_safe,
            "confidence": max_confidence,
            "labels": labels,
            "moderated_at": Utc::now().to_rfc3339(),
        }))
    }

    /// Start video moderation job (returns job ID)
    async fn moderate_video_start(
        client: &RekognitionClient,
        bucket: &str,
        key: &str,
        config: &AwsRekognitionModerationConfig,
    ) -> Result<String> {
        use aws_sdk_rekognition::types::{S3Object, Video};

        let s3_object = S3Object::builder().bucket(bucket).name(key).build();

        let video = Video::builder().s3_object(s3_object).build();

        let response = client
            .start_content_moderation()
            .video(video)
            .min_confidence(config.min_confidence)
            .send()
            .await
            .context("Failed to start content moderation job")?;

        let job_id = response
            .job_id()
            .ok_or_else(|| anyhow::anyhow!("No job ID returned"))?
            .to_string();

        Ok(job_id)
    }

    /// Get video moderation results
    async fn moderate_video_get_result(
        client: &RekognitionClient,
        job_id: &str,
        config: &AwsRekognitionModerationConfig,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_content_moderation()
            .job_id(job_id)
            .send()
            .await
            .context("Failed to get content moderation results")?;

        let status = response
            .job_status()
            .ok_or_else(|| anyhow::anyhow!("No job status returned"))?;

        match status {
            aws_sdk_rekognition::types::VideoJobStatus::InProgress => {
                return Ok(json!({
                    "status": "processing",
                    "is_safe": true, // Assume safe while processing
                    "confidence": 0.0,
                    "labels": [],
                    "error": "Moderation still in progress",
                }));
            }
            aws_sdk_rekognition::types::VideoJobStatus::Failed => {
                return Ok(json!({
                    "status": "failed",
                    "is_safe": true, // Fail open
                    "confidence": 0.0,
                    "labels": [],
                    "error": "Moderation job failed",
                }));
            }
            _ => {}
        }

        let mut labels = Vec::new();
        let mut max_confidence: f32 = 0.0;

        for label in response.moderation_labels() {
            let name = label
                .moderation_label()
                .and_then(|l| l.name())
                .unwrap_or("Unknown")
                .to_string();
            let confidence = label
                .moderation_label()
                .and_then(|l| l.confidence())
                .unwrap_or(0.0);
            max_confidence = max_confidence.max(confidence);

            labels.push(json!({
                "name": name,
                "confidence": confidence,
            }));
        }

        let is_safe = labels.is_empty() || max_confidence < config.safety_threshold;

        Ok(json!({
            "status": "completed",
            "is_safe": is_safe,
            "confidence": max_confidence,
            "labels": labels,
            "moderated_at": Utc::now().to_rfc3339(),
        }))
    }
}

#[async_trait]
impl Plugin for AwsRekognitionModerationPlugin {
    fn name(&self) -> &str {
        "aws_rekognition_moderation"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _config: AwsRekognitionModerationConfig = serde_json::from_value(config.clone())
            .context(
                "Invalid AWS Rekognition moderation configuration: missing or invalid fields",
            )?;

        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Executing AWS Rekognition content moderation plugin"
        );

        // Parse configuration
        let config: AwsRekognitionModerationConfig = serde_json::from_value(context.config.clone())
            .context("Failed to parse AWS Rekognition moderation configuration")?;

        // Create Rekognition client
        let client = Self::create_client(&config.region)
            .await
            .context("Failed to create AWS Rekognition client")?;

        // Get media from repository to determine type
        let media = context
            .media_repo
            .get(context.tenant_id, context.media_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get media: {}", e))?
            .context("Media not found")?;

        let moderation_result = match &media {
            mindia_core::models::Media::Image(image) => {
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
                    "Downloaded image (size validated), sending to AWS Rekognition for moderation"
                );

                Self::moderate_image(&client, &image_data, &config).await?
            }
            mindia_core::models::Media::Video(_video) => {
                // Check if we have job_id for status check
                if let Some(job_id) = &config.job_id {
                    // Check status of existing job
                    Self::moderate_video_get_result(&client, job_id, &config).await?
                } else {
                    // Start new job - need S3 bucket/key
                    let bucket = config.s3_bucket.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("S3 bucket required for video moderation")
                    })?;

                    // Get storage_key - try from config first, then from media metadata
                    let key = if let Some(key) = config.s3_key.as_ref() {
                        key.clone()
                    } else {
                        // Try to get from media metadata
                        let metadata = context
                            .media_repo
                            .get_metadata(context.tenant_id, context.media_id)
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to get media metadata: {}", e))?
                            .unwrap_or_default();

                        // Try to get storage_key from metadata
                        // Storage key might be in different places depending on how it was stored
                        metadata
                            .get("storage_key")
                            .or_else(|| {
                                metadata.get("plugins")
                                    .and_then(|p| p.get("content_moderation"))
                                    .and_then(|cm| cm.get("storage_key"))
                            })
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .ok_or_else(|| anyhow::anyhow!("S3 key required for video moderation. Provide s3_key in plugin config."))?
                    };

                    tracing::info!(
                        bucket = %bucket,
                        key = %key,
                        "Starting video content moderation job"
                    );

                    let job_id = Self::moderate_video_start(&client, bucket, &key, &config).await?;

                    // Return processing status with job_id
                    json!({
                        "status": "processing",
                        "job_id": job_id,
                        "message": "Video moderation job started",
                        "is_safe": true, // Assume safe while processing
                        "confidence": 0.0,
                        "labels": [],
                    })
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Content moderation not supported for media type: {:?}",
                    media.media_type()
                ));
            }
        };

        // Store moderation results in metadata
        let moderation_metadata = json!({
            "content_moderation": moderation_result.clone(),
        });

        context
            .media_repo
            .merge_plugin_metadata(
                context.tenant_id,
                context.media_id,
                "aws_rekognition_moderation",
                moderation_metadata,
            )
            .await
            .context("Failed to update metadata: media not found or unauthorized")?;

        let is_safe = moderation_result
            .get("is_safe")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        tracing::info!(
            media_id = %context.media_id,
            is_safe = is_safe,
            "Content moderation completed"
        );

        Ok(PluginResult {
            status: PluginExecutionStatus::Success,
            data: moderation_result.clone(),
            error: None,
            metadata: Some(json!({
                "is_safe": is_safe,
            })),
            usage: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_config_success() {
        let plugin = AwsRekognitionModerationPlugin::new();
        let config = json!({
            "region": "us-east-1",
            "min_confidence": 50.0,
            "safety_threshold": 70.0
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_validate_config_defaults() {
        let plugin = AwsRekognitionModerationPlugin::new();
        let config = json!({
            "region": "us-west-2"
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Minimal config should use defaults");

        let config_obj: AwsRekognitionModerationConfig = serde_json::from_value(config).unwrap();
        assert_eq!(config_obj.min_confidence, 50.0);
        assert_eq!(config_obj.safety_threshold, 70.0);
    }

    #[test]
    fn test_validate_config_missing_region() {
        let plugin = AwsRekognitionModerationPlugin::new();
        let config = json!({
            "min_confidence": 50.0
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_err(),
            "Config without region should fail validation"
        );
    }
}
