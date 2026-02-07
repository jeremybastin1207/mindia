use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use crate::state::AppState;
use mindia_core::models::{
    CreateFolderRequest, FolderResponse, FolderTreeNode, UpdateFolderRequest,
};
use mindia_core::AppError;

#[derive(Debug, Deserialize, ToSchema)]
pub struct FolderListQuery {
    #[serde(default)]
    pub parent_id: Option<Option<Uuid>>, // Option<Option> to distinguish between None (all folders) and Some(None) (root folders only)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveMediaRequest {
    pub folder_id: Option<Uuid>,
}

/// Create a new folder
#[utoipa::path(
    post,
    path = "/api/v0/folders",
    request_body = CreateFolderRequest,
    responses(
        (status = 201, description = "Folder created successfully", body = FolderResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Folder with same name already exists in parent"),
        (status = 500, description = "Internal server error")
    ),
    tag = "folders"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn create_folder(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Json(request): Json<CreateFolderRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Validate folder name
    let name = request.name.trim();
    if name.is_empty() {
        return Err(HttpAppError::from(AppError::BadRequest(
            "Folder name cannot be empty".to_string(),
        )));
    }

    if name.len() > 255 {
        return Err(HttpAppError::from(AppError::BadRequest(
            "Folder name cannot exceed 255 characters".to_string(),
        )));
    }

    // Create folder
    let folder = state
        .folder_repository
        .create_folder(ctx.tenant_id, name.to_string(), request.parent_id)
        .await
        .map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("not found") || err_msg.contains("Parent folder not found") {
                AppError::BadRequest(
                    "Parent folder not found or does not belong to tenant".to_string(),
                )
            } else if err_msg.contains("unique") || err_msg.contains("Duplicate") {
                AppError::BadRequest(
                    "A folder with this name already exists in the specified parent".to_string(),
                )
            } else {
                AppError::Internal(format!("Failed to create folder: {}", err_msg))
            }
        })?;

    let response = FolderResponse::from(folder);
    Ok(Json(response))
}

/// List folders
#[utoipa::path(
    get,
    path = "/api/v0/folders",
    params(
        ("parent_id" = Option<Option<Uuid>>, Query, description = "Filter by parent folder ID. Use null for root folders only, omit for all folders")
    ),
    responses(
        (status = 200, description = "List of folders", body = Vec<FolderResponse>),
        (status = 500, description = "Internal server error")
    ),
    tag = "folders"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn list_folders(
    ctx: TenantContext,
    Query(query): Query<FolderListQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let folders = state
        .folder_repository
        .list_folders(ctx.tenant_id, query.parent_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list folders: {}", e);
            AppError::Internal("Failed to list folders".to_string())
        })?;

    let responses: Vec<FolderResponse> = folders.into_iter().map(FolderResponse::from).collect();
    Ok(Json(responses))
}

/// Get folder tree (hierarchical structure)
#[utoipa::path(
    get,
    path = "/api/v0/folders/tree",
    responses(
        (status = 200, description = "Hierarchical folder tree", body = Vec<FolderTreeNode>),
        (status = 500, description = "Internal server error")
    ),
    tag = "folders"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn get_folder_tree(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
) -> Result<impl IntoResponse, HttpAppError> {
    let tree = state
        .folder_repository
        .get_folder_tree(ctx.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get folder tree: {}", e);
            AppError::Internal("Failed to get folder tree".to_string())
        })?;

    Ok(Json(tree))
}

