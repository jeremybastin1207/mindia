use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use mindia_core::models::{CreateFileGroupRequest, FileGroupInfo, FileGroupResponse};
use mindia_core::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/v0/groups",
    tag = "file-groups",
    request_body = CreateFileGroupRequest,
    responses(
        (status = 201, description = "File group created", body = FileGroupInfo),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "File not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state, request),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        file_count = request.files.len(),
        operation = "create_file_group"
    )
)]
pub async fn create_file_group(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateFileGroupRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    let group = state
        .media
        .file_group_repository
        .create_group(tenant_ctx.tenant_id, request.files)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create file group");
            AppError::BadRequest(format!("Failed to create file group: {}", e))
        })?;

    // Get file count for response
    let (_, file_count) = state
        .media
        .file_group_repository
        .get_group_info(tenant_ctx.tenant_id, group.id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("File group not found after creation".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(FileGroupInfo {
            id: group.id,
            created_at: group.created_at,
            file_count,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v0/groups/{id}",
    tag = "file-groups",
    params(
        ("id" = Uuid, Path, description = "File group ID")
    ),
    responses(
        (status = 200, description = "File group found", body = FileGroupResponse),
        (status = 404, description = "File group not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        group_id = %id,
        operation = "get_file_group"
    )
)]
pub async fn get_file_group(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let (group, _file_count) = state
        .media
        .file_group_repository
        .get_group_info(tenant_ctx.tenant_id, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("File group not found".to_string()))?;

    let files = state
        .media
        .file_group_repository
        .get_group_files(tenant_ctx.tenant_id, id)
        .await
        .map_err(AppError::from)?;

    Ok(Json(FileGroupResponse {
        id: group.id,
        created_at: group.created_at,
        files,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v0/groups/{id}/info",
    tag = "file-groups",
    params(
        ("id" = Uuid, Path, description = "File group ID")
    ),
    responses(
        (status = 200, description = "File group info found", body = FileGroupInfo),
        (status = 404, description = "File group not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        group_id = %id,
        operation = "get_file_group_info"
    )
)]
pub async fn get_file_group_info(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let (group, file_count) = state
        .media
        .file_group_repository
        .get_group_info(tenant_ctx.tenant_id, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("File group not found".to_string()))?;

    Ok(Json(FileGroupInfo {
        id: group.id,
        created_at: group.created_at,
        file_count,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v0/groups/{id}/nth/{index}",
    tag = "file-groups",
    params(
        ("id" = Uuid, Path, description = "File group ID"),
        ("index" = i32, Path, description = "File index (0-based)")
    ),
    responses(
        (status = 302, description = "Redirect to file URL"),
        (status = 404, description = "File group or file not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        group_id = %id,
        index = index,
        operation = "get_file_by_index"
    )
)]
pub async fn get_file_by_index(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path((id, index)): Path<(Uuid, i32)>,
) -> Result<impl IntoResponse, HttpAppError> {
    if index < 0 {
        return Err(HttpAppError::from(AppError::BadRequest(
            "Index must be non-negative".to_string(),
        )));
    }

    let (_media_id, url) = state
        .media
        .file_group_repository
        .get_file_by_index(tenant_ctx.tenant_id, id, index)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                AppError::NotFound(format!("File at index {} not found in group", index))
            } else {
                AppError::from(e)
            }
        })?
        .ok_or_else(|| AppError::NotFound(format!("File at index {} not found in group", index)))?;

    // Redirect to file URL
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, url)
        .body(axum::body::Body::empty())
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?
        .into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/groups/{id}/archive/{format}",
    tag = "file-groups",
    params(
        ("id" = Uuid, Path, description = "File group ID"),
        ("format" = String, Path, description = "Archive format (zip or tar)")
    ),
    responses(
        (status = 200, description = "Archive file", content_type = "application/zip"),
        (status = 400, description = "Invalid format", body = ErrorResponse),
        (status = 404, description = "File group not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        group_id = %id,
        format = %format_str,
        operation = "get_group_archive"
    )
)]
pub async fn get_group_archive(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path((id, format_str)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, HttpAppError> {
    use mindia_infra::{create_archive, ArchiveFormat};

    let format: ArchiveFormat = format_str
        .parse()
        .map_err(|e| AppError::BadRequest(format!("Invalid archive format: {}", e)))?;

    // Get media items for archive
    let media_items = state
        .media
        .file_group_repository
        .get_group_media_for_archive(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                AppError::NotFound("File group not found".to_string())
            } else {
                AppError::from(e)
            }
        })?;

    if media_items.is_empty() {
        return Err(HttpAppError::from(AppError::BadRequest(
            "Cannot create archive for empty group".to_string(),
        )));
    }

    // Create archive
    let archive_data = create_archive(format, state.media.storage.clone(), media_items)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create archive");
            AppError::Internal(format!("Failed to create archive: {}", e))
        })?;

    // Determine content type and filename
    let (content_type, default_filename) = match format {
        ArchiveFormat::Zip => ("application/zip", "group.zip"),
        ArchiveFormat::Tar => ("application/x-tar", "group.tar"),
    };
    let filename = default_filename.to_string();

    // Build response
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(axum::body::Body::from(archive_data))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build archive response");
            AppError::Internal(format!("Failed to build response: {}", e))
        })?;

    Ok(response.into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v0/groups/{id}",
    tag = "file-groups",
    params(
        ("id" = Uuid, Path, description = "File group ID")
    ),
    responses(
        (status = 204, description = "File group deleted"),
        (status = 404, description = "File group not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        group_id = %id,
        operation = "delete_file_group"
    )
)]
pub async fn delete_file_group(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let deleted = state
        .media
        .file_group_repository
        .delete_group(tenant_ctx.tenant_id, id)
        .await
        .map_err(AppError::from)?;

    if !deleted {
        return Err(HttpAppError::from(AppError::NotFound(
            "File group not found".to_string(),
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}
