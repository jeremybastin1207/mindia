//! Plugin API handlers

use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError, ValidatedJson};
use crate::state::AppState;
use anyhow::Error as AnyhowError;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use mindia_core::models::{
    ExecutePluginRequest, ExecutePluginResponse, PluginConfigResponse, PluginCostSummaryResponse,
    PluginCostsResponse, PluginInfoResponse, UpdatePluginConfigRequest,
};
use mindia_core::AppError;
use serde::Deserialize;
use std::sync::Arc;

#[utoipa::path(
    get,
    path = "/api/v0/plugins",
    tag = "plugins",
    responses(
        (status = 200, description = "List of available plugins", body = Vec<PluginInfoResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(state), fields(operation = "list_plugins"))]
pub async fn list_plugins(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let plugins = state
        .plugins
        .plugin_service
        .list_plugins()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to list plugins: {}", e)))?;

    let response: Vec<PluginInfoResponse> = plugins
        .into_iter()
        .map(|info| PluginInfoResponse {
            name: info.name,
            description: info.description,
            supported_media_types: info.supported_media_types,
        })
        .collect();

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/v0/plugins/{plugin_name}/execute",
    tag = "plugins",
    params(
        ("plugin_name" = String, Path, description = "Plugin name (e.g., 'assembly_ai')")
    ),
    request_body = ExecutePluginRequest,
    responses(
        (status = 200, description = "Plugin execution started", body = ExecutePluginResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Plugin not found", body = ErrorResponse),
    )
)]
#[tracing::instrument(
    skip(state, request),
    fields(
        tenant_id = %tenant_context.tenant_id,
        user_id = ?tenant_context.user_id,
        plugin_name = %plugin_name,
        media_id = ?request.media_id,
        operation = "execute_plugin"
    )
)]
pub async fn execute_plugin(
    tenant_context: TenantContext,
    Path(plugin_name): Path<String>,
    State(state): State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<ExecutePluginRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;

    let task_id = state
        .plugins
        .plugin_service
        .execute_plugin(tenant_id, &plugin_name, request.media_id)
        .await
        .map_err(|e| HttpAppError::from(plugin_execution_error_to_app_error(e)))?;

    Ok(Json(ExecutePluginResponse {
        task_id,
        status: "pending".to_string(),
    }))
}

/// Map plugin execution errors to appropriate HTTP status codes (404, 400, 500)
fn plugin_execution_error_to_app_error(e: AnyhowError) -> AppError {
    let msg = e.to_string();
    if msg.contains("Plugin not found") {
        AppError::NotFound(msg)
    } else if msg.contains("Plugin not configured for tenant")
        || msg.contains("Plugin is not enabled")
    {
        AppError::BadRequest(msg)
    } else {
        AppError::Internal(format!("Failed to execute plugin: {}", e))
    }
}

#[utoipa::path(
    get,
    path = "/api/v0/plugins/{plugin_name}/config",
    tag = "plugins",
    params(
        ("plugin_name" = String, Path, description = "Plugin name (e.g., 'assembly_ai')")
    ),
    responses(
        (status = 200, description = "Plugin configuration", body = PluginConfigResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Plugin configuration not found", body = ErrorResponse),
    )
)]
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %tenant_context.tenant_id,
        user_id = ?tenant_context.user_id,
        plugin_name = %plugin_name,
        operation = "get_plugin_config"
    )
)]
pub async fn get_plugin_config(
    State(state): State<Arc<AppState>>,
    tenant_context: TenantContext,
    Path(plugin_name): Path<String>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;

    let config = state
        .plugins
        .plugin_service
        .get_plugin_config(tenant_id, &plugin_name)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get plugin config: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Plugin configuration not found".to_string()))?;

    Ok(Json(PluginConfigResponse::from(config)))
}

#[utoipa::path(
    put,
    path = "/api/v0/plugins/{plugin_name}/config",
    tag = "plugins",
    params(
        ("plugin_name" = String, Path, description = "Plugin name (e.g., 'assembly_ai')")
    ),
    request_body = UpdatePluginConfigRequest,
    responses(
        (status = 200, description = "Plugin configuration updated", body = PluginConfigResponse),
        (status = 400, description = "Invalid configuration", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Plugin not found", body = ErrorResponse),
    )
)]
#[tracing::instrument(
    skip(state, request),
    fields(
        tenant_id = %tenant_context.tenant_id,
        user_id = ?tenant_context.user_id,
        plugin_name = %plugin_name,
        operation = "update_plugin_config"
    )
)]
pub async fn update_plugin_config(
    State(state): State<Arc<AppState>>,
    tenant_context: TenantContext,
    Path(plugin_name): Path<String>,
    Json(request): Json<UpdatePluginConfigRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;

    let config = state
        .plugins
        .plugin_service
        .update_plugin_config(tenant_id, &plugin_name, request.enabled, request.config)
        .await
        .map_err(|e| HttpAppError::from(plugin_config_error_to_app_error(e)))?;

    Ok(Json(PluginConfigResponse::from(config)))
}

