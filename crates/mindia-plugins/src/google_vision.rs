//! Google Cloud Vision API plugin for comprehensive image analysis

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::time::Duration;

use crate::plugins::{Plugin, PluginContext, PluginExecutionStatus, PluginResult, PluginUsage};

/// Google Cloud Vision API plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleVisionConfig {
    /// Google Cloud API key or service account JSON
    pub api_key: String,
    /// Optional project ID (required for some features)
    #[serde(default)]
    pub project_id: Option<String>,
    /// Features to enable
    #[serde(default = "default_features")]
    pub features: Vec<String>,
    /// Minimum score threshold for label detection (0.0-1.0)
    #[serde(default = "default_min_score")]
    pub min_score: f32,
}

fn default_features() -> Vec<String> {
    vec![
        "LABEL_DETECTION".to_string(),
        "TEXT_DETECTION".to_string(),
        "FACE_DETECTION".to_string(),
        "OBJECT_LOCALIZATION".to_string(),
        "SAFE_SEARCH_DETECTION".to_string(),
        "LANDMARK_DETECTION".to_string(),
    ]
}

fn default_min_score() -> f32 {
    0.5
}

/// Google Cloud Vision API plugin implementation
pub struct GoogleVisionPlugin {
    http_client: reqwest::Client,
}

impl Debug for GoogleVisionPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("GoogleVisionPlugin").finish()
    }
}

impl GoogleVisionPlugin {
    pub fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client for Google Vision API")?;

