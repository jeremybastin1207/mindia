//! Batch media operations: delete and copy (duplicate) multiple media items in one request.

use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::handlers::media_delete::{delete_audio_embeddings, delete_video_hls_files};
use crate::middleware::audit;
use crate::state::AppState;
use axum::{extract::State, Json};
use chrono::Utc;
use mindia_core::models::{
    Media, MediaType, WebhookDataInfo, WebhookEventType, WebhookInitiatorInfo,
};
use mindia_core::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

const MAX_BATCH_SIZE: usize = 50;

#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchMediaRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchDeleteResponse {
    pub results: Vec<BatchDeleteResult>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchDeleteResult {
    pub id: Uuid,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchCopyResponse {
    pub results: Vec<BatchCopyResult>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchCopyResult {
    pub source_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_id: Option<Uuid>,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v0/media/batch/delete",
    tag = "media",
    request_body = BatchMediaRequest,
    responses(
        (status = 200, description = "Batch delete completed", body = BatchDeleteResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn batch_delete_media(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BatchMediaRequest>,
) -> Result<Json<BatchDeleteResponse>, HttpAppError> {
    if body.ids.len() > MAX_BATCH_SIZE {
        return Err(HttpAppError::from(AppError::BadRequest(format!(
            "Batch size exceeds maximum of {}",
            MAX_BATCH_SIZE
        ))));
    }

    // Client IP not available without Request (would conflict with Json body extraction)
    let client_ip: Option<String> = None;

    let mut results = Vec::with_capacity(body.ids.len());

    for id in body.ids {
        let result = match state.media.repository.get(tenant_ctx.tenant_id, id).await {
            Ok(Some(media)) => {
                let media_type = media.media_type();
                let hls_master_playlist = match &media {
                    Media::Video(v) => v.hls_master_playlist.clone(),
                    _ => None,
                };

                match media_type {
                    MediaType::Video => {
                        delete_video_hls_files(
                            &state.media.storage,
                            id,
                            hls_master_playlist.as_ref(),
                        )
                        .await;
                    }
                    MediaType::Audio => {
                        delete_audio_embeddings(
                            &state.db.embedding_repository,
                            tenant_ctx.tenant_id,
                            id,
                        )
                        .await;
                    }
                    MediaType::Image | MediaType::Document => {}
                }

                let filename = match &media {
                    Media::Image(img) => img.original_filename.clone(),
                    Media::Video(v) => v.original_filename.clone(),
                    Media::Audio(a) => a.original_filename.clone(),
                    Media::Document(d) => d.original_filename.clone(),
                };

                match state
                    .media
                    .repository
                    .delete(tenant_ctx.tenant_id, id)
                    .await
                {
                    Ok(true) => {
                        audit::log_file_deleted(
                            tenant_ctx.tenant_id,
                            Some(tenant_ctx.user_id),
                            id,
                            filename,
                            client_ip.clone(),
                        );

                        let (
                            entity_type,
                            original_filename,
                            s3_url,
                            content_type,
                            file_size,
                            uploaded_at,
                            processing_status,
                        ) = match &media {
                            Media::Image(img) => (
                                "image".to_string(),
                                img.original_filename.clone(),
                                img.storage_url().to_string(),
                                img.content_type.clone(),
                                img.file_size,
                                Some(img.uploaded_at),
                                None,
                            ),
                            Media::Video(vid) => (
                                "video".to_string(),
                                vid.original_filename.clone(),
                                vid.storage_url().to_string(),
                                vid.content_type.clone(),
                                vid.file_size,
                                Some(vid.uploaded_at),
                                Some(vid.processing_status.to_string()),
                            ),
                            Media::Audio(aud) => (
                                "audio".to_string(),
                                aud.original_filename.clone(),
                                aud.storage_url().to_string(),
                                aud.content_type.clone(),
                                aud.file_size,
                                Some(aud.uploaded_at),
                                None,
                            ),
                            Media::Document(doc) => (
                                "document".to_string(),
                                doc.original_filename.clone(),
                                doc.storage_url().to_string(),
                                doc.content_type.clone(),
                                doc.file_size,
                                Some(doc.uploaded_at),
                                None,
                            ),
                        };

                        let webhook_data = WebhookDataInfo {
                            id,
                            filename: original_filename,
                            url: s3_url,
                            content_type,
                            file_size,
                            entity_type,
                            uploaded_at,
                            deleted_at: Some(Utc::now()),
                            stored_at: None,
                            processing_status,
                            error_message: None,
                        };
                        let webhook_initiator = WebhookInitiatorInfo {
                            initiator_type: String::from("delete"),
                            id: tenant_ctx.tenant_id,
                        };
                        let webhook_service = state.webhooks.webhook_service.clone();
                        let tenant_id = tenant_ctx.tenant_id;
                        tokio::spawn(async move {
                            let _ = webhook_service
                                .trigger_event(
                                    tenant_id,
                                    WebhookEventType::FileDeleted,
                                    webhook_data,
                                    webhook_initiator,
                                )
                                .await;
                        });

                        BatchDeleteResult {
                            id,
                            status: 204,
                            error: None,
                        }
                    }
                    Ok(false) => BatchDeleteResult {
                        id,
                        status: 404,
                        error: Some("Media not found".to_string()),
                    },
                    Err(e) => BatchDeleteResult {
                        id,
                        status: 500,
                        error: Some(e.to_string()),
                    },
                }
            }
            Ok(None) => BatchDeleteResult {
                id,
                status: 404,
                error: Some("Media not found".to_string()),
            },
            Err(e) => BatchDeleteResult {
                id,
                status: 500,
                error: Some(e.to_string()),
            },
        };
        results.push(result);
    }

    Ok(Json(BatchDeleteResponse { results }))
}

#[utoipa::path(
    post,
    path = "/api/v0/media/batch/copy",
    tag = "media",
    request_body = BatchMediaRequest,
    responses(
        (status = 200, description = "Batch copy completed", body = BatchCopyResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn batch_copy_media(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BatchMediaRequest>,
) -> Result<Json<BatchCopyResponse>, HttpAppError> {
    if body.ids.len() > MAX_BATCH_SIZE {
        return Err(AppError::BadRequest(format!(
            "Batch size exceeds maximum of {}",
            MAX_BATCH_SIZE
        ))
        .into());
    }

    let mut results = Vec::with_capacity(body.ids.len());

    for source_id in body.ids {
        let result = match state
            .media
            .repository
            .copy_media(tenant_ctx.tenant_id, source_id)
            .await
        {
            Ok(media) => {
                let new_id = media.id();
                let (
                    entity_type,
                    original_filename,
                    url,
                    content_type,
                    file_size,
                    uploaded_at,
                    processing_status,
                ) = match &media {
                    Media::Image(img) => (
                        "image".to_string(),
                        img.original_filename.clone(),
                        img.storage_url().to_string(),
                        img.content_type.clone(),
                        img.file_size,
                        Some(img.uploaded_at),
                        None,
                    ),
                    Media::Video(vid) => (
                        "video".to_string(),
                        vid.original_filename.clone(),
                        vid.storage_url().to_string(),
                        vid.content_type.clone(),
                        vid.file_size,
                        Some(vid.uploaded_at),
                        Some(vid.processing_status.to_string()),
                    ),
                    Media::Audio(aud) => (
                        "audio".to_string(),
                        aud.original_filename.clone(),
                        aud.storage_url().to_string(),
                        aud.content_type.clone(),
                        aud.file_size,
                        Some(aud.uploaded_at),
                        None,
                    ),
                    Media::Document(doc) => (
                        "document".to_string(),
                        doc.original_filename.clone(),
                        doc.storage_url().to_string(),
                        doc.content_type.clone(),
                        doc.file_size,
                        Some(doc.uploaded_at),
                        None,
                    ),
                };

                let webhook_data = WebhookDataInfo {
                    id: new_id,
                    filename: original_filename,
                    url,
                    content_type,
                    file_size,
                    entity_type,
                    uploaded_at,
                    deleted_at: None,
                    stored_at: None,
                    processing_status,
                    error_message: None,
                };
                let webhook_initiator = WebhookInitiatorInfo {
                    initiator_type: String::from("copy"),
                    id: tenant_ctx.tenant_id,
                };
                let webhook_service = state.webhooks.webhook_service.clone();
                let tenant_id = tenant_ctx.tenant_id;
                tokio::spawn(async move {
                    let _ = webhook_service
                        .trigger_event(
                            tenant_id,
                            WebhookEventType::FileUploaded,
                            webhook_data,
                            webhook_initiator,
                        )
                        .await;
                });

                BatchCopyResult {
                    source_id,
                    new_id: Some(new_id),
                    status: 201,
                    error: None,
                }
            }
            Err(e) => {
                let (status, msg) =
                    if e.to_string().contains("not found") || matches!(e, AppError::NotFound(_)) {
                        (404, "Media not found".to_string())
                    } else {
                        (500, e.to_string())
                    };
                BatchCopyResult {
                    source_id,
                    new_id: None,
                    status,
                    error: Some(msg),
                }
            }
        };
        results.push(result);
    }

    Ok(Json(BatchCopyResponse { results }))
}
