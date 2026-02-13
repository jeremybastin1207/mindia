use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use mindia_core::models::AudioResponse;
use mindia_core::AppError;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v0/audios/{id}",
    tag = "audios",
    params(
        ("id" = Uuid, Path, description = "Audio ID")
    ),
    responses(
        (status = 200, description = "Audio found", body = AudioResponse),
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
        operation = "get_audio"
    )
)]
pub async fn get_audio(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let audio = state
        .media
        .repository
        .get_audio(tenant_ctx.tenant_id, id)
        .await
        .map_err(HttpAppError::from)?
        .ok_or_else(|| AppError::NotFound("Audio not found".to_string()))?;

    let response = state
        .media
        .repository
        .build_audio_response(tenant_ctx.tenant_id, audio)
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
    path = "/api/v0/audios",
    tag = "audios",
    params(
        PaginationQuery
    ),
    responses(
        (status = 200, description = "List of audios", body = Vec<AudioResponse>),
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
        operation = "list_audios"
    )
)]
pub async fn list_audios(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Enforce maximum limit to prevent abuse
    let limit = pagination.limit.clamp(1, 100);
    let offset = pagination.offset.max(0);

    let audios = state
        .media
        .repository
        .list_audios(tenant_ctx.tenant_id, limit, offset, pagination.folder_id)
        .await?;

    let mut responses = Vec::new();
    for audio in audios {
        let response = state
            .media
            .repository
            .build_audio_response(tenant_ctx.tenant_id, audio)
            .await?;
        responses.push(response);
    }

    Ok(Json(responses))
}
