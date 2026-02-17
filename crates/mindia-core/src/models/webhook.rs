use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[cfg(feature = "sqlx")]
use sqlx::FromRow;

/// Webhook event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "webhook_event_type", rename_all = "lowercase")
)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    FileUploaded,
    FileDeleted,
    FileStored,
    FileProcessingCompleted,
    FileProcessingFailed,
    WorkflowCompleted,
    WorkflowFailed,
}

impl Display for WebhookEventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            WebhookEventType::FileUploaded => write!(f, "file.uploaded"),
            WebhookEventType::FileDeleted => write!(f, "file.deleted"),
            WebhookEventType::FileStored => write!(f, "file.stored"),
            WebhookEventType::FileProcessingCompleted => write!(f, "file.processing_completed"),
            WebhookEventType::FileProcessingFailed => write!(f, "file.processing_failed"),
            WebhookEventType::WorkflowCompleted => write!(f, "workflow.completed"),
            WebhookEventType::WorkflowFailed => write!(f, "workflow.failed"),
        }
    }
}

impl FromStr for WebhookEventType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file.uploaded" => Ok(WebhookEventType::FileUploaded),
            "file.deleted" => Ok(WebhookEventType::FileDeleted),
            "file.stored" => Ok(WebhookEventType::FileStored),
            "file.processing_completed" => Ok(WebhookEventType::FileProcessingCompleted),
            "file.processing_failed" => Ok(WebhookEventType::FileProcessingFailed),
            "workflow.completed" => Ok(WebhookEventType::WorkflowCompleted),
            "workflow.failed" => Ok(WebhookEventType::WorkflowFailed),
            _ => Err(anyhow::anyhow!("Invalid webhook event type: {}", s)),
        }
    }
}

/// Webhook delivery status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "webhook_delivery_status", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum WebhookDeliveryStatus {
    Pending,
    Success,
    Failed,
    Retrying,
}

impl Display for WebhookDeliveryStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            WebhookDeliveryStatus::Pending => write!(f, "pending"),
            WebhookDeliveryStatus::Success => write!(f, "success"),
            WebhookDeliveryStatus::Failed => write!(f, "failed"),
            WebhookDeliveryStatus::Retrying => write!(f, "retrying"),
        }
    }
}

/// Webhook configuration entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct Webhook {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub url: String,
    pub event_type: WebhookEventType,
    pub signing_secret: Option<String>,
    pub is_active: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
    pub deactivation_reason: Option<String>,
}

/// Webhook event log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct WebhookEventLog {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: WebhookEventType,
    pub payload: JsonValue,
    pub status: WebhookDeliveryStatus,
    pub response_status_code: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Webhook retry queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct WebhookRetryQueueItem {
    pub id: Uuid,
    pub webhook_event_id: Uuid,
    pub webhook_id: Uuid,
    pub tenant_id: Uuid,
    pub retry_count: i32,
    pub max_retries: i32,
    pub next_retry_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Webhook payload structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookPayload {
    pub hook: WebhookHookInfo,
    pub data: WebhookDataInfo,
    pub initiator: WebhookInitiatorInfo,
}

/// Hook information in webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookHookInfo {
    pub id: Uuid,
    pub event: String,
    pub target: String,
    pub project: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Data information in webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookDataInfo {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploaded_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stored_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Initiator information in webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookInitiatorInfo {
    #[serde(rename = "type")]
    pub initiator_type: String,
    pub id: Uuid,
}

/// Request models for API endpoints
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateWebhookRequest {
    #[validate(url(message = "Invalid webhook URL"))]
    #[validate(length(max = 2048, message = "URL must be at most 2048 characters"))]
    pub url: String,
    pub event_type: WebhookEventType,
    #[validate(length(
        min = 16,
        max = 256,
        message = "Signing secret must be between 16 and 256 characters"
    ))]
    pub signing_secret: Option<String>,
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateWebhookRequest {
    #[validate(url(message = "Invalid webhook URL"))]
    #[validate(length(max = 2048, message = "URL must be at most 2048 characters"))]
    pub url: Option<String>,
    #[validate(length(
        min = 16,
        max = 256,
        message = "Signing secret must be between 16 and 256 characters"
    ))]
    pub signing_secret: Option<String>,
    pub is_active: Option<bool>,
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
}

/// Response models for API endpoints
#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookResponse {
    pub id: Uuid,
    pub url: String,
    pub event_type: WebhookEventType,
    pub has_signing_secret: bool,
    pub is_active: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
    pub deactivation_reason: Option<String>,
}

impl From<Webhook> for WebhookResponse {
    fn from(webhook: Webhook) -> Self {
        Self {
            id: webhook.id,
            url: webhook.url,
            event_type: webhook.event_type,
            has_signing_secret: webhook.signing_secret.is_some(),
            is_active: webhook.is_active,
            description: webhook.description,
            created_at: webhook.created_at,
            updated_at: webhook.updated_at,
            deactivated_at: webhook.deactivated_at,
            deactivation_reason: webhook.deactivation_reason,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookEventLogResponse {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: WebhookEventType,
    pub status: WebhookDeliveryStatus,
    pub response_status_code: Option<i32>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<WebhookEventLog> for WebhookEventLogResponse {
    fn from(log: WebhookEventLog) -> Self {
        Self {
            id: log.id,
            webhook_id: log.webhook_id,
            event_type: log.event_type,
            status: log.status,
            response_status_code: log.response_status_code,
            error_message: log.error_message,
            retry_count: log.retry_count,
            created_at: log.created_at,
            sent_at: log.sent_at,
            completed_at: log.completed_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WebhookEventListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for WebhookEventListQuery {
    fn default() -> Self {
        Self {
            limit: Some(50),
            offset: Some(0),
        }
    }
}
