use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, Hash)]
#[sqlx(type_name = "text")]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    VideoTranscode,
    GenerateEmbedding,
    PluginExecution,
    ContentModeration,
}

impl Display for TaskType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            TaskType::VideoTranscode => write!(f, "video_transcode"),
            TaskType::GenerateEmbedding => write!(f, "generate_embedding"),
            TaskType::PluginExecution => write!(f, "plugin_execution"),
            TaskType::ContentModeration => write!(f, "content_moderation"),
        }
    }
}

impl FromStr for TaskType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "video_transcode" => Ok(TaskType::VideoTranscode),
            "generate_embedding" => Ok(TaskType::GenerateEmbedding),
            "plugin_execution" => Ok(TaskType::PluginExecution),
            "content_moderation" => Ok(TaskType::ContentModeration),
            _ => Err(anyhow::anyhow!("Invalid task type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "task_status", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Scheduled,
    Cancelled,
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Scheduled => write!(f, "scheduled"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "running" => Ok(TaskStatus::Running),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "scheduled" => Ok(TaskStatus::Scheduled),
            "cancelled" => Ok(TaskStatus::Cancelled),
            _ => Err(anyhow::anyhow!("Invalid task status: {}", s)),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low = 3,
    #[default]
    Normal = 5,
    High = 7,
    Critical = 10,
}

impl Priority {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    pub fn from_i32(value: i32) -> Self {
        match value {
            0..=3 => Priority::Low,
            4..=6 => Priority::Normal,
            7..=9 => Priority::High,
            _ => Priority::Critical,
        }
    }
}

impl From<Priority> for i32 {
    fn from(priority: Priority) -> Self {
        priority as i32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub priority: i32,
    pub payload: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub timeout_seconds: Option<i32>,
    pub depends_on: Option<Vec<Uuid>>,
    /// When true, task is cancelled if any dependency fails (used by workflow chains).
    pub cancel_on_dep_failure: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Task {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Task {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            task_type: row.get::<String, _>("task_type").parse().map_err(|e| {
                sqlx::Error::Decode(format!("Failed to parse task_type: {}", e).into())
            })?,
            status: row.get("status"),
            priority: row.get("priority"),
            payload: row.get("payload"),
            result: row.get("result"),
            scheduled_at: row.get("scheduled_at"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            retry_count: row.get("retry_count"),
            max_retries: row.get("max_retries"),
            timeout_seconds: row.get("timeout_seconds"),
            depends_on: row.get::<Option<Vec<Uuid>>, _>("depends_on"),
            cancel_on_dep_failure: row
                .get::<Option<bool>, _>("cancel_on_dep_failure")
                .unwrap_or(false),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

impl Task {
    pub fn is_ready_to_run(&self) -> bool {
        matches!(self.status, TaskStatus::Pending | TaskStatus::Scheduled)
            && self.scheduled_at <= Utc::now()
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn should_timeout(&self, started_at: DateTime<Utc>) -> bool {
        if let Some(timeout) = self.timeout_seconds {
            let elapsed = Utc::now().signed_duration_since(started_at);
            elapsed.num_seconds() >= timeout as i64
        } else {
            false
        }
    }

    /// Extract the payload as a typed struct.
    /// Returns None if deserialization fails.
    pub fn payload_as<P: TaskPayload>(&self) -> Option<P> {
        serde_json::from_value(self.payload.clone()).ok()
    }

    /// Extract the payload as a typed struct, returning an error on failure.
    pub fn try_payload_as<P: TaskPayload>(&self) -> Result<P, serde_json::Error> {
        serde_json::from_value(self.payload.clone())
    }

    /// Extract the result as a typed struct.
    /// Returns None if result is not set or deserialization fails.
    pub fn result_as<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        self.result
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Create a new payload from a typed struct.
    /// Use this when creating tasks to ensure type consistency.
    pub fn payload_from<P: TaskPayload>(payload: &P) -> serde_json::Value {
        serde_json::to_value(payload).unwrap_or_default()
    }
}

/// Trait for type-safe task payloads
pub trait TaskPayload: Serialize + for<'de> Deserialize<'de> {
    fn task_type() -> TaskType;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTranscodePayload {
    pub video_id: Uuid,
}

impl TaskPayload for VideoTranscodePayload {
    fn task_type() -> TaskType {
        TaskType::VideoTranscode
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateEmbeddingPayload {
    pub entity_id: Uuid,
    pub entity_type: String, // "image", "video", "document"
    pub s3_url: String,
}

impl TaskPayload for GenerateEmbeddingPayload {
    fn task_type() -> TaskType {
        TaskType::GenerateEmbedding
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionPayload {
    pub plugin_name: String,
    pub media_id: Uuid,
    pub tenant_id: Uuid,
}

impl TaskPayload for PluginExecutionPayload {
    fn task_type() -> TaskType {
        TaskType::PluginExecution
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentModerationPayload {
    pub media_id: Uuid,
    pub media_type: String, // "image", "video"
    pub s3_key: String,
    pub s3_url: String,
}

impl TaskPayload for ContentModerationPayload {
    fn task_type() -> TaskType {
        TaskType::ContentModeration
    }
}

/// Response models for API endpoints
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: Uuid,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub priority: i32,
    pub payload: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            task_type: task.task_type,
            status: task.status,
            priority: task.priority,
            payload: task.payload,
            result: task.result,
            scheduled_at: task.scheduled_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
            retry_count: task.retry_count,
            max_retries: task.max_retries,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TaskStats {
    pub total: i64,
    pub pending: i64,
    pub running: i64,
    pub completed: i64,
    pub failed: i64,
    pub scheduled: i64,
    pub cancelled: i64,
}

#[derive(Debug, Deserialize)]
pub struct TaskListQuery {
    pub status: Option<TaskStatus>,
    pub task_type: Option<TaskType>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for TaskListQuery {
    fn default() -> Self {
        Self {
            status: None,
            task_type: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_display() {
        assert_eq!(TaskType::VideoTranscode.to_string(), "video_transcode");
        assert_eq!(
            TaskType::GenerateEmbedding.to_string(),
            "generate_embedding"
        );
    }

    #[test]
    fn test_task_type_from_str() {
        assert_eq!(
            "video_transcode".parse::<TaskType>().unwrap(),
            TaskType::VideoTranscode
        );
        assert_eq!(
            "generate_embedding".parse::<TaskType>().unwrap(),
            TaskType::GenerateEmbedding
        );
        assert!("invalid_type".parse::<TaskType>().is_err());
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Pending.to_string(), "pending");
        assert_eq!(TaskStatus::Running.to_string(), "running");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::Failed.to_string(), "failed");
        assert_eq!(TaskStatus::Scheduled.to_string(), "scheduled");
        assert_eq!(TaskStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_task_status_from_str() {
        assert_eq!(
            "pending".parse::<TaskStatus>().unwrap(),
            TaskStatus::Pending
        );
        assert_eq!(
            "running".parse::<TaskStatus>().unwrap(),
            TaskStatus::Running
        );
        assert_eq!(
            "completed".parse::<TaskStatus>().unwrap(),
            TaskStatus::Completed
        );
        assert_eq!("failed".parse::<TaskStatus>().unwrap(), TaskStatus::Failed);
        assert_eq!(
            "scheduled".parse::<TaskStatus>().unwrap(),
            TaskStatus::Scheduled
        );
        assert_eq!(
            "cancelled".parse::<TaskStatus>().unwrap(),
            TaskStatus::Cancelled
        );
        assert!("invalid_status".parse::<TaskStatus>().is_err());
    }

    #[test]
    fn test_priority_as_i32() {
        assert_eq!(Priority::Low.as_i32(), 3);
        assert_eq!(Priority::Normal.as_i32(), 5);
        assert_eq!(Priority::High.as_i32(), 7);
        assert_eq!(Priority::Critical.as_i32(), 10);
    }

    #[test]
    fn test_priority_from_i32() {
        assert_eq!(Priority::from_i32(0), Priority::Low);
        assert_eq!(Priority::from_i32(3), Priority::Low);
        assert_eq!(Priority::from_i32(4), Priority::Normal);
        assert_eq!(Priority::from_i32(5), Priority::Normal);
        assert_eq!(Priority::from_i32(6), Priority::Normal);
        assert_eq!(Priority::from_i32(7), Priority::High);
        assert_eq!(Priority::from_i32(9), Priority::High);
        assert_eq!(Priority::from_i32(10), Priority::Critical);
        assert_eq!(Priority::from_i32(100), Priority::Critical);
    }

    #[test]
    fn test_priority_into_i32() {
        let priority: i32 = Priority::Normal.into();
        assert_eq!(priority, 5);
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(Priority::default(), Priority::Normal);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Critical);
    }

    #[test]
    fn test_task_is_ready_to_run_with_pending_status() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Pending,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now() - chrono::Duration::seconds(10),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(task.is_ready_to_run());
    }

    #[test]
    fn test_task_is_ready_to_run_with_scheduled_status() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::GenerateEmbedding,
            status: TaskStatus::Scheduled,
            priority: Priority::High.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now() - chrono::Duration::seconds(5),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(task.is_ready_to_run());
    }

    #[test]
    fn test_task_is_not_ready_when_scheduled_in_future() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Scheduled,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now() + chrono::Duration::seconds(3600),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!task.is_ready_to_run());
    }

    #[test]
    fn test_task_is_not_ready_when_running() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Running,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now() - chrono::Duration::seconds(10),
            started_at: Some(Utc::now()),
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!task.is_ready_to_run());
    }

    #[test]
    fn test_task_can_retry_when_under_limit() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Failed,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 2,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(task.can_retry());
    }

    #[test]
    fn test_task_cannot_retry_when_at_limit() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Failed,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 3,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!task.can_retry());
    }

    #[test]
    fn test_task_cannot_retry_when_over_limit() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Failed,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 5,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!task.can_retry());
    }

    #[test]
    fn test_task_should_timeout_when_exceeded() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Running,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: Some(60),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let started_at = Utc::now() - chrono::Duration::seconds(120);
        assert!(task.should_timeout(started_at));
    }

    #[test]
    fn test_task_should_not_timeout_when_under_limit() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Running,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let started_at = Utc::now() - chrono::Duration::seconds(10);
        assert!(!task.should_timeout(started_at));
    }

    #[test]
    fn test_task_should_not_timeout_when_no_timeout_set() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Running,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: None,
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let started_at = Utc::now() - chrono::Duration::days(365);
        assert!(!task.should_timeout(started_at));
    }

    #[test]
    fn test_task_should_timeout_at_exact_limit() {
        let task = Task {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Running,
            priority: Priority::Normal.as_i32(),
            payload: serde_json::json!({}),
            result: None,
            scheduled_at: Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: Some(60),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let started_at = Utc::now() - chrono::Duration::seconds(60);
        assert!(task.should_timeout(started_at));
    }

    #[test]
    fn test_task_list_query_default() {
        let query = TaskListQuery::default();
        assert_eq!(query.status, None);
        assert_eq!(query.task_type, None);
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_task_response_from_task() {
        let task_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let created_at = Utc::now();
        let updated_at = Utc::now();
        let scheduled_at = Utc::now();
        let payload = serde_json::json!({"video_id": "test"});

        let task = Task {
            id: task_id,
            tenant_id,
            task_type: TaskType::VideoTranscode,
            status: TaskStatus::Pending,
            priority: Priority::High.as_i32(),
            payload: payload.clone(),
            result: None,
            scheduled_at,
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: Some(3600),
            depends_on: None,
            cancel_on_dep_failure: false,
            created_at,
            updated_at,
        };

        let response = TaskResponse::from(task);

        assert_eq!(response.id, task_id);
        assert_eq!(response.task_type, TaskType::VideoTranscode);
        assert_eq!(response.status, TaskStatus::Pending);
        assert_eq!(response.priority, 7);
        assert_eq!(response.payload, payload);
        assert_eq!(response.result, None);
        assert_eq!(response.retry_count, 0);
        assert_eq!(response.max_retries, 3);
    }

    #[test]
    fn test_task_payload_trait_video_transcode() {
        assert_eq!(VideoTranscodePayload::task_type(), TaskType::VideoTranscode);
    }

    #[test]
    fn test_task_payload_trait_generate_embedding() {
        assert_eq!(
            GenerateEmbeddingPayload::task_type(),
            TaskType::GenerateEmbedding
        );
    }
}
