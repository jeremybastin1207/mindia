use crate::auth::models::TenantContext;
use crate::error::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
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
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Get media using the unified repository method
    let media = state
        .media
        .repository
        .get(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, media_id = %id, "Failed to fetch media");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch media".to_string(),
                    details: None,
                    error_type: None,
                    code: "DATABASE_ERROR".to_string(),
                    recoverable: true,
                    suggested_action: Some("Retry after a short delay".to_string()),
                }),
            )
        })?;

    let media = match media {
        Some(m) => m,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Media not found".to_string(),
                    details: None,
                    error_type: None,
                    code: "NOT_FOUND".to_string(),
                    recoverable: false,
                    suggested_action: Some("Verify the media ID exists".to_string()),
                }),
            ));
        }
    };

    // Match on media type and build appropriate response
    let response: JsonValue = match media {
        mindia_core::models::Media::Image(image) => {
            let image_response = state
                .media
                .repository
                .build_image_response(tenant_ctx.tenant_id, image)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, media_id = %id, "Failed to build image response");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Failed to build image response".to_string(),
                            details: None,
                            error_type: None,
                            code: "INTERNAL_ERROR".to_string(),
                            recoverable: true,
                            suggested_action: Some("Retry after a short delay".to_string()),
                        }),
                    )
                })?;
            serde_json::to_value(image_response).map_err(|e| {
                tracing::error!(error = ?e, media_id = %id, "Failed to serialize image response");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to serialize response".to_string(),
                        details: None,
                        error_type: None,
                        code: "INTERNAL_ERROR".to_string(),
                        recoverable: true,
                        suggested_action: Some("Retry after a short delay".to_string()),
                    }),
                )
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
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Failed to build video response".to_string(),
                            details: None,
                            error_type: None,
                            code: "INTERNAL_ERROR".to_string(),
                            recoverable: true,
                            suggested_action: Some("Retry after a short delay".to_string()),
                        }),
                    )
                })?;
            serde_json::to_value(video_response).map_err(|e| {
                tracing::error!(error = ?e, media_id = %id, "Failed to serialize video response");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to serialize response".to_string(),
                        details: None,
                        error_type: None,
                        code: "INTERNAL_ERROR".to_string(),
                        recoverable: true,
                        suggested_action: Some("Retry after a short delay".to_string()),
                    }),
                )
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
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Failed to build audio response".to_string(),
                            details: None,
                            error_type: None,
                            code: "INTERNAL_ERROR".to_string(),
                            recoverable: true,
                            suggested_action: Some("Retry after a short delay".to_string()),
                        }),
                    )
                })?;
            serde_json::to_value(audio_response).map_err(|e| {
                tracing::error!(error = ?e, media_id = %id, "Failed to serialize audio response");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to serialize response".to_string(),
                        details: None,
                        error_type: None,
                        code: "INTERNAL_ERROR".to_string(),
                        recoverable: true,
                        suggested_action: Some("Retry after a short delay".to_string()),
                    }),
                )
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
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Failed to build document response".to_string(),
                            details: None,
                            error_type: None,
                            code: "INTERNAL_ERROR".to_string(),
                            recoverable: true,
                            suggested_action: Some("Retry after a short delay".to_string()),
                        }),
                    )
                })?;
            serde_json::to_value(document_response).map_err(|e| {
                tracing::error!(error = ?e, media_id = %id, "Failed to serialize document response");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to serialize response".to_string(),
                        details: None,
                        error_type: None,
                        code: "INTERNAL_ERROR".to_string(),
                        recoverable: true,
                        suggested_action: Some("Retry after a short delay".to_string()),
                    }),
                )
            })?
        }
    };

    Ok(Json(response))
}