/// Get folder by ID
#[utoipa::path(
    get,
    path = "/api/v0/folders/{id}",
    params(
        ("id" = Uuid, Path, description = "Folder ID")
    ),
    responses(
        (status = 200, description = "Folder details", body = FolderResponse),
        (status = 404, description = "Folder not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "folders"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn get_folder(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let folder = state
        .folder_repository
        .get_folder(ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get folder: {}", e);
            AppError::Internal("Failed to get folder".to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Folder not found".to_string()))?;

    // Get media count and subfolder count
    let media_count = state
        .folder_repository
        .count_media_in_folder(ctx.tenant_id, id)
        .await
        .ok();
    let subfolder_count = state
        .folder_repository
        .count_subfolders(ctx.tenant_id, id)
        .await
        .ok();

    let response = folder.to_response_with_counts(media_count, subfolder_count);
    Ok(Json(response))
}

/// Update folder
#[utoipa::path(
    put,
    path = "/api/v0/folders/{id}",
    params(
        ("id" = Uuid, Path, description = "Folder ID")
    ),
    request_body = UpdateFolderRequest,
    responses(
        (status = 200, description = "Folder updated successfully", body = FolderResponse),
        (status = 400, description = "Invalid request - duplicate name or cycle detected"),
        (status = 404, description = "Folder not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "folders"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn update_folder(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateFolderRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Validate name if provided
    let name = request.name.map(|n| {
        let trimmed = n.trim().to_string();
        if trimmed.is_empty() {
            Err(AppError::BadRequest(
                "Folder name cannot be empty".to_string(),
            ))
        } else if trimmed.len() > 255 {
            Err(AppError::BadRequest(
                "Folder name cannot exceed 255 characters".to_string(),
            ))
        } else {
            Ok(trimmed)
        }
    });

    let name = match name {
        Some(Ok(n)) => Some(n),
        Some(Err(e)) => return Err(HttpAppError::from(e)),
        None => None,
    };

    let folder = state
        .folder_repository
        .update_folder(ctx.tenant_id, id, name, request.parent_id)
        .await
        .map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("not found") || err_msg.contains("Folder not found") {
                AppError::NotFound("Folder not found or parent folder not found".to_string())
            } else if err_msg.contains("unique") || err_msg.contains("Duplicate") {
                AppError::BadRequest(
                    "A folder with this name already exists in the specified parent".to_string(),
                )
            } else if err_msg.contains("cycle") || err_msg.contains("CHECK") {
                AppError::BadRequest(
                    "Cannot move folder: would create a cycle in folder hierarchy".to_string(),
                )
            } else {
                AppError::Internal(format!("Failed to update folder: {}", err_msg))
            }
        })?;

    let response = FolderResponse::from(folder);
    Ok(Json(response))
}

/// Delete folder
#[utoipa::path(
    delete,
    path = "/api/v0/folders/{id}",
    params(
        ("id" = Uuid, Path, description = "Folder ID")
    ),
    responses(
        (status = 204, description = "Folder deleted successfully"),
        (status = 400, description = "Folder is not empty"),
        (status = 404, description = "Folder not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "folders"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn delete_folder(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let deleted = state
        .folder_repository
        .delete_folder(ctx.tenant_id, id)
        .await
        .map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("foreign key") || err_msg.contains("constraint") {
                AppError::BadRequest(
                    "Cannot delete folder: folder contains media items or subfolders".to_string(),
                )
            } else if err_msg.contains("not found") || err_msg.contains("RowNotFound") {
                AppError::NotFound("Folder not found".to_string())
            } else {
                tracing::error!("Failed to delete folder: {}", err_msg);
                AppError::Internal(format!("Failed to delete folder: {}", err_msg))
            }
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(HttpAppError::from(AppError::NotFound(
            "Folder not found".to_string(),
        )))
    }
}

/// Move media to a folder
#[utoipa::path(
    put,
    path = "/api/v0/media/{id}/folder",
    params(
        ("id" = Uuid, Path, description = "Media ID")
    ),
    request_body = MoveMediaRequest,
    responses(
        (status = 200, description = "Media moved successfully"),
        (status = 400, description = "Invalid request - folder not found"),
        (status = 404, description = "Media not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "folders"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn move_media(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(request): Json<MoveMediaRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    let moved = state
        .media
        .repository
        .move_media_to_folder(ctx.tenant_id, id, request.folder_id)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                AppError::BadRequest("Folder not found or does not belong to tenant".to_string())
            } else {
                tracing::error!("Failed to move media: {}", e);
                AppError::Internal("Failed to move media".to_string())
            }
        })?;

    if moved {
        Ok(StatusCode::OK)
    } else {
        Err(HttpAppError::from(AppError::NotFound(
            "Media not found".to_string(),
        )))
    }
}
