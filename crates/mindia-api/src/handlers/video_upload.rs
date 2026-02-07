use crate::auth::models::TenantContext;
use crate::error::{error_response_with_event, ErrorResponse, HttpAppError};
use crate::middleware::json_response_with_event;
use crate::state::AppState;
use crate::telemetry::wide_event::WideEvent;
use axum::{
    extract::{Multipart, Query, State},
    response::Response,
    Json,
};
use chrono::{Duration, Utc};
use mindia_core::models::VideoResponse;
use mindia_core::models::{
    GenerateEmbeddingPayload, Priority, TaskType, VideoTranscodePayload, WebhookDataInfo,
    WebhookEventType, WebhookInitiatorInfo,
};
use mindia_core::AppError;
use mindia_services::ScanResult;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

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
    mut multipart: Multipart,
) -> Result<Response, HttpAppError> {
    // Create a wide event for this request (middleware will merge with its event)
    // We can't use WideEventCtx extractor with Multipart, so we create our own
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut wide_event = WideEvent::new(
        request_id,
        state.config.otel_service_name().to_string(),
        state.config.environment().to_string(),
        "POST".to_string(),
        "/api/v0/videos".to_string(),
        chrono::Utc::now(),
    );
    wide_event.with_tenant_context(&tenant_ctx);

    // Enrich wide event with business context
    wide_event.with_business_context(|ctx| {
        ctx.media_type = Some("video".to_string());
        ctx.operation = Some("upload".to_string());
    });

    let store_behavior = query.store.to_lowercase();
    if !["0", "1", "auto"].contains(&store_behavior.as_str()) {
        return Ok(error_response_with_event(
            HttpAppError::from(AppError::InvalidInput(
                "Invalid store parameter. Must be '0', '1', or 'auto'".to_string(),
            )),
            wide_event,
        ));
    }

    let store_permanently = match store_behavior.as_str() {
        "1" => true,
        "0" => false,
        "auto" => state.config.auto_store_enabled(),
        _ => state.config.auto_store_enabled(), // fallback
    };

    let expires_at = if !store_permanently {
        Some(Utc::now() + Duration::hours(24))
    } else {
        None
    };

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                return Ok(error_response_with_event(
                    HttpAppError::from(AppError::BadRequest(format!(
                        "Failed to read multipart: {}",
                        e
                    ))),
                    wide_event,
                ));
            }
        };
        let field_name = field.name().map(|s| s.to_string()).unwrap_or_default();

        if field_name == "file" {
            filename = field.file_name().map(|s: &str| s.to_string());
            content_type = field.content_type().map(|s: &str| s.to_string());

            let data = match field.bytes().await {
                Ok(data) => data,
                Err(e) => {
                    return Ok(error_response_with_event(
                        HttpAppError::from(AppError::BadRequest(format!(
                            "Failed to read file data: {}",
                            e
                        ))),
                        wide_event,
                    ));
                }
            };

            file_data = Some(data.to_vec());
        }
    }

    let file_data = match file_data {
        Some(data) => data,
        None => {
            return Ok(error_response_with_event(
                HttpAppError::from(AppError::BadRequest("No file provided".to_string())),
                wide_event,
            ));
        }
    };

    let original_filename = filename.unwrap_or_else(|| "unknown.mp4".to_string());
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    if file_data.len() > state.media.video_max_file_size {
        return Ok(error_response_with_event(
            HttpAppError::from(AppError::PayloadTooLarge(format!(
                "File size exceeds maximum allowed size of {} MB",
                state.media.video_max_file_size / 1024 / 1024
            ))),
            wide_event,
        ));
    }

    if !state
        .media
        .video_allowed_content_types
        .iter()
        .any(|ct| content_type.to_lowercase().contains(ct))
    {
        return Ok(error_response_with_event(
            HttpAppError::from(AppError::InvalidInput(format!(
                "Invalid content type. Allowed types: {}",
                state.media.video_allowed_content_types.join(", ")
            ))),
            wide_event,
        ));
    }

    let extension = original_filename
        .split('.')
        .next_back()
        .unwrap_or("mp4")
        .to_lowercase();

    if !state.media.video_allowed_extensions.contains(&extension) {
        return Ok(error_response_with_event(
            HttpAppError::from(AppError::InvalidInput(format!(
                "Invalid file extension. Allowed extensions: {}",
                state.media.video_allowed_extensions.join(", ")
            ))),
            wide_event,
        ));
    }

    // Prevent Content-Type spoofing
    if let Err(e) =
        crate::validation::validate_extension_content_type_match(&original_filename, &content_type)
    {
        return Ok(error_response_with_event(
            HttpAppError::from(AppError::BadRequest(e)),
            wide_event,
        ));
    }

    if state.security.clamav_enabled {
        if let Some(ref clamav) = state.security.clamav {
            tracing::debug!("Scanning video with ClamAV");
            match clamav.scan_bytes(&file_data).await {
                ScanResult::Clean => {
                    tracing::debug!("Video passed virus scan");
                }
                ScanResult::Infected(virus_name) => {
                    return Ok(error_response_with_event(
                        HttpAppError::from(AppError::InvalidInput(format!(
                            "File rejected: virus detected ({})",
                            virus_name
                        ))),
                        wide_event,
                    ));
                }
                ScanResult::Error(err) => {
                    tracing::debug!(
                        error = %err,
                        original_filename = %original_filename,
                        "ClamAV scan failed"
                    );
                    return Ok(error_response_with_event(
                        HttpAppError::from(AppError::Internal(
                            "Virus scanning temporarily unavailable".to_string(),
                        )),
                        wide_event,
                    ));
                }
            }
        }
    }

    let file_uuid = Uuid::new_v4();

    let safe_original_filename = original_filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>();

    let safe_original_filename = if safe_original_filename.trim().is_empty()
        || safe_original_filename.len() < 3
        || safe_original_filename == format!(".{}", extension)
    {
        format!("{}.{}", file_uuid, extension)
    } else {
        safe_original_filename
    };

    let uuid_filename = format!("{}.{}", file_uuid, extension);

    let file_size = file_data.len();

    wide_event.with_business_context(|ctx| {
        ctx.file_size = Some(file_size as u64);
    });

    // Move file_data instead of cloning for performance
    let (storage_key, storage_url) = match state
        .media
        .storage
        .upload(
            tenant_ctx.tenant_id,
            &uuid_filename,
            &content_type,
            file_data, // Move instead of clone
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::debug!(error = %e, file_uuid = %file_uuid, "Failed to upload to storage");
            return Ok(error_response_with_event(
                HttpAppError::from(AppError::S3(format!("Failed to upload file: {}", e))),
                wide_event,
            ));
        }
    };

    let video = match state
        .media
        .repository
        .create_video_from_storage(
            tenant_ctx.tenant_id,
            file_uuid,
            uuid_filename.clone(),
            safe_original_filename.clone(),
            content_type.clone(),
            file_size as i64,
            store_behavior.clone(),
            store_permanently,
            expires_at,
            None, // folder_id
            storage_key.clone(),
            storage_url.clone(),
        )
        .await
    {
        Ok(vid) => {
            // Enrich wide event with media_id after successful creation
            wide_event.with_business_context(|ctx| {
                ctx.media_id = Some(vid.id);
            });
            vid
        }
        Err(e) => {
            // Enrich wide event with error context
            wide_event.with_error(crate::telemetry::wide_event::ErrorContext {
                error_type: "DatabaseError".to_string(),
                code: Some("DATABASE_ERROR".to_string()),
                message: format!("Failed to save video to database: {}", e),
                retriable: true,
                stack_trace: None,
            });

            if let Err(cleanup_err) = state.media.storage.delete(&storage_key).await {
                tracing::debug!(
                    error = %cleanup_err,
                    storage_key = %storage_key,
                    "Failed to cleanup storage file after DB error"
                );
            }

            return Ok(error_response_with_event(HttpAppError::from(e), wide_event));
        }
    };

    let payload = VideoTranscodePayload {
        video_id: file_uuid,
    };

    let payload_json = match serde_json::to_value(&payload) {
        Ok(val) => val,
        Err(e) => {
            tracing::debug!(
                video_id = %file_uuid,
                error = %e,
                "Failed to serialize transcoding payload"
            );
            return Ok(error_response_with_event(
                HttpAppError::from(AppError::Internal(
                    "Failed to queue transcoding task".to_string(),
                )),
                wide_event,
            ));
        }
    };

    match state
        .task_queue
        .submit_task(
            video.tenant_id,
            TaskType::VideoTranscode,
            payload_json,
            Priority::Normal,
            None, // Run immediately
            None, // No dependencies
        )
        .await
    {
        Ok(task_id) => {
            wide_event.with_business_context(|ctx| {
                ctx.task_id = Some(task_id);
            });
        }
        Err(e) => {
            tracing::debug!(
                video_id = %file_uuid,
                error = %e,
                "Failed to queue transcoding task"
            );
        }
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

    let webhook_service = state.webhook_service.clone();
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
            tracing::warn!(
                error = %e,
                tenant_id = %tenant_id,
                "Failed to trigger webhook for video upload"
            );
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
            .task_queue
            .submit_task(
                video.tenant_id,
                TaskType::ContentModeration,
                moderation_payload_json,
                Priority::Normal,
                None,
                None,
            )
            .await
        {
            tracing::warn!(
                error = %e,
                video_id = %file_uuid,
                "Failed to queue content moderation task"
            );
        }
    } else {
        tracing::error!(
            video_id = %file_uuid,
            "Failed to serialize moderation payload"
        );
    }

    if state.semantic_search.is_some() {
        let embedding_payload = GenerateEmbeddingPayload {
            entity_id: file_uuid,
            entity_type: "video".to_string(),
            s3_url: storage_url.clone(),
        };

        if let Ok(embedding_payload_json) = serde_json::to_value(&embedding_payload) {
            match state
                .task_queue
                .submit_task(
                    video.tenant_id,
                    TaskType::GenerateEmbedding,
                    embedding_payload_json,
                    Priority::Low, // Lower priority than transcoding
                    None,          // Run immediately
                    None,          // No dependencies
                )
                .await
            {
                Ok(_task_id) => {
                    // Embedding task queued successfully (logged in wide event)
                }
                Err(e) => {
                    tracing::debug!(
                        error = %e,
                        file_uuid = %file_uuid,
                        "Failed to queue embedding generation task"
                    );
                }
            }
        } else {
            tracing::debug!(
                video_id = %file_uuid,
                "Failed to serialize embedding payload"
            );
            // Don't fail the upload for this - continue without embedding
        }
    }

    Ok(json_response_with_event(
        Json(VideoResponse::from(video)),
        wide_event,
    ))
}
