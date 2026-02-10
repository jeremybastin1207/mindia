use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use mindia_core::AppError;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v0/media/{id}",
    tag = "media",
    params(
        ("id" = Uuid, Path, description = "Media ID")
    ),
    responses(
        (status = 200, description = "Media found", body = serde_json::Value),
        (status = 404, description = "Media not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_media(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let media = state
        .media
        .repository
        .get(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, media_id = %id, "Failed to fetch media");
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Media not found".to_string()))?;

    let response: JsonValue = match media {
        mindia_core::models::Media::Image(image) => {
            let image_response = state
                .media
                .repository
                .build_image_response(tenant_ctx.tenant_id, image)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, media_id = %id, "Failed to build image response");
                    AppError::Internal(e.to_string())
                })?;
            serde_json::to_value(image_response).map_err(|e| {
                tracing::error!(error = ?e, media_id = %id, "Failed to serialize image response");
                AppError::Internal(e.to_string())
            })?
        }
        mindia_core::models::Media::Video(video) => {
            let video_response = state
                .media
                .repository
                .build_video_response(tenant_ctx.tenant_id, video)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, media_id = %id, "Failed to build video response");
                    AppError::Internal(e.to_string())
                })?;
            serde_json::to_value(video_response).map_err(|e| {
                tracing::error!(error = ?e, media_id = %id, "Failed to serialize video response");
                AppError::Internal(e.to_string())
            })?
        }
        mindia_core::models::Media::Audio(audio) => {
            let audio_response = state
                .media
                .repository
                .build_audio_response(tenant_ctx.tenant_id, audio)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, media_id = %id, "Failed to build audio response");
                    AppError::Internal(e.to_string())
                })?;
            serde_json::to_value(audio_response).map_err(|e| {
                tracing::error!(error = ?e, media_id = %id, "Failed to serialize audio response");
                AppError::Internal(e.to_string())
            })?
        }
        mindia_core::models::Media::Document(document) => {
            let document_response = state
                .media
                .repository
                .build_document_response(tenant_ctx.tenant_id, document)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, media_id = %id, "Failed to build document response");
                    AppError::Internal(e.to_string())
                })?;
            serde_json::to_value(document_response).map_err(|e| {
                tracing::error!(error = ?e, media_id = %id, "Failed to serialize document response");
                AppError::Internal(e.to_string())
            })?
        }
    };

    Ok(Json(response))
}
