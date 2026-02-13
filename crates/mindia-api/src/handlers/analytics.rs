use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
};
use mindia_core::models::{
    AnalyticsQuery, AuditLogListResponse, AuditLogQuery, RequestLog, StorageSummary,
    TrafficSummary, UrlStatistics,
};
use std::sync::Arc;

#[utoipa::path(
    get,
    path = "/api/v0/analytics/traffic",
    tag = "analytics",
    params(
        AnalyticsQuery
    ),
    responses(
        (status = 200, description = "Traffic statistics", body = TrafficSummary),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_traffic_summary(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    let summary = state
        .db
        .analytics
        .get_traffic_summary(query)
        .await
        .map_err(HttpAppError::from)?;
    Ok(Json(summary))
}

#[utoipa::path(
    get,
    path = "/api/v0/analytics/urls",
    tag = "analytics",
    params(
        AnalyticsQuery
    ),
    responses(
        (status = 200, description = "URL statistics", body = Vec<UrlStatistics>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_url_statistics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    let stats = state
        .db
        .analytics
        .get_url_statistics(query)
        .await
        .map_err(HttpAppError::from)?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v0/analytics/storage",
    tag = "analytics",
    responses(
        (status = 200, description = "Storage metrics", body = StorageSummary),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_storage_summary(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let summary = state
        .db
        .analytics
        .get_storage_summary()
        .await
        .map_err(HttpAppError::from)?;
    Ok(Json(summary))
}

#[utoipa::path(
    post,
    path = "/api/v0/analytics/storage/refresh",
    tag = "analytics",
    responses(
        (status = 200, description = "Storage metrics refreshed successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn refresh_storage_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    state
        .db
        .analytics
        .refresh_storage_metrics()
        .await
        .map_err(HttpAppError::from)?;
    Ok(Json(serde_json::json!({
        "message": "Storage metrics refreshed successfully"
    })))
}

#[utoipa::path(
    get,
    path = "/api/v0/audit-logs",
    tag = "analytics",
    params(
        AuditLogQuery
    ),
    responses(
        (status = 200, description = "List of audit logs with pagination", body = AuditLogListResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_token" = []))
)]
pub async fn list_audit_logs(
    tenant_ctx: TenantContext,
    Query(query): Query<AuditLogQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let response = state
        .db
        .analytics
        .list_audit_logs(Some(tenant_ctx.tenant_id), query)
        .await
        .map_err(HttpAppError::from)?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v0/audit-logs/{id}",
    tag = "analytics",
    params(
        ("id" = i64, Path, description = "Audit log ID")
    ),
    responses(
        (status = 200, description = "Audit log details", body = RequestLog),
        (status = 404, description = "Audit log not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_token" = []))
)]
pub async fn get_audit_log(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, HttpAppError> {
    let log = state
        .db
        .analytics
        .get_audit_log(id, Some(tenant_ctx.tenant_id))
        .await
        .map_err(HttpAppError::from)?;
    match log {
        Some(l) => Ok(Json(l)),
        None => Err(mindia_core::AppError::NotFound("Audit log not found".to_string()).into()),
    }
}
