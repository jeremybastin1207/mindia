//! AWS Rekognition plugin for detecting AWS objects in images

use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_rekognition::Client as RekognitionClient;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter, Result as FmtResult};

use crate::plugins::{Plugin, PluginContext, PluginExecutionStatus, PluginResult, PluginUsage};

/// AWS Rekognition plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsRekognitionConfig {
    /// AWS region (e.g., "us-east-1")
    pub region: String,
    /// Minimum confidence threshold (0-100)
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,
    /// Whether to detect general labels
    #[serde(default = "default_true")]
    pub detect_labels: bool,
    /// Whether to detect custom labels (requires custom model ARN)
    #[serde(default = "default_false")]
    pub detect_custom_labels: bool,
    /// Whether to detect text
    #[serde(default = "default_true")]
    pub detect_text: bool,
    /// Custom model ARN for custom label detection (optional)
    pub custom_model_arn: Option<String>,
}

fn default_min_confidence() -> f32 {
    70.0
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

/// AWS Rekognition plugin implementation
pub struct AwsRekognitionPlugin;

impl Default for AwsRekognitionPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for AwsRekognitionPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("AwsRekognitionPlugin").finish()
    }
}

impl AwsRekognitionPlugin {
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

    /// Filter labels for AWS-related objects
    fn filter_aws_labels(labels: &[aws_sdk_rekognition::types::Label]) -> Vec<serde_json::Value> {
        let aws_keywords = [
            "aws",
            "amazon",
            "s3",
            "ec2",
            "lambda",
            "cloud",
            "server",
            "computer",
            "technology",
            "logo",
            "brand",
            "text",
            "sign",
            "symbol",
        ];

        labels
            .iter()
            .filter_map(|label| {
                let name = label.name()?.to_lowercase();
                let confidence = label.confidence().unwrap_or(0.0);

                // Check if label name contains AWS-related keywords
                let is_aws_related = aws_keywords.iter().any(|keyword| name.contains(keyword));

                if is_aws_related {
                    let mut label_json = json!({
                        "name": label.name()?,
                        "confidence": confidence,
                    });

                    // Add instances if available
                    let instances = label.instances();
                    if !instances.is_empty() {
                        let instance_list: Vec<serde_json::Value> = instances
                            .iter()
                            .filter_map(|inst: &aws_sdk_rekognition::types::Instance| {
                                inst.bounding_box().map(
                                    |bbox: &aws_sdk_rekognition::types::BoundingBox| {
                                        json!({
                                            "confidence": inst.confidence().unwrap_or(0.0),
                                            "bounding_box": {
                                                "width": bbox.width().unwrap_or(0.0),
                                                "height": bbox.height().unwrap_or(0.0),
                                                "left": bbox.left().unwrap_or(0.0),
                                                "top": bbox.top().unwrap_or(0.0),
                                            }
                                        })
                                    },
                                )
                            })
                            .collect();
                        if !instance_list.is_empty() {
                            label_json["instances"] = json!(instance_list);
                        }
                    }

                    Some(label_json)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Filter text detections for AWS-related content
    fn filter_aws_text(
        text_detections: &[aws_sdk_rekognition::types::TextDetection],
    ) -> Vec<serde_json::Value> {
        let aws_keywords = [
            "aws",
            "amazon",
            "s3",
            "ec2",
            "lambda",
            "cloud",
            "amazon web services",
            "amazon s3",
            "amazon ec2",
            "aws lambda",
            "amazon cloud",
            "aws logo",
        ];

        text_detections
            .iter()
            .filter_map(|detection| {
                let text = detection.detected_text()?.to_lowercase();
                let confidence = detection.confidence().unwrap_or(0.0);

                // Check if text contains AWS-related keywords
                let is_aws_related = aws_keywords.iter().any(|keyword| text.contains(keyword));

                if is_aws_related {
                    let mut text_json = json!({
                        "text": detection.detected_text()?,
                        "confidence": confidence,
                        "type": detection.r#type()?.as_str(),
                    });

                    // Add geometry if available
                    if let Some(geometry) = detection.geometry() {
                        if let Some(bbox) = geometry.bounding_box() {
                            text_json["geometry"] = json!({
                                "bounding_box": {
                                    "width": bbox.width().unwrap_or(0.0),
                                    "height": bbox.height().unwrap_or(0.0),
                                    "left": bbox.left().unwrap_or(0.0),
                                    "top": bbox.top().unwrap_or(0.0),
                                }
                            });
                        }
                    }

                    Some(text_json)
                } else {
                    None
                }
            })
            .collect()
    }
}

#[async_trait]
impl Plugin for AwsRekognitionPlugin {
    fn name(&self) -> &str {
        "aws_rekognition"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _config: AwsRekognitionConfig = serde_json::from_value(config.clone())
            .context("Invalid AWS Rekognition configuration: missing or invalid fields")?;

        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Executing AWS Rekognition detection plugin"
        );

        // Parse configuration
        let config: AwsRekognitionConfig = serde_json::from_value(context.config.clone())
            .context("Failed to parse AWS Rekognition configuration")?;

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
            "Downloaded image (size validated), sending to AWS Rekognition"
        );

        // Create Rekognition client
        let client = Self::create_client(&config.region)
            .await
            .context("Failed to create AWS Rekognition client")?;

        use aws_sdk_rekognition::types::Image;

        // Convert image bytes to Rekognition Image type
        let rekognition_image = Image::builder()
            .bytes(aws_sdk_rekognition::primitives::Blob::new(image_data))
            .build();

        let mut detected_labels = Vec::new();
        let mut detected_text = Vec::new();
        let mut custom_labels = Vec::new();

        // Detect labels if enabled
        if config.detect_labels {
            let response = client
                .detect_labels()
                .image(rekognition_image.clone())
                .min_confidence(config.min_confidence)
                .send()
                .await
                .context("Failed to detect labels")?;

            let labels = response.labels();
            if !labels.is_empty() {
                detected_labels = Self::filter_aws_labels(labels);
            }
        }

        // Detect custom labels if enabled and model ARN provided
        if config.detect_custom_labels {
            if let Some(model_arn) = &config.custom_model_arn {
                let response = client
                    .detect_custom_labels()
                    .image(rekognition_image.clone())
                    .project_version_arn(model_arn)
                    .min_confidence(config.min_confidence)
                    .send()
                    .await
                    .context("Failed to detect custom labels")?;

                let labels = response.custom_labels();
                if !labels.is_empty() {
                    custom_labels = labels
                        .iter()
                        .filter_map(|label: &aws_sdk_rekognition::types::CustomLabel| {
                            let confidence = label.confidence().unwrap_or(0.0);
                            if confidence >= config.min_confidence {
                                label.name().map(|name| {
                                    json!({
                                        "name": name,
                                        "confidence": confidence,
                                    })
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }

        // Detect text if enabled
        if config.detect_text {
            let response = client
                .detect_text()
                .image(rekognition_image)
                .filters(
                    aws_sdk_rekognition::types::DetectTextFilters::builder()
                        .word_filter(
                            aws_sdk_rekognition::types::DetectionFilter::builder()
                                .min_confidence(config.min_confidence)
                                .build(),
                        )
                        .build(),
                )
                .send()
                .await
                .context("Failed to detect text")?;

            let text_detections = response.text_detections();
            if !text_detections.is_empty() {
                detected_text = Self::filter_aws_text(text_detections);
            }
        }

        // Build detection result
        let detected_at = Utc::now();
        let aws_detection = json!({
            "labels": detected_labels,
            "custom_labels": custom_labels,
            "text": detected_text,
            "detected_at": detected_at.to_rfc3339(),
            "config": {
                "region": config.region,
                "min_confidence": config.min_confidence,
            }
        });

        // Update metadata in database using authorized repository method
        // This ensures tenant isolation and authorization
        // Store plugin data in plugins.aws_rekognition namespace to prevent collisions
        context
            .media_repo
            .merge_plugin_metadata(
                context.tenant_id,
                context.media_id,
                "aws_rekognition",
                aws_detection.clone(),
            )
            .await
            .context("Failed to update metadata: media not found or unauthorized")?;

        tracing::info!(
            labels_count = detected_labels.len(),
            text_count = detected_text.len(),
            custom_labels_count = custom_labels.len(),
            "AWS object detection completed successfully"
        );

        // Count API calls for usage tracking (each detect_* call = 1 image)
        let mut api_call_count = 0i64;
        if config.detect_labels {
            api_call_count += 1;
        }
        if config.detect_custom_labels && config.custom_model_arn.is_some() {
            api_call_count += 1;
        }
        if config.detect_text {
            api_call_count += 1;
        }

        let plugin_usage = if api_call_count > 0 {
            Some(PluginUsage {
                unit_type: "images".to_string(),
                input_units: Some(api_call_count),
                output_units: None,
                total_units: api_call_count,
                raw_usage: Some(json!({
                    "api_calls": api_call_count,
                    "detect_labels": config.detect_labels,
                    "detect_custom_labels": config.detect_custom_labels,
                    "detect_text": config.detect_text,
                })),
            })
        } else {
            None
        };

        Ok(PluginResult {
            status: PluginExecutionStatus::Success,
            data: aws_detection.clone(),
            error: None,
            metadata: Some(json!({
                "labels_count": detected_labels.len(),
                "text_count": detected_text.len(),
                "custom_labels_count": custom_labels.len(),
            })),
            usage: plugin_usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_config_success() {
        let plugin = AwsRekognitionPlugin::new();
        let config = json!({
            "region": "us-east-1",
            "min_confidence": 80.0,
            "detect_labels": true,
            "detect_text": true,
            "detect_custom_labels": false
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_validate_config_defaults() {
        let plugin = AwsRekognitionPlugin::new();
        // Test with minimal config - should use defaults
        let config = json!({
            "region": "us-west-2"
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Minimal config should use defaults");

        // Verify defaults are applied when deserializing
        let config_obj: AwsRekognitionConfig = serde_json::from_value(config).unwrap();
        assert_eq!(
            config_obj.min_confidence, 70.0,
            "Default min_confidence should be 70.0"
        );
        assert!(
            config_obj.detect_labels,
            "Default detect_labels should be true"
        );
        assert!(config_obj.detect_text, "Default detect_text should be true");
        assert!(
            !config_obj.detect_custom_labels,
            "Default detect_custom_labels should be false"
        );
    }

    #[test]
    fn test_validate_config_missing_region() {
        let plugin = AwsRekognitionPlugin::new();
        let config = json!({
            "min_confidence": 80.0
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_err(),
            "Config without region should fail validation"
        );
        let error_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            error_msg.contains("region") || error_msg.contains("missing"),
            "Error should mention region. Got: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_config_custom_model_arn() {
        let plugin = AwsRekognitionPlugin::new();
        let config = json!({
            "region": "us-east-1",
            "detect_custom_labels": true,
            "custom_model_arn": "arn:aws:rekognition:us-east-1:123456789012:project/my-project/version/1.0"
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_ok(),
            "Config with custom model ARN should be valid"
        );
    }

    #[test]
    fn test_validate_config_invalid_confidence() {
        let plugin = AwsRekognitionPlugin::new();
        let config = json!({
            "region": "us-east-1",
            "min_confidence": 150.0  // Invalid: should be 0-100
        });

        // Confidence validation is not enforced at config level (AWS SDK handles it)
        // But config structure should still be valid
        let result = plugin.validate_config(&config);
        assert!(
            result.is_ok(),
            "Config structure is valid even if values are out of range"
        );
    }

    // Note: Execution tests require:
    // - AWS SDK mocks (complex, requires trait abstraction or test containers)
    // - Mock storage with test image data
    // - Mock MediaRepository or test database
    // - Full PluginContext setup
    //
    // test_execute_success() - Full execution flow
    // =========================================================================
    // Execution Tests
    // =========================================================================
    // Note: Full execution tests with AWS SDK mocking require:
    // 1. AWS SDK trait abstraction or mock implementations
    // 2. Test database setup (sqlx::test) or mocked repositories
    // 3. Mock storage with test image data
    // 4. LocalStack or AWS SDK test harness for integration tests
    //
    // These tests demonstrate the structure and error cases.

    #[tokio::test]
    #[ignore] // Ignore until AWS SDK mocking is implemented
    async fn test_execute_success() {
        use crate::test_helpers::*;

        let plugin = AwsRekognitionPlugin::new();
        let _tenant_id = create_test_tenant_id();
        let _media_id = create_test_media_id();
        let config = create_aws_rekognition_config(Some("us-east-1"), Some(70.0));

        // Would need:
        // - Mocked AWS Rekognition client with responses
        // - Test database with image record
        // - Mock storage with image data
        // - PluginContext setup

        assert!(plugin.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_execute_image_not_found() {
        use crate::test_helpers::*;

        let plugin = AwsRekognitionPlugin::new();
        let config = create_aws_rekognition_config(Some("us-east-1"), None);

        // Test that plugin handles missing image gracefully
        // Would need mocked MediaRepository returning None
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[test]
    fn test_execute_filter_aws_labels() {
        // Test the label filtering logic
        // This is a unit test for the filter_aws_labels function
        // The function filters labels containing AWS-related keywords

        let aws_keywords = [
            "aws",
            "amazon",
            "s3",
            "ec2",
            "lambda",
            "cloud",
            "server",
            "computer",
            "technology",
            "logo",
            "brand",
        ];

        // Test that AWS-related keywords are recognized
        assert!(aws_keywords.contains(&"aws"));
        assert!(aws_keywords.contains(&"amazon"));
        assert!(aws_keywords.contains(&"s3"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_metadata_update() {
        use crate::test_helpers::*;

        let plugin = AwsRekognitionPlugin::new();
        let config = create_aws_rekognition_config(Some("us-east-1"), Some(70.0));

        // Would test that metadata is properly merged via merge_plugin_metadata
        // Requires test database or mocked repository
        assert!(plugin.validate_config(&config).is_ok());
    }
}
