mod parser;
mod transformer;

use parser::parse_operations;
use transformer::apply_transformations;

use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::StreamExt;
use mindia_core::AppError;
use std::sync::Arc;
use uuid::Uuid;

/// Resolve preset references in operations string.
/// If the operations contain a `-/preset/{name}/` reference, expand it to the actual operations.
async fn resolve_preset_in_operations(
    operations: &str,
    tenant_id: Uuid,
    state: &AppState,
) -> Result<String, HttpAppError> {
    if !operations.contains("preset/") {
        return Ok(operations.to_string());
    }

    let parsed = parse_operations(operations).map_err(|e| {
        AppError::BadRequest(format!("Invalid transformation: {}", e))
    })?;

    let preset_name = match parsed.preset_name {
        Some(name) => name,
        None => return Ok(operations.to_string()),
    };

    let preset = state.db
        .named_transformation_repository
        .get_by_name(tenant_id, &preset_name)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, preset_name = %preset_name, "Failed to look up preset");
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound(format!("Preset not found: {}", preset_name)))?;

    // Replace the preset reference with the actual operations
    // Format: `-/preset/{name}/` -> actual operations
    let preset_pattern = format!("preset/{}", preset_name);
    let expanded = operations.replace(&preset_pattern, preset.operations.trim_matches('/'));

    // Clean up any double separators that might result
    let cleaned = expanded
        .replace("/-/-/", "/-/")
        .replace("-/-/", "-/")
        .replace("/-/-", "/-/");

    tracing::debug!(
        preset_name = %preset_name,
        original = %operations,
        expanded = %cleaned,
        "Expanded preset in operations"
    );

    Ok(cleaned)
}

#[utoipa::path(
    get,
    path = "/api/v0/images/{id}/{operations}",
    tag = "images",
    params(
        ("id" = Uuid, Path, description = "Image ID"),
        ("operations" = String, Path, description = "Transformation operations in URL format with /-/ separators (e.g., '-/resize/500x300/-/format/webp/-/quality/high')")
    ),
    responses(
        (status = 200, description = "Transformed image", content_type = "image/*"),
        (status = 400, description = "Invalid transformation parameters", body = ErrorResponse),
        (status = 404, description = "Image not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn transform_image(
    tenant_ctx: TenantContext,
    Path((id, operations)): Path<(Uuid, String)>,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, HttpAppError> {
    tracing::debug!(image_id = %id, operations = %operations, "Processing image transformation");

    let image = state
        .media
        .repository
        .get_image(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, image_id = %id, "Database error");
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Image not found".to_string()))?;

    // Download original image using Storage trait (works with any backend: S3, local, etc.)
    // Note: We need the full image in memory for processing, so we collect the stream
    let stream = state
        .media
        .storage
        .download_stream(image.storage_key())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %image.storage_key(), "Failed to retrieve from storage");
            AppError::Internal(e.to_string())
        })?;

    // Collect stream into bytes (required for image processing)
    let mut original_data = Vec::new();
    let mut stream = stream;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            tracing::error!(error = %e, storage_key = %image.storage_key(), "Failed to read chunk from storage");
            AppError::Internal(e.to_string())
        })?;
        original_data.extend_from_slice(&chunk);
    }
    let original_data = bytes::Bytes::from(original_data);

    let accept_header = headers.get(header::ACCEPT).and_then(|h| h.to_str().ok());

    let resolved_operations =
        resolve_preset_in_operations(&operations, tenant_ctx.tenant_id, &state).await?;

    let ops = parse_operations(&resolved_operations).map_err(|e| {
        AppError::BadRequest(format!("Invalid transformation: {}", e))
    })?;

    let watermark_data = if let Some(ref watermark_config) = ops.watermark {
        let watermark_image = state
            .media
            .repository
            .get_image(tenant_ctx.tenant_id, watermark_config.watermark_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, watermark_id = %watermark_config.watermark_id, "Database error");
                AppError::Internal(e.to_string())
            })?
            .ok_or_else(|| {
                AppError::NotFound(format!("Watermark image not found: {}", watermark_config.watermark_id))
            })?;

        let stream = state
            .media
            .storage
            .download_stream(watermark_image.storage_key())
            .await
            .map_err(|e| {
                tracing::error!(error = %e, storage_key = %watermark_image.storage_key(), "Failed to retrieve watermark from storage");
                AppError::Internal(e.to_string())
            })?;

        let mut watermark_bytes = Vec::new();
        let mut stream = stream;
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                tracing::error!(error = %e, storage_key = %watermark_image.storage_key(), "Failed to read watermark chunk from storage");
                AppError::Internal(e.to_string())
            })?;
            watermark_bytes.extend_from_slice(&chunk);
        }
        Some(watermark_bytes)
    } else {
        None
    };

    let accept_header_str = accept_header.map(|s| s.to_string());
    let original_content_type = image.content_type.clone();

    let (transformed_data, output_content_type) = tokio::task::spawn_blocking(move || {
        apply_transformations(
            &original_data,
            ops,
            accept_header_str.as_deref(),
            &original_content_type,
            watermark_data.as_deref(),
        )
    })
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to spawn blocking task");
        AppError::Internal(e.to_string())
    })?
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to transform image");
        AppError::Internal(e.to_string())
    })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, output_content_type)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::CONTENT_LENGTH, transformed_data.len())
        .body(Body::from(transformed_data))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build response");
            AppError::Internal(e.to_string()).into()
        })?;

    Ok(response)
}
