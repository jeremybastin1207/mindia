//! Workflow models for automated processing pipelines

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Single step in a workflow: run a plugin by name
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowStep {
    /// Action type: "plugin" for plugin execution
    pub action: String,
    /// Plugin name (e.g. aws_rekognition_moderation, replicate_deoldify)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
}

/// Workflow execution status (matches database enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "workflow_execution_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum WorkflowExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow definition (database row)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workflow {
    pub id: Uuid,
    pub tenant_id: Uuid,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workflow execution instance (one run of a workflow on a media item)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub tenant_id: Uuid,
    pub media_id: Uuid,
    pub status: WorkflowExecutionStatus,
    pub task_ids: Vec<Uuid>,
    pub current_step: i32,
    pub stop_on_failure: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
