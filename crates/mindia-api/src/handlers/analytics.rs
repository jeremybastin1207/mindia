use crate::auth::models::TenantContext;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
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
) -> impl IntoResponse {
    match state.analytics.get_traffic_summary(query).await {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get traffic summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve traffic summary"
                })),
            )
                .into_response()
        }
    }
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
) -> impl IntoResponse {
    match state.analytics.get_url_statistics(query).await {
        Ok(stats) => (StatusCode::OK, Json(stats)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get URL statistics");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve URL statistics"
                })),
            )
                .into_response()
        }
    }
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
pub async fn get_storage_summary(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.analytics.get_storage_summary().await {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get storage summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve storage summary"
                })),
            )
                .into_response()
        }
    }
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
pub async fn refresh_storage_metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.analytics.refresh_storage_metrics().await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Storage metrics refreshed successfully"
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to refresh storage metrics");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to refresh storage metrics"
                })),
            )
                .into_response()
        }
    }
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
) -> impl IntoResponse {
    match state
        .analytics
        .list_audit_logs(Some(tenant_ctx.tenant_id), query)
        .await
    {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to list audit logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve audit logs"
                })),
            )
                .into_response()
        }
    }
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
) -> impl IntoResponse {
    match state
        .analytics
        .get_audit_log(id, Some(tenant_ctx.tenant_id))
        .await
    {
        Ok(Some(log)) => (StatusCode::OK, Json(log)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Audit log not found"
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get audit log");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve audit log"
                })),
            )
                .into_response()
        }
    }
}
