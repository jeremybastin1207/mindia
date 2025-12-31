use crate::auth::models::TenantContext;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;
use mindia_core::models::{TaskListQuery, TaskResponse, TaskStats};

/// List tasks with optional filters
#[tracing::instrument(skip(state))]
pub async fn list_tasks(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    tracing::debug!("Listing tasks with filters: {:?}", query);

    let tasks = state
        .task_repository
        .list_tasks(tenant_ctx.tenant_id, query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list tasks");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to list tasks"
                })),
            )
        })?;

    let task_responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(Json(json!({
        "tasks": task_responses,
        "count": task_responses.len()
    })))
}

/// Get a task by ID
#[tracing::instrument(skip(state))]
pub async fn get_task(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<serde_json::Value>)> {
    tracing::debug!(task_id = %task_id, "Getting task details");

    let task = state
        .task_repository
        .get_task(tenant_ctx.tenant_id, task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "Failed to get task");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get task"
                })),
            )
        })?;

    match task {
        Some(task) => Ok(Json(TaskResponse::from(task))),
        None => {
            tracing::warn!(task_id = %task_id, "Task not found");
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Task not found"
                })),
            ))
        }
    }
}

/// Cancel a pending or scheduled task
#[tracing::instrument(skip(state))]
pub async fn cancel_task(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<serde_json::Value>)> {
    tracing::info!(task_id = %task_id, "Cancelling task");

    let task = state
        .task_repository
        .cancel_task(tenant_ctx.tenant_id, task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "Failed to cancel task");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Failed to cancel task - task not found or not in cancellable state"
                })),
            )
        })?;

    tracing::info!(task_id = %task_id, "Task cancelled successfully");

    Ok(Json(TaskResponse::from(task)))
}

/// Retry a failed task
#[tracing::instrument(skip(state))]
pub async fn retry_task(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<serde_json::Value>)> {
    tracing::info!(task_id = %task_id, "Manually retrying task");

    let task = state
        .task_repository
        .retry_task(tenant_ctx.tenant_id, task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "Failed to retry task");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Failed to retry task - task not found or not in failed state"
                })),
            )
        })?;

    tracing::info!(task_id = %task_id, "Task retry scheduled successfully");

    Ok(Json(TaskResponse::from(task)))
}

/// Get aggregated task statistics
#[tracing::instrument(skip(state))]
pub async fn get_task_stats(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
) -> Result<Json<TaskStats>, (StatusCode, Json<serde_json::Value>)> {
    tracing::debug!("Getting task statistics");

    let stats = state
        .task_repository
        .get_stats(tenant_ctx.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get task stats");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get task statistics"
                })),
            )
        })?;

    Ok(Json(stats))
}
