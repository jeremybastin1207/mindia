use crate::auth::models::TenantContext;
use crate::error::{error_response_with_event, ErrorResponse, HttpAppError};
use crate::middleware::json_response_with_event;
use crate::services::upload::{
    MediaLimitsConfig, MediaProcessor, MediaUploadService, VideoMetadata, VideoProcessorImpl,
};
use crate::state::AppState;
use crate::telemetry::wide_event::WideEvent;
use crate::utils::upload::parse_store_parameter;
use axum::{
    extract::{Multipart, Query, State},
    response::Response,
    Json,
};
use chrono::Utc;
use mindia_core::models::VideoResponse;
use mindia_core::models::{
    GenerateEmbeddingPayload, Priority, TaskType, VideoTranscodePayload, WebhookDataInfo,
    WebhookEventType, WebhookInitiatorInfo,
};
use mindia_core::{models::MediaType, AppError};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct StoreQuery {
    #[serde(default = "default_store")]
    store: String,
}

fn default_store() -> String {
    "auto".to_string()
}

#[utoipa::path(
    post,
    path = "/api/v0/videos",
    tag = "videos",
    params(
        ("store" = Option<String>, Query, description = "Storage behavior: '0' (temporary), '1' (permanent), 'auto' (default)")
    ),
    request_body(content = inline(Object), content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Video uploaded successfully", body = VideoResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 413, description = "File too large", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn upload_video(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(query): Query<StoreQuery>,
    multipart: Multipart,
) -> Result<Response, HttpAppError> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut wide_event = WideEvent::new(
        request_id,
        state.config.otel_service_name().to_string(),
        state.config.environment().to_string(),
        "POST".to_string(),
        "/api/v0/videos".to_string(),
        Utc::now(),
    );
    wide_event.with_tenant_context(&tenant_ctx);
    wide_event.with_business_context(|ctx| {
        ctx.media_type = Some("video".to_string());
        ctx.operation = Some("upload".to_string());
    });

    let (store_permanently, expires_at) =
        match parse_store_parameter(&query.store, state.config.auto_store_enabled()) {
            Ok(result) => result,
            Err(e) => return Ok(error_response_with_event(HttpAppError::from(e), wide_event)),
        };
    let store_behavior = query.store.clone();

    let video_limits = state.media.limits_for(MediaType::Video);
    let config = MediaLimitsConfig {
        limits: &video_limits,
        media_type_name: "video",
    };

    let service = MediaUploadService::new(&state);
    let processor: Box<dyn MediaProcessor<Metadata = VideoMetadata> + Send + Sync> =
        Box::new(VideoProcessorImpl::new());

    let (upload_data, _metadata) = match service
        .upload(
            tenant_ctx.tenant_id,
            multipart,
            &config,
            processor,
            store_permanently,
            expires_at,
            store_behavior.clone(),
        )
        .await
    {
        Ok(result) => result,
        Err(e) => return Ok(error_response_with_event(HttpAppError::from(e), wide_event)),
    };

    let file_uuid = upload_data.file_id;
    let uuid_filename = upload_data.uuid_filename.clone();
    let safe_original_filename = upload_data.safe_original_filename.clone();
    let content_type = upload_data.content_type.clone();
    let file_size = upload_data.file_size;
    let storage_key = upload_data.storage_key.clone();
    let storage_url = upload_data.storage_url.clone();

    wide_event.with_business_context(|ctx| {
        ctx.file_size = Some(file_size as u64);
    });

    let video = match state
        .media
        .repository
        .create_video_from_storage(
            tenant_ctx.tenant_id,
            file_uuid,
            uuid_filename,
            safe_original_filename,
            content_type,
            file_size,
            store_behavior.clone(),
            store_permanently,
            expires_at,
            None,
            storage_key.clone(),
            storage_url.clone(),
        )
        .await
    {
        Ok(vid) => {
            wide_event.with_business_context(|ctx| {
                ctx.media_id = Some(vid.id);
            });
            vid
        }
        Err(e) => {
            wide_event.with_error(crate::telemetry::wide_event::ErrorContext {
                error_type: "DatabaseError".to_string(),
                code: Some("DATABASE_ERROR".to_string()),
                message: format!("Failed to save video to database: {}", e),
                retriable: true,
                stack_trace: None,
            });
            let storage = state.media.storage.clone();
            tokio::spawn(async move {
                if let Err(cleanup_err) = storage.delete(&storage_key).await {
                    tracing::debug!(
                        error = %cleanup_err,
                        storage_key = %storage_key,
                        "Failed to cleanup storage file after DB error"
                    );
                }
            });
            return Ok(error_response_with_event(HttpAppError::from(e), wide_event));
        }
    };

    let payload = VideoTranscodePayload {
        video_id: file_uuid,
    };
    let payload_json = match serde_json::to_value(&payload) {
        Ok(val) => val,
        Err(e) => {
            tracing::debug!(video_id = %file_uuid, error = %e, "Failed to serialize transcoding payload");
            return Ok(error_response_with_event(
                HttpAppError::from(AppError::Internal(
                    "Failed to queue transcoding task".to_string(),
                )),
                wide_event,
            ));
        }
    };

    if let Ok(task_id) = state
        .tasks
        .task_queue
        .submit_task(
            video.tenant_id,
            TaskType::VideoTranscode,
            payload_json,
            Priority::Normal,
            None,
            None,
            false,
        )
        .await
    {
        wide_event.with_business_context(|ctx| {
            ctx.task_id = Some(task_id);
        });
    } else {
        tracing::debug!(video_id = %file_uuid, "Failed to queue transcoding task");
    }

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
        processing_status: Some(video.processing_status.to_string()),
        error_message: None,
    };
    let webhook_initiator = WebhookInitiatorInfo {
        initiator_type: String::from("upload"),
        id: video.tenant_id,
    };
    let webhook_service = state.webhooks.webhook_service.clone();
    let tenant_id = video.tenant_id;
    tokio::spawn(async move {
        if let Err(e) = webhook_service
            .trigger_event(
                tenant_id,
                WebhookEventType::FileUploaded,
                webhook_data,
                webhook_initiator,
            )
            .await
        {
            tracing::warn!(error = %e, tenant_id = %tenant_id, "Failed to trigger webhook for video upload");
        }
    });

    let moderation_payload = mindia_core::models::ContentModerationPayload {
        media_id: file_uuid,
        media_type: "video".to_string(),
        s3_key: storage_key.clone(),
        s3_url: storage_url.clone(),
    };
    if let Ok(moderation_payload_json) = serde_json::to_value(&moderation_payload) {
        if let Err(e) = state
            .tasks
            .task_queue
            .submit_task(
                video.tenant_id,
                TaskType::ContentModeration,
                moderation_payload_json,
                Priority::Normal,
                None,
                None,
                false,
            )
            .await
        {
            tracing::warn!(error = %e, video_id = %file_uuid, "Failed to queue content moderation task");
        }
    }

    #[cfg(feature = "semantic-search")]
    if state.semantic_search.is_some() {
        let embedding_payload = GenerateEmbeddingPayload {
            entity_id: file_uuid,
            entity_type: "video".to_string(),
            s3_url: storage_url,
        };
        if let Ok(embedding_payload_json) = serde_json::to_value(&embedding_payload) {
            if let Err(e) = state
                .tasks
                .task_queue
                .submit_task(
                    video.tenant_id,
                    TaskType::GenerateEmbedding,
                    embedding_payload_json,
                    Priority::Low,
                    None,
                    None,
                    false,
                )
                .await
            {
                tracing::debug!(error = %e, file_uuid = %file_uuid, "Failed to queue embedding generation task");
            }
        }
    }

    Ok(json_response_with_event(
        Json(VideoResponse::from(video)),
        wide_event,
    ))
}
