use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use mindia_core::models::DocumentResponse;
use mindia_core::AppError;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v0/documents/{id}",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document found", body = DocumentResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let document = state
        .media
        .repository
        .get_document(tenant_ctx.tenant_id, id)
        .await
        .map_err(HttpAppError::from)?
        .ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

    let response = state
        .media
        .repository
        .build_document_response(tenant_ctx.tenant_id, document)
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
    path = "/api/v0/documents",
    tag = "documents",
    params(
        PaginationQuery
    ),
    responses(
        (status = 200, description = "List of documents", body = Vec<DocumentResponse>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Enforce maximum limit to prevent abuse
    let limit = pagination.limit.clamp(1, 100);
    let offset = pagination.offset.max(0);

    let documents = state
        .media
        .repository
        .list_documents(tenant_ctx.tenant_id, limit, offset, pagination.folder_id)
        .await?;

    let mut responses = Vec::new();
    for document in documents {
        let response = state
            .media
            .repository
            .build_document_response(tenant_ctx.tenant_id, document)
            .await?;
        responses.push(response);
    }

    Ok(Json(responses))
}