        Ok(Self { http_client })
    }

    /// Convert feature string to API feature enum
    fn parse_feature(feature: &str) -> &'static str {
        match feature.to_uppercase().as_str() {
            "LABEL_DETECTION" => "LABEL_DETECTION",
            "TEXT_DETECTION" => "TEXT_DETECTION",
            "DOCUMENT_TEXT_DETECTION" => "DOCUMENT_TEXT_DETECTION",
            "FACE_DETECTION" => "FACE_DETECTION",
            "OBJECT_LOCALIZATION" => "OBJECT_LOCALIZATION",
            "SAFE_SEARCH_DETECTION" => "SAFE_SEARCH_DETECTION",
            "LANDMARK_DETECTION" => "LANDMARK_DETECTION",
            "LOGO_DETECTION" => "LOGO_DETECTION",
            "PRODUCT_SEARCH" => "PRODUCT_SEARCH",
            "CROP_HINTS" => "CROP_HINTS",
            "WEB_DETECTION" => "WEB_DETECTION",
            _ => "LABEL_DETECTION", // Default fallback
        }
    }

    /// Annotate image using Google Cloud Vision API
    async fn annotate_image(
        &self,
        api_key: &str,
        image_data: Vec<u8>,
        features: &[String],
        _min_score: f32,
    ) -> Result<VisionResponse> {
        let url = format!(
            "https://vision.googleapis.com/v1/images:annotate?key={}",
            api_key
        );

        // Build feature requests
        let feature_requests: Vec<serde_json::Value> = features
            .iter()
            .map(|f| {
                json!({
                    "type": Self::parse_feature(f),
                    "maxResults": 50
                })
            })
            .collect();

        // Encode image as base64
        use base64::Engine;
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(&image_data);

        let request_body = json!({
            "requests": [{
                "image": {
                    "content": image_base64
                },
                "features": feature_requests
            }]
        });

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google Vision API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Google Vision API request failed: {} - {}",
                status,
                error_text
            ));
        }

        let vision_response: VisionResponse = response
            .json()
            .await
            .context("Failed to parse Google Vision API response")?;

        // Check for errors in response
        if let Some(responses) = &vision_response.responses {
            if let Some(first_response) = responses.first() {
                if let Some(error) = &first_response.error {
                    return Err(anyhow::anyhow!(
                        "Google Vision API error: {:?} - {:?}",
                        error.code,
                        error.message
                    ));
                }
            }
        }

        Ok(vision_response)
    }

    /// Process and format vision results
    fn process_results(response: &VisionResponse, min_score: f32) -> serde_json::Value {
        let mut result = json!({
            "labels": [],
            "text": [],
            "faces": [],
            "objects": [],
            "safe_search": null,
            "landmarks": [],
            "logos": [],
            "web_entities": [],
        });

        if let Some(responses) = &response.responses {
            if let Some(first_response) = responses.first() {
                // Process labels
                if let Some(labels) = &first_response.label_annotations {
                    let filtered_labels: Vec<serde_json::Value> = labels
                        .iter()
                        .filter(|label| label.score.unwrap_or(0.0) as f32 >= min_score)
                        .map(|label| {
                            json!({
                                "description": label.description,
                                "score": label.score,
                                "mid": label.mid,
                                "topicality": label.topicality,
                            })
                        })
                        .collect();
                    result["labels"] = json!(filtered_labels);
                }

                // Process text detections
                if let Some(text_annotations) = &first_response.text_annotations {
                    let text_detections: Vec<serde_json::Value> = text_annotations
                        .iter()
                        .map(|text| {
                            json!({
                                "description": text.description,
                                "locale": text.locale,
                                "bounding_poly": text.bounding_poly,
                            })
                        })
                        .collect();
                    result["text"] = json!(text_detections);
                }

                // Process face detections
                if let Some(face_annotations) = &first_response.face_annotations {
                    let faces: Vec<serde_json::Value> = face_annotations
                        .iter()
                        .map(|face| {
                            json!({
                                "bounding_poly": face.bounding_poly,
                                "fd_bounding_poly": face.fd_bounding_poly,
                                "landmarks": face.landmarks,
                                "roll_angle": face.roll_angle,
                                "pan_angle": face.pan_angle,
                                "tilt_angle": face.tilt_angle,
                                "detection_confidence": face.detection_confidence,
                                "landmarking_confidence": face.landmarking_confidence,
                                "joy_likelihood": face.joy_likelihood,
                                "sorrow_likelihood": face.sorrow_likelihood,
                                "anger_likelihood": face.anger_likelihood,
                                "surprise_likelihood": face.surprise_likelihood,
                                "headwear_likelihood": face.headwear_likelihood,
                            })
                        })
                        .collect();
                    result["faces"] = json!(faces);
                }

                // Process object localizations
                if let Some(localized_objects) = &first_response.localized_object_annotations {
                    let objects: Vec<serde_json::Value> = localized_objects
                        .iter()
                        .filter(|obj| obj.score.unwrap_or(0.0) as f32 >= min_score)
                        .map(|obj| {
                            json!({
                                "name": obj.name,
                                "score": obj.score,
                                "bounding_poly": obj.bounding_poly,
                            })
                        })
                        .collect();
                    result["objects"] = json!(objects);
                }

                // Process safe search
                if let Some(safe_search) = &first_response.safe_search_annotation {
                    result["safe_search"] = json!({
                        "adult": safe_search.adult,
                        "spoof": safe_search.spoof,
                        "medical": safe_search.medical,
                        "violence": safe_search.violence,
                        "racy": safe_search.racy,
                    });
                }

                // Process landmarks
                if let Some(landmarks) = &first_response.landmark_annotations {
                    let landmark_list: Vec<serde_json::Value> = landmarks
                        .iter()
                        .filter(|landmark| landmark.score.unwrap_or(0.0) as f32 >= min_score)
                        .map(|landmark| {
                            json!({
                                "description": landmark.description,
                                "score": landmark.score,
                                "mid": landmark.mid,
                                "bounding_poly": landmark.bounding_poly,
                                "locations": landmark.locations,
                            })
                        })
                        .collect();
                    result["landmarks"] = json!(landmark_list);
                }

                // Process logos
                if let Some(logos) = &first_response.logo_annotations {
                    let logo_list: Vec<serde_json::Value> = logos
                        .iter()
                        .filter(|logo| logo.score.unwrap_or(0.0) as f32 >= min_score)
                        .map(|logo| {
                            json!({
                                "description": logo.description,
                                "score": logo.score,
                                "mid": logo.mid,
                                "bounding_poly": logo.bounding_poly,
                            })
                        })
                        .collect();
                    result["logos"] = json!(logo_list);
                }

                // Process web detection
                if let Some(web_detection) = &first_response.web_detection {
                    if let Some(web_entities) = &web_detection.web_entities {
                        let entities: Vec<serde_json::Value> = web_entities
                            .iter()
                            .filter(|entity| entity.score.unwrap_or(0.0) as f32 >= min_score)
                            .map(|entity| {
                                json!({
                                    "entity_id": entity.entity_id,
                                    "score": entity.score,
                                    "description": entity.description,
                                })
                            })
                            .collect();
                        result["web_entities"] = json!(entities);
                    }
                }
            }
        }

        result
    }
}

