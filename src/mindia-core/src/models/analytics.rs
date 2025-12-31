use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RequestLog {
    pub id: i64,
    pub request_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub method: String,
    pub path: String,
    pub normalized_path: String,
    pub query_string: Option<String>,
    pub status_code: i32,
    pub request_size_bytes: i64,
    pub response_size_bytes: i64,
    pub duration_ms: i64,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLogInput {
    pub tenant_id: Option<Uuid>,
    pub method: String,
    pub path: String,
    pub normalized_path: String,
    pub query_string: Option<String>,
    pub status_code: i32,
    pub request_size_bytes: i64,
    pub response_size_bytes: i64,
    pub duration_ms: i64,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StorageMetrics {
    pub id: i32,
    pub tenant_id: Option<Uuid>,
    pub total_files: i64,
    pub total_storage_bytes: i64,
    pub image_count: i64,
    pub image_bytes: i64,
    pub video_count: i64,
    pub video_bytes: i64,
    pub audio_count: i64,
    pub audio_bytes: i64,
    pub document_count: i64,
    pub document_bytes: i64,
    pub by_content_type: JsonValue,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct UrlStatistics {
    pub path: String,
    pub request_count: i64,
    pub total_bytes_sent: i64,
    pub total_bytes_received: i64,
    pub avg_response_time_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub status_2xx: i64,
    pub status_3xx: i64,
    pub status_4xx: i64,
    pub status_5xx: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrafficSummary {
    pub total_requests: i64,
    pub total_bytes_sent: i64,
    pub total_bytes_received: i64,
    pub avg_response_time_ms: f64,
    pub requests_per_method: HashMap<String, i64>,
    pub requests_per_status: HashMap<i32, i64>,
    pub popular_urls: Vec<UrlStatistics>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StorageSummary {
    pub total_files: i64,
    pub total_storage_bytes: i64,
    pub image_count: i64,
    pub image_bytes: i64,
    pub video_count: i64,
    pub video_bytes: i64,
    pub audio_count: i64,
    pub audio_bytes: i64,
    pub by_content_type: HashMap<String, ContentTypeStats>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ContentTypeStats {
    pub count: i64,
    pub bytes: i64,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AnalyticsQuery {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AuditLogQuery {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub method: Option<String>,
    pub status_code: Option<i32>,
    pub path_filter: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogListResponse {
    pub logs: Vec<RequestLog>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
