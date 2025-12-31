use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use mindia_core::models::{
    ProcessingStatus, Task, VideoTranscodePayload, WebhookDataInfo, WebhookEventType,
    WebhookInitiatorInfo,
};
use mindia_processing::{VideoOrchestrator, VideoOrchestratorConfig};

use super::TaskHandler;
use crate::state::AppState;
use crate::video_storage_impl::GenericVideoStorage;

pub struct VideoTaskHandler;

#[async_trait]
impl TaskHandler for VideoTaskHandler {
    #[tracing::instrument(skip(self, task, state), fields(task.id = %task.id, video.id = tracing::field::Empty))]
    async fn process(&self, task: &Task, state: Arc<AppState>) -> Result<serde_json::Value> {
        let payload: VideoTranscodePayload = serde_json::from_value(task.payload.clone())
            .context("Failed to parse video transcode payload")?;

        tracing::Span::current().record("video.id", payload.video_id.to_string());

        tracing::info!(
            video_id = %payload.video_id,
            "Processing video transcode task"
        );

        state
            .video_db
            .update_video_processing_status(
                task.tenant_id,
                payload.video_id,
                ProcessingStatus::Processing,
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to update video status to processing")?;

        let storage = Arc::new(GenericVideoStorage::new(state.media.storage.clone()));
        let config = VideoOrchestratorConfig {
            ffmpeg_path: state.media.ffmpeg_path.clone(),
            hls_segment_duration: state.media.hls_segment_duration,
            hls_variants: state.media.hls_variants.clone(),
            capacity_monitor_interval_secs: state.config.capacity_monitor_interval_secs(),
        };
        let orchestrator = VideoOrchestrator::new(
            state.video_db.clone(),
            storage,
            state.capacity_checker.clone(),
            config,
        );
        let result = orchestrator.process_video(payload.video_id).await;

        match result {
            Ok(_) => {
                tracing::info!(
                    video_id = %payload.video_id,
                    "Video transcode completed successfully"
                );

                // Get video data for webhook
                if let Ok(Some(video)) = state
                    .video_db
                    .get_video(task.tenant_id, payload.video_id)
                    .await
                {
                    // Trigger webhook for processing completed
                    let webhook_data = WebhookDataInfo {
                        id: video.id,
                        filename: video.original_filename.clone(),
                        url: video.storage_url().to_string(),
                        content_type: video.content_type.clone(),
                        file_size: video.file_size,
                        entity_type: String::from("video"),
                        uploaded_at: Some(video.uploaded_at),
                        deleted_at: None,
                        stored_at: if video.store_permanently {
                            Some(video.uploaded_at)
                        } else {
                            None
                        },
                        processing_status: Some(ProcessingStatus::Completed.to_string()),
                        error_message: None,
                    };

                    let webhook_initiator = WebhookInitiatorInfo {
                        initiator_type: String::from("task_processor"),
                        id: task.tenant_id,
                    };

                    let webhook_service = state.webhook_service.clone();
                    let tenant_id = task.tenant_id;
                    tokio::spawn(async move {
                        if let Err(e) = webhook_service
                            .trigger_event(
                                tenant_id,
                                WebhookEventType::FileProcessingCompleted,
                                webhook_data,
                                webhook_initiator,
                            )
                            .await
                        {
                            tracing::warn!(
                                error = %e,
                                tenant_id = %tenant_id,
                                "Failed to trigger webhook for video processing completed"
                            );
                        }
                    });
                }

                Ok(json!({
                    "status": "success",
                    "video_id": payload.video_id
                }))
            }
            Err(e) => {
                tracing::error!(
                    video_id = %payload.video_id,
                    error = %e,
                    "Video transcode failed"
                );

                let error_msg = e.to_string();

                // Update video status to failed
                state
                    .video_db
                    .update_video_processing_status(
                        task.tenant_id,
                        payload.video_id,
                        ProcessingStatus::Failed,
                    )
                    .await
                    .ok(); // Don't fail the task if status update fails

                // Get video data for webhook
                if let Ok(Some(video)) = state
                    .video_db
                    .get_video(task.tenant_id, payload.video_id)
                    .await
                {
                    // Trigger webhook for processing failed
                    let webhook_data = WebhookDataInfo {
                        id: video.id,
                        filename: video.original_filename.clone(),
                        url: video.storage_url().to_string(),
                        content_type: video.content_type.clone(),
                        file_size: video.file_size,
                        entity_type: String::from("video"),
                        uploaded_at: Some(video.uploaded_at),
                        deleted_at: None,
                        stored_at: if video.store_permanently {
                            Some(video.uploaded_at)
                        } else {
                            None
                        },
                        processing_status: Some(ProcessingStatus::Failed.to_string()),
                        error_message: Some(error_msg.clone()),
                    };

                    let webhook_initiator = WebhookInitiatorInfo {
                        initiator_type: String::from("task_processor"),
                        id: task.tenant_id,
                    };

                    let webhook_service = state.webhook_service.clone();
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
                                "Failed to trigger webhook for video processing failed"
                            );
                        }
                    });
                }

                Err(e)
            }
        }
    }
}
