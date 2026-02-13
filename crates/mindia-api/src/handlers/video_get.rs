use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use mindia_core::models::VideoResponse;
use mindia_core::AppError;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
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
    path = "/api/v0/videos/{id}",
    tag = "videos",
    params(
        ("id" = Uuid, Path, description = "Video ID")
    ),
    responses(
        (status = 200, description = "Video found", body = VideoResponse),
        (status = 404, description = "Video not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        video_id = %id,
        operation = "get_video"
    )
)]
pub async fn get_video(
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let video = state
        .media
        .repository
        .get_video(tenant_ctx.tenant_id, id)
        .await
        .map_err(HttpAppError::from)?
        .ok_or_else(|| AppError::NotFound("Video not found".to_string()))?;

    let response = state
        .media
        .repository
        .build_video_response(tenant_ctx.tenant_id, video)
        .await?;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v0/videos",
    tag = "videos",
    params(
        ListQuery
    ),
    responses(
        (status = 200, description = "List of videos", body = Vec<VideoResponse>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state, params),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        limit = params.limit,
        offset = params.offset,
        folder_id = ?params.folder_id,
        operation = "list_videos"
    )
)]
pub async fn list_videos(
    tenant_ctx: TenantContext,
    Query(params): Query<ListQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let videos = state
        .media
        .repository
        .list_videos(tenant_ctx.tenant_id, limit, offset, params.folder_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list videos");
            HttpAppError::from(e)
        })?;

    let mut responses = Vec::new();
    for video in videos {
        match state
            .media
            .repository
            .build_video_response(tenant_ctx.tenant_id, video)
            .await
        {
            Ok(response) => responses.push(response),
            Err(e) => {
                tracing::error!(error = %e, "Failed to build video response");
                return Err(HttpAppError::from(AppError::Internal(format!(
                    "Failed to build video response: {}",
                    e
                ))));
            }
        }
    }

    Ok(Json(responses))
}
