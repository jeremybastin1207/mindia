use crate::plugins::{PluginContext, PluginRegistry};
use crate::state::AppState;
use anyhow::{Context, Result};
use async_trait::async_trait;
use mindia_core::models::{
    ContentModerationPayload, Task, TaskType, WebhookDataInfo, WebhookEventType,
    WebhookInitiatorInfo,
};
use serde_json::json;
use std::sync::Arc;

/// Task handler for content moderation (uses plugin system)
#[derive(Clone)]
pub struct ContentModerationTaskHandler {
    plugin_registry: Arc<PluginRegistry>,
}

impl ContentModerationTaskHandler {
    pub fn new(plugin_registry: Arc<PluginRegistry>) -> Self {
        Self { plugin_registry }
    }
}

#[async_trait]
impl crate::task_handlers::TaskHandler for ContentModerationTaskHandler {
    async fn process(&self, task: &Task, state: Arc<AppState>) -> Result<serde_json::Value> {
        if task.task_type != TaskType::ContentModeration {
            return Err(anyhow::anyhow!(
                "Invalid task type for content moderation handler"
            ));
        }

        let payload: ContentModerationPayload = serde_json::from_value(task.payload.clone())?;

        tracing::info!(
            media_id = %payload.media_id,
            media_type = %payload.media_type,
            "Processing content moderation"
        );

        // AWS Rekognition video moderation requires S3; skip gracefully when using local storage
        if payload.media_type == "video" && state.s3.is_none() {
            tracing::info!(
                media_id = %payload.media_id,
                "Skipping video content moderation (S3 not configured; use S3 backend for video moderation)"
            );
            return Ok(json!({
                "status": "skipped",
                "is_safe": true,
                "confidence": 0.0,
                "labels": [],
                "message": "Video content moderation requires S3 storage; skipped with local storage"
            }));
        }

        // Get the moderation plugin from registry
        // If not available, skip gracefully (plugin may not be enabled/configured)
        let plugin = match self.plugin_registry.get("aws_rekognition_moderation").await {
            Ok(plugin) => plugin,
            Err(_) => {
                tracing::info!(
                    media_id = %payload.media_id,
                    media_type = %payload.media_type,
                    "Skipping content moderation (AWS Rekognition plugin not configured)"
                );
                return Ok(json!({
                    "status": "skipped",
                    "is_safe": true,
                    "confidence": 0.0,
                    "labels": [],
                    "message": "Content moderation plugin not configured; skipped"
                }));
            }
        };

        // Extract bucket from S3Config if available (content moderation currently requires S3)
        let bucket = state.s3.as_ref().map(|s3| s3.bucket.clone());

        // Build plugin configuration
        let aws_region = state
            .s3
            .as_ref()
            .map(|s3| s3.region.clone())
            .or_else(|| {
                // Try to get from config
                std::env::var("AWS_REGION")
                    .ok()
                    .or_else(|| std::env::var("S3_REGION").ok())
            })
            .unwrap_or_else(|| "us-east-1".to_string());

        let mut plugin_config = json!({
            "region": aws_region,
            "min_confidence": 50.0,
            "safety_threshold": 70.0,
        });

        // Add S3 info for videos
        if payload.media_type == "video" {
            if let Some(bucket) = &bucket {
                plugin_config["s3_bucket"] = json!(bucket);
            }
            plugin_config["s3_key"] = json!(payload.s3_key);
        }

        // Create plugin context
        let context = PluginContext {
            tenant_id: task.tenant_id,
            media_id: payload.media_id,
            storage: state.media.storage.clone(),
            media_repo: Arc::new(state.media.repository.clone()),
            file_group_repo: Arc::new(state.media.file_group_repository.clone()),
            get_public_file_url: None,
            config: plugin_config,
        };

        // Execute plugin
        let plugin_result = plugin
            .execute(context)
            .await
            .context("Failed to execute content moderation plugin")?;

        // Extract result data
        let result_data = plugin_result.data;
        let is_safe = result_data
            .get("is_safe")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let confidence = result_data
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let labels: Vec<serde_json::Value> = result_data
            .get("labels")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Check if video moderation is still processing
        if let Some(status) = result_data.get("status").and_then(|v| v.as_str()) {
            if status == "processing" {
                if let Some(job_id) = result_data.get("job_id").and_then(|v| v.as_str()) {
                    // Store job_id in metadata for follow-up check
                    let job_metadata = json!({
                        "content_moderation": {
                            "job_id": job_id,
                            "status": "processing",
                        }
                    });

                    sqlx::query(
                        r#"
                        UPDATE media
                        SET metadata = COALESCE(metadata, '{}'::jsonb) || $1::jsonb,
                            updated_at = NOW()
                        WHERE id = $2
                        "#,
                    )
                    .bind(&job_metadata)
                    .bind(payload.media_id)
                    .execute(&state.db.pool)
                    .await?;

                    return Ok(json!({
                        "status": "processing",
                        "job_id": job_id,
                        "message": "Video moderation job started"
                    }));
                }
            }
        }

        // Trigger webhook if content is unsafe
        if !is_safe {
            let label_names: Vec<String> = labels
                .iter()
                .filter_map(|l| {
                    l.get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();

            let webhook_data = WebhookDataInfo {
                id: payload.media_id,
                filename: String::new(),
                url: payload.s3_url.clone(),
                content_type: String::new(),
                file_size: 0,
                entity_type: payload.media_type.clone(),
                uploaded_at: None,
                deleted_at: None,
                stored_at: None,
                processing_status: None,
                error_message: Some(format!(
                    "Content moderation flagged: {}",
                    label_names.join(", ")
                )),
            };

            let webhook_initiator = WebhookInitiatorInfo {
                initiator_type: String::from("content_moderation"),
                id: task.tenant_id,
            };

            let webhook_service = state.webhooks.webhook_service.clone();
            let tenant_id = task.tenant_id;
            tokio::spawn(async move {
                if let Err(e) = webhook_service
                    .trigger_event(
                        tenant_id,
                        WebhookEventType::FileProcessingFailed,
                        webhook_data,
                        webhook_initiator,
                    )
                    .await
                {
                    tracing::warn!(
                        error = %e,
                        tenant_id = %tenant_id,
                        "Failed to trigger webhook for content moderation"
                    );
                }
            });
        }

        tracing::info!(
            media_id = %payload.media_id,
            is_safe = is_safe,
            confidence = confidence,
            "Content moderation completed"
        );

        Ok(json!({
            "status": "completed",
            "is_safe": is_safe,
            "confidence": confidence,
            "labels": labels,
        }))
    }
}