/// Map plugin config update errors to appropriate HTTP status codes (404, 400, 500)
fn plugin_config_error_to_app_error(e: AnyhowError) -> AppError {
    let msg = e.to_string();
    let msg_lower = msg.to_lowercase();
    // Registry returns "Plugin 'x' not found"; service uses "Plugin not found"
    if msg_lower.contains("plugin") && msg_lower.contains("not found") {
        AppError::NotFound(msg)
    } else if msg_lower.contains("invalid")
        || msg_lower.contains("validation")
        || msg_lower.contains("configuration")
    {
        AppError::BadRequest(msg)
    } else {
        AppError::Internal(format!("Failed to update plugin config: {}", e))
    }
}

/// Query parameters for plugin costs endpoints
#[derive(Debug, Deserialize)]
pub struct PluginCostsQuery {
    /// Filter by plugin name
    pub plugin_name: Option<String>,
    /// Start of date range (ISO 8601)
    pub period_start: Option<DateTime<Utc>>,
    /// End of date range (ISO 8601)
    pub period_end: Option<DateTime<Utc>>,
}

#[utoipa::path(
    get,
    path = "/api/v0/plugins/costs",
    tag = "plugins",
    params(
        ("plugin_name" = Option<String>, Query, description = "Filter by plugin name"),
        ("period_start" = Option<String>, Query, description = "Start of date range (ISO 8601)"),
        ("period_end" = Option<String>, Query, description = "End of date range (ISO 8601)"),
    ),
    responses(
        (status = 200, description = "Plugin usage/cost summary", body = PluginCostsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    )
)]
#[tracing::instrument(
    skip(state, params),
    fields(
        tenant_id = %tenant_context.tenant_id,
        user_id = ?tenant_context.user_id,
        plugin_name = ?params.plugin_name,
        operation = "get_plugin_costs"
    )
)]
pub async fn get_plugin_costs(
    tenant_context: TenantContext,
    State(state): State<Arc<AppState>>,
    Query(params): Query<PluginCostsQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;

    // When both omitted, default to last 30 days so response period reflects the data
    let now = Utc::now();
    let (query_start, query_end) = match (params.period_start, params.period_end) {
        (Some(s), Some(e)) => (s, e),
        (Some(s), None) => (s, now),
        (None, Some(e)) => (e - chrono::Duration::days(30), e),
        (None, None) => (now - chrono::Duration::days(30), now),
    };

    let summary = state
        .plugins
        .plugin_execution_repository
        .get_usage_summary(
            tenant_id,
            params.plugin_name.as_deref(),
            Some(query_start),
            Some(query_end),
        )
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get plugin costs: {}", e)))?;

    let period_start = query_start;
    let period_end = query_end;

    let costs: Vec<PluginCostSummaryResponse> = summary
        .into_iter()
        .map(
            |(plugin_name, execution_count, total_units, unit_type)| PluginCostSummaryResponse {
                plugin_name,
                execution_count,
                total_units,
                unit_type,
                period_start,
                period_end,
            },
        )
        .collect();

    let total_executions = costs.iter().map(|c| c.execution_count).sum();

    Ok(Json(PluginCostsResponse {
        costs,
        total_executions,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v0/plugins/costs/summary",
    tag = "plugins",
    params(
        ("plugin_name" = Option<String>, Query, description = "Filter by plugin name"),
        ("period_start" = Option<String>, Query, description = "Start of date range (ISO 8601)"),
        ("period_end" = Option<String>, Query, description = "End of date range (ISO 8601)"),
    ),
    responses(
        (status = 200, description = "Aggregated plugin usage summary", body = PluginCostsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    )
)]
#[tracing::instrument(
    skip(state, params),
    fields(
        tenant_id = %tenant_context.tenant_id,
        user_id = ?tenant_context.user_id,
        operation = "get_plugin_costs_summary"
    )
)]
pub async fn get_plugin_costs_summary(
    tenant_context: TenantContext,
    State(state): State<Arc<AppState>>,
    Query(params): Query<PluginCostsQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Same as get_plugin_costs - both return aggregated summary
    get_plugin_costs(tenant_context, State(state), Query(params)).await
}

#[utoipa::path(
    get,
    path = "/api/v0/plugins/{plugin_name}/costs",
    tag = "plugins",
    params(
        ("plugin_name" = String, Path, description = "Plugin name (e.g., 'assembly_ai')"),
        ("period_start" = Option<String>, Query, description = "Start of date range (ISO 8601)"),
        ("period_end" = Option<String>, Query, description = "End of date range (ISO 8601)"),
    ),
    responses(
        (status = 200, description = "Plugin usage/cost for specific plugin", body = PluginCostsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    )
)]
#[tracing::instrument(
    skip(state, params),
    fields(
        tenant_id = %tenant_context.tenant_id,
        user_id = ?tenant_context.user_id,
        plugin_name = %plugin_name,
        operation = "get_plugin_costs_by_name"
    )
)]
pub async fn get_plugin_costs_by_name(
    tenant_context: TenantContext,
    Path(plugin_name): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<PluginCostsQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    let query_params = PluginCostsQuery {
        plugin_name: Some(plugin_name),
        period_start: params.period_start,
        period_end: params.period_end,
    };
    get_plugin_costs(tenant_context, State(state), Query(query_params)).await
}
