use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use mindia_core::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v0/images/{id}/file",
    tag = "images",
    params(
        ("id" = Uuid, Path, description = "Image ID")
    ),
    responses(
        (status = 200, description = "Image file", content_type = "application/octet-stream"),
        (status = 404, description = "Image not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn download_image(
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let image = state
        .media
        .repository
        .get_image(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, image_id = %id, "Database error fetching image for download");
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Image not found".to_string()))?;

    tracing::debug!(image_id = %id, storage_key = %image.storage_key(), "Proxying file from storage");

    let stream = state
        .media
        .storage
        .download_stream(image.storage_key())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %image.storage_key(), "Failed to retrieve file from storage");
            AppError::Internal(e.to_string())
        })?;

    let body_stream = stream.map(|result| {
        result.map_err(|e| std::io::Error::other(format!("Storage stream error: {}", e)))
    });

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, image.content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", image.original_filename),
        )
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from_stream(body_stream))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build response");
            AppError::Internal(e.to_string()).into()
        })?;

    Ok(response)
}
