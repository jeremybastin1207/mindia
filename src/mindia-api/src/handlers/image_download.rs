use crate::auth::models::TenantContext;
use crate::error::ErrorResponse;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::StreamExt;
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
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let image = state
        .image
        .repository
        .get_image(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, image_id = %id, "Database error fetching image for download");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve image".to_string(),
                    details: None,
                    error_type: None,
                    code: "DATABASE_ERROR".to_string(),
                    recoverable: true,
                    suggested_action: Some("Retry after a short delay".to_string()),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Image not found".to_string(),
                    details: None,
                    error_type: None,
                    code: "NOT_FOUND".to_string(),
                    recoverable: false,
                    suggested_action: Some("Verify the image ID exists".to_string()),
                }),
            )
        })?;

    tracing::debug!(image_id = %id, storage_key = %image.storage_key(), "Proxying file from storage");

    // Download file as stream using Storage trait (works with any backend: S3, local, etc.)
    let stream = state
        .media
        .storage
        .download_stream(image.storage_key())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %image.storage_key(), "Failed to retrieve file from storage");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve file".to_string(),
                    details: None,
                    error_type: None,
                    code: "STORAGE_ERROR".to_string(),
                    recoverable: true,
                    suggested_action: Some("Retry after a short delay".to_string()),
                }),
            )
        })?;

    // Wrap storage stream for axum Body
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to build response".to_string(),
                    details: None,
                    error_type: None,
                    code: "INTERNAL_ERROR".to_string(),
                    recoverable: true,
                    suggested_action: Some("Retry after a short delay".to_string()),
                }),
            )
        })?;

    Ok(response)
}