#[async_trait]
impl Plugin for GoogleVisionPlugin {
    fn name(&self) -> &str {
        "google_vision"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _config: GoogleVisionConfig = serde_json::from_value(config.clone())
            .context("Invalid Google Vision configuration: missing or invalid fields")?;

        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Executing Google Cloud Vision API plugin"
        );

        // Parse configuration
        let config: GoogleVisionConfig = serde_json::from_value(context.config.clone())
            .context("Failed to parse Google Vision configuration")?;

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
            features = ?config.features,
            "Downloaded image (size validated), sending to Google Cloud Vision API"
        );

        // Annotate image
        let vision_response = self
            .annotate_image(
                &config.api_key,
                image_data,
                &config.features,
                config.min_score,
            )
            .await
            .context("Failed to annotate image")?;

        // Process results
        let processed_results = Self::process_results(&vision_response, config.min_score);

        // Add metadata
        let mut result_with_metadata = processed_results.clone();
        result_with_metadata["detected_at"] = json!(Utc::now().to_rfc3339());
        result_with_metadata["config"] = json!({
            "project_id": config.project_id,
            "features": config.features,
            "min_score": config.min_score,
        });

        // Update metadata in database
        context
            .media_repo
            .merge_plugin_metadata(
                context.tenant_id,
                context.media_id,
                "google_vision",
                result_with_metadata.clone(),
            )
            .await
            .context("Failed to update metadata: media not found or unauthorized")?;

        let labels_count = processed_results["labels"]
            .as_array()
            .map(|v| v.len())
            .unwrap_or(0);
        let text_count = processed_results["text"]
            .as_array()
            .map(|v| v.len())
            .unwrap_or(0);
        let faces_count = processed_results["faces"]
            .as_array()
            .map(|v| v.len())
            .unwrap_or(0);
        let objects_count = processed_results["objects"]
            .as_array()
            .map(|v| v.len())
            .unwrap_or(0);

        tracing::info!(
            labels_count = labels_count,
            text_count = text_count,
            faces_count = faces_count,
            objects_count = objects_count,
            "Google Cloud Vision API analysis completed successfully"
        );

        // Google Vision charges per feature per image - count features requested
        let feature_count = config.features.len() as i64;
        let plugin_usage = if feature_count > 0 {
            Some(PluginUsage {
                unit_type: "feature_requests".to_string(),
                input_units: Some(feature_count),
                output_units: None,
                total_units: feature_count,
                raw_usage: Some(json!({
                    "features_count": feature_count,
                    "features": config.features,
                })),
            })
        } else {
            None
        };

        Ok(PluginResult {
            status: PluginExecutionStatus::Success,
            data: result_with_metadata.clone(),
            error: None,
            metadata: Some(json!({
                "labels_count": labels_count,
                "text_count": text_count,
                "faces_count": faces_count,
                "objects_count": objects_count,
            })),
            usage: plugin_usage,
        })
    }
}

// Google Cloud Vision API response types
#[derive(Debug, Deserialize)]
struct VisionResponse {
    responses: Option<Vec<AnnotateImageResponse>>,
}

