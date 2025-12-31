use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Usage event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "usage_event_type", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum UsageEventType {
    StorageUpload,
    StorageDelete,
    ApiRequest,
    UploadCreated,
    WebhookCreated,
    WebhookDeleted,
    ApiKeyCreated,
    ApiKeyDeleted,
    MemberAdded,
    MemberRemoved,
    PeriodReset,
}

/// Usage tracking for an organization
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct UsageTracking {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub subscription_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub storage_bytes_used: i64,
    pub storage_bytes_limit: i64,
    pub api_requests_count: i32,
    pub api_requests_limit: i32,
    pub uploads_count: i32,
    pub uploads_limit: i32,
    pub webhooks_count: i32,
    pub webhooks_limit: i32,
    pub api_keys_count: i32,
    pub api_keys_limit: i32,
    pub organization_members_count: i32,
    pub organization_members_limit: i32,
    pub last_updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Usage event (audit log)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct UsageEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub event_type: UsageEventType,
    pub metadata: serde_json::Value,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
}

/// Alert level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "alert_level", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Warning,
    Critical,
    Exceeded,
}

/// Alert type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "alert_type", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum AlertType {
    Storage,
    ApiRequests,
    Uploads,
    Other,
}

/// Usage alert
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct UsageAlert {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub alert_type: AlertType,
    pub alert_level: AlertLevel,
    pub current_usage: i64,
    pub limit_value: i64,
    #[schema(value_type = f64, example = 75.5)]
    pub usage_percent: rust_decimal::Decimal, // 0-100 percentage
    pub message: Option<String>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
