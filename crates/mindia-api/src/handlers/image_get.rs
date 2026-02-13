use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use mindia_core::models::ImageResponse;
use mindia_core::AppError;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v0/images/{id}",
    tag = "images",
    params(
        ("id" = Uuid, Path, description = "Image ID")
    ),
    responses(
        (status = 200, description = "Image found", body = ImageResponse),
        (status = 404, description = "Image not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        image_id = %id,
        operation = "get_image"
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        image_id = %id,
        operation = "get_image"
    )
)]
pub async fn get_image(
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
) -> Result<impl IntoResponse, HttpAppError> {
    let image = state
        .media
        .repository
        .get_image(tenant_ctx.tenant_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Image not found".to_string()))?;

    // Image is already the correct type from get_image

    let response = state
        .media
        .repository
        .build_image_response(tenant_ctx.tenant_id, image)
        .await?;

    Ok(Json(response))
}

#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub folder_id: Option<Uuid>,
}

fn default_limit() -> i64 {
    50
}

#[utoipa::path(
    get,
    path = "/api/v0/images",
    tag = "images",
    params(
        PaginationQuery
    ),
    responses(
        (status = 200, description = "List of images", body = Vec<ImageResponse>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state, pagination),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        limit = pagination.limit,
        offset = pagination.offset,
        folder_id = ?pagination.folder_id,
        operation = "list_images"
    )
)]
pub async fn list_images(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Enforce maximum limit to prevent abuse
    let limit = pagination.limit.clamp(1, 100);
    let offset = pagination.offset.max(0);

    let images = state
        .media
        .repository
        .list_images(tenant_ctx.tenant_id, limit, offset, pagination.folder_id)
        .await?;

    // Build responses with folder info
    let mut responses = Vec::new();
    for image in images {
        let response = state
            .media
            .repository
            .build_image_response(tenant_ctx.tenant_id, image)
            .await?;
        responses.push(response);
    }

    Ok(Json(responses))
}