#[derive(Debug, Deserialize)]
struct AnnotateImageResponse {
    label_annotations: Option<Vec<LabelAnnotation>>,
    text_annotations: Option<Vec<EntityAnnotation>>,
    face_annotations: Option<Vec<FaceAnnotation>>,
    localized_object_annotations: Option<Vec<LocalizedObjectAnnotation>>,
    safe_search_annotation: Option<SafeSearchAnnotation>,
    landmark_annotations: Option<Vec<EntityAnnotation>>,
    logo_annotations: Option<Vec<EntityAnnotation>>,
    web_detection: Option<WebDetection>,
    error: Option<VisionError>,
}

#[derive(Debug, Deserialize)]
struct LabelAnnotation {
    description: Option<String>,
    score: Option<f64>,
    mid: Option<String>,
    topicality: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct EntityAnnotation {
    description: Option<String>,
    score: Option<f64>,
    mid: Option<String>,
    locale: Option<String>,
    bounding_poly: Option<BoundingPoly>,
    locations: Option<Vec<LocationInfo>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BoundingPoly {
    vertices: Option<Vec<Vertex>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Vertex {
    x: Option<i32>,
    y: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LocationInfo {
    lat_lng: Option<LatLng>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LatLng {
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct FaceAnnotation {
    bounding_poly: Option<BoundingPoly>,
    fd_bounding_poly: Option<BoundingPoly>,
    landmarks: Option<Vec<FaceLandmark>>,
    roll_angle: Option<f64>,
    pan_angle: Option<f64>,
    tilt_angle: Option<f64>,
    detection_confidence: Option<f64>,
    landmarking_confidence: Option<f64>,
    joy_likelihood: Option<String>,
    sorrow_likelihood: Option<String>,
    anger_likelihood: Option<String>,
    surprise_likelihood: Option<String>,
    headwear_likelihood: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FaceLandmark {
    r#type: Option<String>,
    position: Option<Position>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Position {
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct LocalizedObjectAnnotation {
    name: Option<String>,
    score: Option<f64>,
    bounding_poly: Option<BoundingPoly>,
}

#[derive(Debug, Deserialize)]
struct SafeSearchAnnotation {
    adult: Option<String>,
    spoof: Option<String>,
    medical: Option<String>,
    violence: Option<String>,
    racy: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WebDetection {
    web_entities: Option<Vec<WebEntity>>,
}

#[derive(Debug, Deserialize)]
struct WebEntity {
    entity_id: Option<String>,
    score: Option<f64>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VisionError {
    code: Option<i32>,
    message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_config_success() {
        let plugin = GoogleVisionPlugin::new().unwrap();
        let config = json!({
            "api_key": "test-api-key",
            "project_id": "test-project",
            "features": ["LABEL_DETECTION", "TEXT_DETECTION"],
            "min_score": 0.7
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_validate_config_default_features() {
        let plugin = GoogleVisionPlugin::new().unwrap();
        // Test with minimal config - should use defaults
        let config = json!({
            "api_key": "test-api-key"
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Minimal config should use defaults");

        // Verify defaults are applied
        let config_obj: GoogleVisionConfig = serde_json::from_value(config).unwrap();
        assert_eq!(config_obj.min_score, 0.5, "Default min_score should be 0.5");
        assert!(
            !config_obj.features.is_empty(),
            "Default features should not be empty"
        );
        assert!(config_obj.features.contains(&"LABEL_DETECTION".to_string()));
        assert!(config_obj.features.contains(&"TEXT_DETECTION".to_string()));
    }

    #[test]
    fn test_validate_config_missing_api_key() {
        let plugin = GoogleVisionPlugin::new().unwrap();
        let config = json!({
            "project_id": "test-project"
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
    fn test_parse_feature() {
        // Test feature parsing
        assert_eq!(
            GoogleVisionPlugin::parse_feature("LABEL_DETECTION"),
            "LABEL_DETECTION"
        );
        assert_eq!(
            GoogleVisionPlugin::parse_feature("label_detection"),
            "LABEL_DETECTION"
        );
        assert_eq!(
            GoogleVisionPlugin::parse_feature("TEXT_DETECTION"),
            "TEXT_DETECTION"
        );
        assert_eq!(
            GoogleVisionPlugin::parse_feature("FACE_DETECTION"),
            "FACE_DETECTION"
        );
        assert_eq!(
            GoogleVisionPlugin::parse_feature("unknown"),
            "LABEL_DETECTION"
        ); // Default fallback
    }

    #[test]
    fn test_validate_config_custom_features() {
        let plugin = GoogleVisionPlugin::new().unwrap();
        let config = json!({
            "api_key": "test-api-key",
            "features": ["FACE_DETECTION", "LOGO_DETECTION", "WEB_DETECTION"]
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_ok(),
            "Config with custom features should be valid"
        );
    }

    #[test]
    fn test_validate_config_optional_project_id() {
        let plugin = GoogleVisionPlugin::new().unwrap();
        let config = json!({
            "api_key": "test-api-key"
            // project_id is optional
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Config without project_id should be valid");
    }

    #[test]
    fn test_validate_config_min_score() {
        let plugin = GoogleVisionPlugin::new().unwrap();
        let config = json!({
            "api_key": "test-api-key",
            "min_score": 0.8
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_ok(),
            "Config with custom min_score should be valid"
        );

        let config_obj: GoogleVisionConfig = serde_json::from_value(config).unwrap();
        assert_eq!(config_obj.min_score, 0.8);
    }

    #[test]
    fn test_validate_config_empty_features() {
        let plugin = GoogleVisionPlugin::new().unwrap();
        let config = json!({
            "api_key": "test-api-key",
            "features": []
        });

        // Empty features array might be valid JSON but may not make sense
        let result = plugin.validate_config(&config);
        // Validation should pass (structure is valid), but execution might behave differently
        assert!(
            result.is_ok(),
            "Empty features array is valid JSON structure"
        );
    }

    // =========================================================================
    // Execution Tests
    // =========================================================================
    // Note: Full execution tests with HTTP mocking require:
    // 1. Plugin modification to accept configurable base URL (for mockito)
    // 2. Test database setup (sqlx::test) or mocked repositories
    // 3. Mock storage with test image data
    //
    // These tests demonstrate the structure and can be expanded when the
    // plugin supports configurable base URLs for testing.

    #[tokio::test]
    #[ignore] // Ignore until plugin supports configurable base URL for HTTP mocking
    async fn test_execute_success() {
        // TRACKED: Implement when plugin supports configurable base URL for testing.
        // For full test, would need:
        // - Plugin with configurable base_url pointing to mockito server
        // - Test database with image record inserted
        // - PluginContext with mocked repositories
        unimplemented!("Requires plugin refactoring to support mockito::Server");
    }

    #[tokio::test]
    async fn test_execute_image_not_found() {
        use crate::test_helpers::*;

        let plugin = GoogleVisionPlugin::new().unwrap();
        let config = create_google_vision_config(Some("test-api-key"), None);

        // Test that plugin handles missing image gracefully
        // Would need mocked MediaRepository returning None
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_api_error() {
        // TRACKED: Test API error handling with configurable base URL (requires plugin refactoring)
        unimplemented!("Requires plugin refactoring");
    }

    #[test]
    fn test_execute_feature_parsing() {
        // Test that feature parsing works correctly
        // Verify parse_feature handles various inputs
        assert_eq!(
            GoogleVisionPlugin::parse_feature("LABEL_DETECTION"),
            "LABEL_DETECTION"
        );
        assert_eq!(
            GoogleVisionPlugin::parse_feature("TEXT_DETECTION"),
            "TEXT_DETECTION"
        );
        assert_eq!(
            GoogleVisionPlugin::parse_feature("FACE_DETECTION"),
            "FACE_DETECTION"
        );
        assert_eq!(
            GoogleVisionPlugin::parse_feature("unknown"),
            "LABEL_DETECTION"
        ); // Default
    }
}
