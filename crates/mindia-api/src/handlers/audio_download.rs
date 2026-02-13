use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Response, StatusCode},
    response::IntoResponse,
};
use futures::StreamExt;
use mindia_core::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v0/audios/{id}/file",
    tag = "audios",
    params(
        ("id" = Uuid, Path, description = "Audio ID")
    ),
    responses(
        (status = 200, description = "Audio file", content_type = "audio/*"),
        (status = 404, description = "Audio not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        audio_id = %id,
        operation = "download_audio"
    )
)]
pub async fn download_audio(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Get audio metadata from database
    let audio = state
        .media
        .repository
        .get_audio(tenant_ctx.tenant_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Audio not found".to_string()))?;

    tracing::debug!(
        audio_id = %id,
        storage_key = %audio.storage_key(),
        "Proxying audio from storage"
    );

    let stream = state
        .media
        .storage
        .download_stream(audio.storage_key())
        .await
        .map_err(|e| AppError::S3(format!("Failed to download from storage: {}", e)))?;

    // Wrap storage stream for axum Body
    let body_stream = stream.map(|result| {
        result.map_err(|e| std::io::Error::other(format!("Storage stream error: {}", e)))
    });

    let content_disposition = format!("attachment; filename=\"{}\"", audio.original_filename);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, audio.content_type.as_str())
        .header(header::CONTENT_DISPOSITION, content_disposition.as_str())
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from_stream(body_stream))
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}
