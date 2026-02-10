use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;
use mindia_core::models::{TaskListQuery, TaskResponse, TaskStats};
use mindia_core::AppError;

/// List tasks with optional filters
#[tracing::instrument(skip(state))]
pub async fn list_tasks(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<serde_json::Value>, HttpAppError> {
    tracing::debug!("Listing tasks with filters: {:?}", query);

    let tasks = state
        .tasks
        .task_repository
        .list_tasks(tenant_ctx.tenant_id, query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list tasks");
            AppError::Internal(e.to_string())
        })?;

    let task_responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(Json(serde_json::json!({
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
) -> Result<Json<TaskResponse>, HttpAppError> {
    tracing::debug!(task_id = %task_id, "Getting task details");

    let task = state
        .tasks
        .task_repository
        .get_task(tenant_ctx.tenant_id, task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "Failed to get task");
            AppError::Internal(e.to_string())
        })?;

    match task {
        Some(task) => Ok(Json(TaskResponse::from(task))),
        None => {
            tracing::warn!(task_id = %task_id, "Task not found");
            Err(AppError::NotFound("Task not found".to_string()).into())
        }
    }
}

/// Cancel a pending or scheduled task
#[tracing::instrument(skip(state))]
pub async fn cancel_task(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, HttpAppError> {
    tracing::info!(task_id = %task_id, "Cancelling task");

    let task = state
        .tasks
        .task_repository
        .cancel_task(tenant_ctx.tenant_id, task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "Failed to cancel task");
            AppError::BadRequest(
                "Failed to cancel task - task not found or not in cancellable state".to_string(),
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
) -> Result<Json<TaskResponse>, HttpAppError> {
    tracing::info!(task_id = %task_id, "Manually retrying task");

    let task = state
        .tasks
        .task_repository
        .retry_task(tenant_ctx.tenant_id, task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "Failed to retry task");
            AppError::BadRequest(
                "Failed to retry task - task not found or not in failed state".to_string(),
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
) -> Result<Json<TaskStats>, HttpAppError> {
    tracing::debug!("Getting task statistics");

    let stats = state
        .tasks
        .task_repository
        .get_stats(tenant_ctx.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get task stats");
            AppError::Internal(e.to_string())
        })?;

    Ok(Json(stats))
}
