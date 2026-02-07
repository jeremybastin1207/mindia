//! Named transformation (preset) management handlers
//!
//! CRUD operations for named transformations. Presets store reusable transformation
//! chains that can be referenced in image URLs using `-/preset/{name}/` syntax.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use std::sync::Arc;

use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use crate::state::AppState;
use mindia_core::models::{
    CreateNamedTransformationRequest, NamedTransformationResponse, UpdateNamedTransformationRequest,
};
use mindia_core::transform_url::ImageTransformUrlParser;
use mindia_core::AppError;

/// Create a new named transformation (preset)
#[utoipa::path(
    post,
    path = "/api/v0/presets",
    request_body = CreateNamedTransformationRequest,
    responses(
        (status = 201, description = "Preset created successfully", body = NamedTransformationResponse),
        (status = 400, description = "Invalid request - validation failed"),
        (status = 409, description = "Preset with this name already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "presets"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn create_preset(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Json(request): Json<CreateNamedTransformationRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Validate preset name
    let name = request.name.trim().to_string();
    CreateNamedTransformationRequest::validate_name(&name)
        .map_err(|e| HttpAppError::from(AppError::BadRequest(e)))?;

    // Validate that operations don't reference other presets (no recursion)
    CreateNamedTransformationRequest::validate_no_preset_reference(&request.operations)
        .map_err(|e| HttpAppError::from(AppError::BadRequest(e)))?;

    // Validate the operations string is valid transformation syntax
    // We parse it with a dummy image ID just to validate syntax
    let ops_trimmed = request.operations.trim();
    if !ops_trimmed.is_empty() {
        ImageTransformUrlParser::parse_operations(ops_trimmed, "validation".to_string()).map_err(
            |e| HttpAppError::from(AppError::BadRequest(format!("Invalid operations: {}", e))),
        )?;
    }

    // Create the preset
    let preset = state.db
        .named_transformation_repository
        .create(
            ctx.tenant_id,
            name,
            request.operations.trim().to_string(),
            request.description,
        )
        .await
        .map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("already exists") {
                AppError::BadRequest("A preset with this name already exists".to_string())
            } else {
                tracing::error!(error = %e, "Failed to create preset");
                AppError::Internal(format!("Failed to create preset: {}", err_msg))
            }
        })?;

    let response = NamedTransformationResponse::from(preset);
    Ok((StatusCode::CREATED, Json(response)))
}

/// List all presets for the tenant
#[utoipa::path(
    get,
    path = "/api/v0/presets",
    responses(
        (status = 200, description = "List of presets", body = Vec<NamedTransformationResponse>),
        (status = 500, description = "Internal server error")
    ),
    tag = "presets"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn list_presets(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
) -> Result<impl IntoResponse, HttpAppError> {
    let presets = state.db
        .named_transformation_repository
        .list(ctx.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list presets");
            HttpAppError::from(AppError::Internal("Failed to list presets".to_string()))
        })?;

    let responses: Vec<NamedTransformationResponse> = presets
        .into_iter()
        .map(NamedTransformationResponse::from)
        .collect();
    Ok(Json(responses))
}

/// Get a preset by name
#[utoipa::path(
    get,
    path = "/api/v0/presets/{name}",
    params(
        ("name" = String, Path, description = "Preset name")
    ),
    responses(
        (status = 200, description = "Preset details", body = NamedTransformationResponse),
        (status = 404, description = "Preset not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "presets"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn get_preset(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, HttpAppError> {
    let preset = state.db
        .named_transformation_repository
        .get_by_name(ctx.tenant_id, &name)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get preset");
            HttpAppError::from(AppError::Internal("Failed to get preset".to_string()))
        })?
        .ok_or_else(|| AppError::NotFound("Preset not found".to_string()))?;

    Ok(Json(NamedTransformationResponse::from(preset)))
}

/// Update a preset
#[utoipa::path(
    put,
    path = "/api/v0/presets/{name}",
    params(
        ("name" = String, Path, description = "Preset name")
    ),
    request_body = UpdateNamedTransformationRequest,
    responses(
        (status = 200, description = "Preset updated successfully", body = NamedTransformationResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Preset not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "presets"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn update_preset(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(name): Path<String>,
    Json(request): Json<UpdateNamedTransformationRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Validate operations if provided
    if let Some(ref operations) = request.operations {
        // Validate no preset references
        CreateNamedTransformationRequest::validate_no_preset_reference(operations)
            .map_err(|e| HttpAppError::from(AppError::BadRequest(e)))?;

        // Validate syntax
        let ops_trimmed = operations.trim();
        if !ops_trimmed.is_empty() {
            ImageTransformUrlParser::parse_operations(ops_trimmed, "validation".to_string())
                .map_err(|e| {
                    HttpAppError::from(AppError::BadRequest(format!("Invalid operations: {}", e)))
                })?;
        }
    }

    let preset = state.db
        .named_transformation_repository
        .update(
            ctx.tenant_id,
            &name,
            request.operations.map(|s| s.trim().to_string()),
            request.description,
        )
        .await
        .map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("not found") {
                AppError::NotFound("Preset not found".to_string())
            } else {
                tracing::error!(error = %e, "Failed to update preset");
                AppError::Internal(format!("Failed to update preset: {}", err_msg))
            }
        })?;

    Ok(Json(NamedTransformationResponse::from(preset)))
}

/// Delete a preset
#[utoipa::path(
    delete,
    path = "/api/v0/presets/{name}",
    params(
        ("name" = String, Path, description = "Preset name")
    ),
    responses(
        (status = 204, description = "Preset deleted successfully"),
        (status = 404, description = "Preset not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "presets"
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn delete_preset(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, HttpAppError> {
    let deleted = state.db
        .named_transformation_repository
        .delete(ctx.tenant_id, &name)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete preset");
            HttpAppError::from(AppError::Internal("Failed to delete preset".to_string()))
        })?;

    if !deleted {
        return Err(HttpAppError::from(AppError::NotFound(
            "Preset not found".to_string(),
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}
