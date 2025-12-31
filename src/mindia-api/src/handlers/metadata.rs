use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use mindia_core::validation;
use mindia_core::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Request to update metadata
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMetadataRequest {
    /// Metadata as key-value pairs (JSON object)
    pub metadata: serde_json::Value,
}

/// Response with updated metadata
#[derive(Debug, Serialize, ToSchema)]
pub struct MetadataResponse {
    /// Media ID
    pub id: Uuid,
    /// Current metadata
    pub metadata: serde_json::Value,
}

/// Update metadata for an image
#[utoipa::path(
    put,
    path = "/api/v0/images/{id}/metadata",
    tag = "images",
    params(
        ("id" = Uuid, Path, description = "Image ID")
    ),
    request_body = UpdateMetadataRequest,
    responses(
        (status = 200, description = "Metadata updated", body = MetadataResponse),
        (status = 404, description = "Image not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_image_metadata(
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<UpdateMetadataRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    update_metadata(tenant_ctx, state, id, request).await
}

/// Update metadata for a video
#[utoipa::path(
    put,
    path = "/api/v0/videos/{id}/metadata",
    tag = "videos",
    params(
        ("id" = Uuid, Path, description = "Video ID")
    ),
    request_body = UpdateMetadataRequest,
    responses(
        (status = 200, description = "Metadata updated", body = MetadataResponse),
        (status = 404, description = "Video not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_video_metadata(
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Json(request): Json<UpdateMetadataRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    update_metadata(tenant_ctx, state, id, request).await
}

/// Update metadata for an audio file
#[utoipa::path(
    put,
    path = "/api/audios/{id}/metadata",
    tag = "audios",
    params(
        ("id" = Uuid, Path, description = "Audio ID")
    ),
    request_body = UpdateMetadataRequest,
    responses(
        (status = 200, description = "Metadata updated", body = MetadataResponse),
        (status = 404, description = "Audio not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_audio_metadata(
    Path(id): Path<Uuid>,
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(request): Json<UpdateMetadataRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    update_metadata(tenant_ctx, state, id, request).await
}

/// Update metadata for a document
#[utoipa::path(
    put,
    path = "/api/v0/documents/{id}/metadata",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = UpdateMetadataRequest,
    responses(
        (status = 200, description = "Metadata updated", body = MetadataResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_document_metadata(
    Path(id): Path<Uuid>,
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(request): Json<UpdateMetadataRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    update_metadata(tenant_ctx, state, id, request).await
}

/// Internal function to update metadata for any media type
/// Uses repository method to ensure tenant isolation and authorization
/// Replaces entire user namespace, preserves plugin namespace
async fn update_metadata(
    tenant_ctx: TenantContext,
    state: Arc<AppState>,
    id: Uuid,
    request: UpdateMetadataRequest,
) -> Result<impl IntoResponse, HttpAppError> {
    // Validate metadata is an object
    if !request.metadata.is_object() {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Metadata must be a JSON object".to_string(),
        )));
    }

    // Validate user metadata (keys, values, count limits)
    validation::validate_user_metadata(&request.metadata)
        .map_err(|e| AppError::InvalidMetadataKey(e.to_string()))?;

    // Replace entire user namespace (preserves plugins)
    let updated_metadata = state
        .media
        .repository
        .replace_user_metadata(tenant_ctx.tenant_id, id, request.metadata.clone())
        .await
        .map_err(|e| match e.to_string().as_str() {
            msg if msg.contains("not found") || msg.contains("does not belong") => {
                AppError::NotFound(format!("Media not found: {}", id))
            }
            msg if msg.contains("exceeds maximum length") || msg.contains("invalid characters") => {
                AppError::InvalidMetadataKey(msg.to_string())
            }
            msg if msg.contains("exceeds maximum") && msg.contains("keys") => {
                AppError::MetadataKeyLimitExceeded(msg.to_string())
            }
            _ => AppError::Database(sqlx::Error::RowNotFound),
        })?
        .ok_or_else(|| AppError::NotFound(format!("Media not found: {}", id)))?;

    tracing::info!(
        media_id = %id,
        tenant_id = %tenant_ctx.tenant_id,
        "User metadata updated"
    );

    Ok(Json(MetadataResponse {
        id,
        metadata: updated_metadata,
    }))
}

/// Query parameters for get_metadata
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetMetadataQuery {
    /// Filter by namespace: "user", "plugins", or omit for all
    #[param(example = "user")]
    pub namespace: Option<String>,
}

/// Get metadata for a media file
#[utoipa::path(
    get,
    path = "/api/v0/media/{id}/metadata",
    tag = "media",
    params(
        ("id" = Uuid, Path, description = "Media ID"),
        GetMetadataQuery
    ),
    responses(
        (status = 200, description = "Metadata retrieved", body = MetadataResponse),
        (status = 404, description = "Media not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_metadata(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(params): Query<GetMetadataQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Get metadata using repository method (ensures tenant isolation)
    let metadata = state
        .media
        .repository
        .get_metadata(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| match e.to_string().as_str() {
            msg if msg.contains("not found") || msg.contains("does not belong") => {
                AppError::NotFound(format!("Media not found: {}", id))
            }
            _ => AppError::Database(sqlx::Error::RowNotFound),
        })?;

    let metadata = metadata.unwrap_or_else(|| serde_json::json!({"user": {}, "plugins": {}}));

    // Filter by namespace if requested
    let filtered_metadata = if let Some(ref namespace) = params.namespace {
        match namespace.as_str() {
            "user" => metadata
                .get("user")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            "plugins" => metadata
                .get("plugins")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            _ => {
                return Err(HttpAppError::from(AppError::InvalidInput(format!(
                    "Invalid namespace: {}. Must be 'user' or 'plugins'",
                    namespace
                ))));
            }
        }
    } else {
        metadata
    };

    Ok(Json(MetadataResponse {
        id,
        metadata: filtered_metadata,
    }))
}

/// Update a single metadata key
#[utoipa::path(
    put,
    path = "/api/v0/media/{id}/metadata/{key}",
    tag = "media",
    params(
        ("id" = Uuid, Path, description = "Media ID"),
        ("key" = String, Path, description = "Metadata key name")
    ),
    request_body(content = serde_json::Value, description = "Metadata value (any JSON value)", content_type = "application/json"),
    responses(
        (status = 200, description = "Metadata key updated", body = MetadataResponse),
        (status = 400, description = "Invalid key or value", body = ErrorResponse),
        (status = 404, description = "Media not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_metadata_key(
    Path((id, key)): Path<(Uuid, String)>,
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(value): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Validate key
    validation::validate_metadata_key(&key)
        .map_err(|e| AppError::InvalidMetadataKey(e.to_string()))?;

    // Validate value
    validation::validate_metadata_value(&value)
        .map_err(|e| AppError::InvalidMetadataValue(e.to_string()))?;

    // Update single key in user namespace
    let updated_metadata = state
        .media
        .repository
        .merge_user_metadata_key(tenant_ctx.tenant_id, id, &key, value)
        .await
        .map_err(|e| match e.to_string().as_str() {
            msg if msg.contains("not found") || msg.contains("does not belong") => {
                AppError::NotFound(format!("Media not found: {}", id))
            }
            msg if msg.contains("exceeds maximum length") || msg.contains("invalid characters") => {
                AppError::InvalidMetadataKey(msg.to_string())
            }
            msg if msg.contains("exceeds maximum") && msg.contains("keys") => {
                AppError::MetadataKeyLimitExceeded(msg.to_string())
            }
            _ => AppError::Database(sqlx::Error::RowNotFound),
        })?
        .ok_or_else(|| AppError::NotFound(format!("Media not found: {}", id)))?;

    tracing::info!(
        media_id = %id,
        tenant_id = %tenant_ctx.tenant_id,
        metadata.key = %key,
        "Metadata key updated"
    );

    Ok(Json(MetadataResponse {
        id,
        metadata: updated_metadata,
    }))
}

/// Get a single metadata key value
#[utoipa::path(
    get,
    path = "/api/v0/media/{id}/metadata/{key}",
    tag = "media",
    params(
        ("id" = Uuid, Path, description = "Media ID"),
        ("key" = String, Path, description = "Metadata key name")
    ),
    responses(
        (status = 200, description = "Metadata key value", body = serde_json::Value),
        (status = 404, description = "Media or key not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_metadata_key(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path((id, key)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Get single key from user namespace
    let value = state
        .media
        .repository
        .get_user_metadata_key(tenant_ctx.tenant_id, id, &key)
        .await
        .map_err(|e| match e.to_string().as_str() {
            msg if msg.contains("not found") || msg.contains("does not belong") => {
                AppError::NotFound(format!("Media not found: {}", id))
            }
            _ => AppError::Database(sqlx::Error::RowNotFound),
        })?;

    let value = value.ok_or_else(|| {
        AppError::MetadataKeyNotFound(format!("Metadata key '{}' not found for media {}", key, id))
    })?;

    Ok(Json(value))
}

/// Delete a single metadata key
#[utoipa::path(
    delete,
    path = "/api/v0/media/{id}/metadata/{key}",
    tag = "media",
    params(
        ("id" = Uuid, Path, description = "Media ID"),
        ("key" = String, Path, description = "Metadata key name")
    ),
    responses(
        (status = 204, description = "Metadata key deleted"),
        (status = 404, description = "Media or key not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_metadata_key(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path((id, key)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Delete single key from user namespace
    let _updated_metadata = state
        .media
        .repository
        .delete_user_metadata_key(tenant_ctx.tenant_id, id, &key)
        .await
        .map_err(|e| match e.to_string().as_str() {
            msg if msg.contains("not found") || msg.contains("does not belong") => {
                AppError::NotFound(format!("Media not found: {}", id))
            }
            _ => AppError::Database(sqlx::Error::RowNotFound),
        })?;

    tracing::info!(
        media_id = %id,
        tenant_id = %tenant_ctx.tenant_id,
        metadata.key = %key,
        "Metadata key deleted"
    );

    Ok(StatusCode::NO_CONTENT)
}
