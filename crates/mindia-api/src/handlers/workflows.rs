//! Workflow API handlers

use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use mindia_core::models::{Workflow, WorkflowExecution};
use mindia_core::AppError;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub steps: serde_json::Value,
    #[serde(default = "default_true")]
    pub trigger_on_upload: bool,
    #[serde(default = "default_true")]
    pub stop_on_failure: bool,
    pub media_types: Option<Vec<String>>,
    pub folder_ids: Option<Vec<Uuid>>,
    pub content_types: Option<Vec<String>>,
    pub metadata_filter: Option<serde_json::Value>,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub steps: Option<serde_json::Value>,
    pub trigger_on_upload: Option<bool>,
    pub stop_on_failure: Option<bool>,
    pub media_types: Option<Option<Vec<String>>>,
    pub folder_ids: Option<Option<Vec<Uuid>>>,
    pub content_types: Option<Option<Vec<String>>>,
    pub metadata_filter: Option<Option<serde_json::Value>>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct WorkflowResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub steps: serde_json::Value,
    pub trigger_on_upload: bool,
    pub stop_on_failure: bool,
    pub media_types: Option<Vec<String>>,
    pub folder_ids: Option<Vec<Uuid>>,
    pub content_types: Option<Vec<String>>,
    pub metadata_filter: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct WorkflowExecutionResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub media_id: Uuid,
    pub status: String,
    pub task_ids: Vec<Uuid>,
    pub current_step: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListWorkflowsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListExecutionsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn workflow_to_response(w: &Workflow) -> WorkflowResponse {
    WorkflowResponse {
        id: w.id,
        name: w.name.clone(),
        description: w.description.clone(),
        enabled: w.enabled,
        steps: w.steps.clone(),
        trigger_on_upload: w.trigger_on_upload,
        stop_on_failure: w.stop_on_failure,
        media_types: w.media_types.clone(),
        folder_ids: w.folder_ids.clone(),
        content_types: w.content_types.clone(),
        metadata_filter: w.metadata_filter.clone(),
        created_at: w.created_at,
        updated_at: w.updated_at,
    }
}

fn execution_to_response(e: &WorkflowExecution) -> WorkflowExecutionResponse {
    WorkflowExecutionResponse {
        id: e.id,
        workflow_id: e.workflow_id,
        media_id: e.media_id,
        status: format!("{:?}", e.status).to_lowercase(),
        task_ids: e.task_ids.clone(),
        current_step: e.current_step,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}

#[utoipa::path(
    post,
    path = "/api/v0/workflows",
    tag = "workflows",
    request_body = CreateWorkflowRequest,
    responses(
        (status = 200, description = "Workflow created", body = WorkflowResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
    )
)]
pub async fn create_workflow(
    tenant_context: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;
    let w = state.workflows
        .workflow_repository
        .create(
            tenant_id,
            &req.name,
            req.description.as_deref(),
            req.enabled,
            req.steps,
            req.trigger_on_upload,
            req.stop_on_failure,
            req.media_types.as_deref(),
            req.folder_ids.as_deref(),
            req.content_types.as_deref(),
            req.metadata_filter,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create workflow: {}", e)))?;
    Ok(Json(workflow_to_response(&w)))
}

#[utoipa::path(
    get,
    path = "/api/v0/workflows",
    tag = "workflows",
    params(ListWorkflowsQuery),
    responses(
        (status = 200, description = "List of workflows", body = Vec<WorkflowResponse>),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
    )
)]
pub async fn list_workflows(
    tenant_context: TenantContext,
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListWorkflowsQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;
    let limit = q.limit.unwrap_or(50).min(500);
    let offset = q.offset.unwrap_or(0);
    let list = state.workflows
        .workflow_repository
        .list(tenant_id, limit, offset)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to list workflows: {}", e)))?;
    Ok(Json(
        list.iter().map(workflow_to_response).collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v0/workflows/{id}",
    tag = "workflows",
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "Workflow", body = WorkflowResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn get_workflow(
    tenant_context: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;
    let w = state.workflows
        .workflow_repository
        .get(tenant_id, id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get workflow: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Workflow not found".into()))?;
    Ok(Json(workflow_to_response(&w)))
}

#[utoipa::path(
    put,
    path = "/api/v0/workflows/{id}",
    tag = "workflows",
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = UpdateWorkflowRequest,
    responses(
        (status = 200, description = "Workflow updated", body = WorkflowResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn update_workflow(
    tenant_context: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateWorkflowRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;
    let w = state.workflows
        .workflow_repository
        .update(
            tenant_id,
            id,
            req.name.as_deref(),
            req.description.as_deref(),
            req.enabled,
            req.steps,
            req.trigger_on_upload,
            req.stop_on_failure,
            req.media_types,
            req.folder_ids,
            req.content_types,
            req.metadata_filter,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update workflow: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Workflow not found".into()))?;
    Ok(Json(workflow_to_response(&w)))
}

#[utoipa::path(
    delete,
    path = "/api/v0/workflows/{id}",
    tag = "workflows",
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 204, description = "Workflow deleted"),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn delete_workflow(
    tenant_context: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;
    let deleted = state.workflows
        .workflow_repository
        .delete(tenant_id, id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete workflow: {}", e)))?;
    if !deleted {
        return Err(AppError::NotFound("Workflow not found".into()).into());
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v0/workflows/{id}/trigger/{media_id}",
    tag = "workflows",
    params(("id" = Uuid, Path, description = "Workflow ID"), ("media_id" = Uuid, Path, description = "Media ID")),
    responses(
        (status = 200, description = "Workflow triggered", body = WorkflowExecutionResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Workflow or media not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn trigger_workflow(
    tenant_context: TenantContext,
    Path((id, media_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;
    let exec = state.workflows
        .workflow_service
        .trigger_workflow(tenant_id, id, media_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to trigger workflow: {}", e)))?;
    Ok(Json(execution_to_response(&exec)))
}

#[utoipa::path(
    get,
    path = "/api/v0/workflows/{id}/executions",
    tag = "workflows",
    params(("id" = Uuid, Path), ListExecutionsQuery),
    responses(
        (status = 200, description = "List of workflow executions", body = Vec<WorkflowExecutionResponse>),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
    )
)]
pub async fn list_workflow_executions(
    tenant_context: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListExecutionsQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;
    let limit = q.limit.unwrap_or(50).min(500);
    let offset = q.offset.unwrap_or(0);
    let list = state.workflows
        .workflow_execution_repository
        .list_by_workflow(tenant_id, id, limit, offset)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to list executions: {}", e)))?;
    Ok(Json(
        list.iter().map(execution_to_response).collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v0/workflow-executions/{id}",
    tag = "workflows",
    params(("id" = Uuid, Path, description = "Execution ID")),
    responses(
        (status = 200, description = "Workflow execution", body = WorkflowExecutionResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    )
)]
pub async fn get_workflow_execution(
    tenant_context: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let tenant_id = tenant_context.tenant_id;
    let e = state.workflows
        .workflow_execution_repository
        .get_by_tenant_and_id(tenant_id, id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get execution: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Workflow execution not found".into()))?;
    Ok(Json(execution_to_response(&e)))
}
