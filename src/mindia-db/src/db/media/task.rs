use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use mindia_core::models::{Task, TaskListQuery, TaskStats, TaskStatus, TaskType};

#[derive(Clone)]
pub struct TaskRepository {
    pool: PgPool,
}

impl TaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new task
    #[tracing::instrument(skip(self, payload))]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_task(
        &self,
        tenant_id: Uuid,
        task_type: TaskType,
        payload: serde_json::Value,
        priority: i32,
        scheduled_at: Option<DateTime<Utc>>,
        max_retries: Option<i32>,
        timeout_seconds: Option<i32>,
        depends_on: Option<Vec<Uuid>>,
    ) -> Result<Task> {
        let scheduled_at = scheduled_at.unwrap_or_else(Utc::now);
        let max_retries = max_retries.unwrap_or(3);
        let status = if scheduled_at > Utc::now() {
            TaskStatus::Scheduled
        } else {
            TaskStatus::Pending
        };

        // Use a transaction to ensure atomicity of task creation and notification
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction for task creation")?;

        let task: Task = sqlx::query_as::<Postgres, Task>(
            r#"
            INSERT INTO tasks (
                tenant_id, task_type, status, priority, payload, scheduled_at,
                max_retries, timeout_seconds, depends_on
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(task_type.to_string())
        .bind(status)
        .bind(priority)
        .bind(payload)
        .bind(scheduled_at)
        .bind(max_retries)
        .bind(timeout_seconds)
        .bind(depends_on.as_deref())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                tenant_id = %tenant_id,
                task_type = %task_type,
                "Failed to insert task into database"
            );
            anyhow::anyhow!("Failed to insert task into database: {}", e)
        })?;

        tracing::debug!(
            task_id = %task.id,
            tenant_id = %tenant_id,
            task_type = %task_type,
            "Task inserted successfully, sending notification"
        );

        // Notify workers so they can wake immediately instead of waiting for poll interval
        // This is inside the transaction to ensure atomicity
        // Make pg_notify non-fatal - workers will poll if LISTEN/NOTIFY fails
        if let Err(e) = sqlx::query("SELECT pg_notify('mindia_new_task', '')")
            .execute(&mut *tx)
            .await
        {
            tracing::warn!(
                error = %e,
                task_id = %task.id,
                "Failed to send pg_notify for new task, workers will discover task via polling"
            );
            // Don't return error - continue with commit
        } else {
            tracing::debug!(
                task_id = %task.id,
                "pg_notify sent successfully, committing transaction"
            );
        }

        // Commit the transaction
        tx.commit().await.map_err(|e| {
            tracing::error!(
                error = %e,
                task_id = %task.id,
                "Failed to commit transaction for task creation"
            );
            anyhow::anyhow!("Failed to commit transaction: {}", e)
        })?;

        tracing::info!(
            task_id = %task.id,
            tenant_id = %tenant_id,
            task_type = %task_type,
            priority = priority,
            "Task created"
        );

        Ok(task)
    }

    /// Get a task by ID with tenant check
    #[tracing::instrument(skip(self))]
    pub async fn get_task(&self, tenant_id: Uuid, task_id: Uuid) -> Result<Option<Task>> {
        let task: Option<Task> = sqlx::query_as::<Postgres, Task>(
            r#"
            SELECT
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            FROM tasks
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch task")?;

        Ok(task)
    }

    /// List tasks with optional filters (tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn list_tasks(&self, tenant_id: Uuid, query: TaskListQuery) -> Result<Vec<Task>> {
        let limit = query.limit.unwrap_or(50).min(1000);
        let offset = query.offset.unwrap_or(0);

        let mut sql = String::from(
            r#"
            SELECT
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            FROM tasks
            WHERE tenant_id = $1
            "#,
        );

        let mut conditions = Vec::new();
        let mut bind_count = 2; // Start at 2 because tenant_id is $1

        if query.status.is_some() {
            conditions.push(format!("AND status = ${}", bind_count));
            bind_count += 1;
        }

        if query.task_type.is_some() {
            conditions.push(format!("AND task_type = ${}", bind_count));
            bind_count += 1;
        }

        for condition in conditions {
            sql.push_str(&format!(" {} ", condition));
        }

        sql.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            bind_count,
            bind_count + 1
        ));

        let mut query_builder = sqlx::query_as::<_, Task>(&sql);
        query_builder = query_builder.bind(tenant_id);

        if let Some(status) = query.status {
            query_builder = query_builder.bind(status);
        }

        if let Some(task_type) = query.task_type {
            query_builder = query_builder.bind(task_type.to_string());
        }

        let tasks = query_builder
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list tasks")?;

        Ok(tasks)
    }

    /// Atomically claim the next available task (system-wide, used by workers)
    ///
    /// # Tenant Isolation Design
    ///
    /// **IMPORTANT:** This method queries tasks across ALL tenants without tenant filtering.
    /// This is intentional for shared worker pools where workers process tasks from any tenant.
    ///
    /// **Security Considerations:**
    /// - Task handlers MUST verify tenant access before processing tasks
    /// - Task payloads should be validated for tenant_id consistency
    /// - Workers should have appropriate authorization to process any tenant's tasks
    ///
    /// **Alternative Design (Tenant-Specific Workers):**
    /// If tenant-specific worker pools are needed, add tenant_id parameter:
    /// ```text
    /// pub async fn claim_next_task_for_tenant(&self, tenant_id: Uuid) -> Result<Option<Task>> {
    ///     // WHERE tenant_id = $1 AND status IN ('pending', 'scheduled') ...
    /// }
    /// ```
    ///
    /// # Current Implementation
    /// - Processes tasks from all tenants in priority order
    /// - Uses FOR UPDATE SKIP LOCKED for concurrent access safety
    /// - Task handlers are responsible for tenant verification
    #[tracing::instrument(skip(self))]
    pub async fn claim_next_task(&self) -> Result<Option<Task>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction")?;

        // Find and claim the next task across all tenants
        // NOTE: Intentionally queries all tenants for shared worker pool design
        // Task handlers must verify tenant access before processing
        let task: Option<Task> = sqlx::query_as::<Postgres, Task>(
            r#"
            SELECT
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            FROM tasks
            WHERE status IN ('pending', 'scheduled')
                AND scheduled_at <= NOW()
            ORDER BY priority DESC, scheduled_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to fetch next task")?;

        if let Some(task) = task {
            // Update the task status to running
            let updated_task: Task = sqlx::query_as::<Postgres, Task>(
                r#"
                UPDATE tasks
                SET status = 'running',
                    started_at = NOW(),
                    updated_at = NOW()
                WHERE id = $1
                RETURNING
                    id,
                    tenant_id,
                    task_type,
                    status,
                    priority,
                    payload,
                    result,
                    scheduled_at,
                    started_at,
                    completed_at,
                    retry_count,
                    max_retries,
                    timeout_seconds,
                    depends_on,
                    created_at,
                    updated_at
                "#,
            )
            .bind(task.id)
            .fetch_one(&mut *tx)
            .await
            .context("Failed to update task status")?;

            tx.commit().await.context("Failed to commit transaction")?;

            tracing::debug!(
                task_id = %updated_task.id,
                tenant_id = %updated_task.tenant_id,
                task_type = %updated_task.task_type,
                "Task claimed"
            );

            Ok(Some(updated_task))
        } else {
            tx.rollback().await.ok();
            Ok(None)
        }
    }

    /// Update task status (system method, no tenant check for workers)
    #[tracing::instrument(skip(self))]
    pub async fn update_status(&self, task_id: Uuid, status: TaskStatus) -> Result<Task> {
        let task: Task = sqlx::query_as::<Postgres, Task>(
            r#"
            UPDATE tasks
            SET status = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            "#,
        )
        .bind(task_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .context("Failed to update task status")?;

        tracing::debug!(
            task_id = %task_id,
            status = %status,
            "Task status updated"
        );

        Ok(task)
    }

    /// Mark task as completed with result (system method)
    #[tracing::instrument(skip(self, result))]
    pub async fn mark_completed(&self, task_id: Uuid, result: serde_json::Value) -> Result<Task> {
        let task: Task = sqlx::query_as::<Postgres, Task>(
            r#"
            UPDATE tasks
            SET status = 'completed',
                result = $2,
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            "#,
        )
        .bind(task_id)
        .bind(result)
        .fetch_one(&self.pool)
        .await
        .context("Failed to mark task as completed")?;

        tracing::info!(
            task_id = %task_id,
            tenant_id = %task.tenant_id,
            task_type = %task.task_type,
            "Task completed"
        );

        Ok(task)
    }

    /// Mark task as failed with error details (system method)
    #[tracing::instrument(skip(self, error))]
    pub async fn mark_failed(&self, task_id: Uuid, error: serde_json::Value) -> Result<Task> {
        let task: Task = sqlx::query_as::<Postgres, Task>(
            r#"
            UPDATE tasks
            SET status = 'failed',
                result = $2,
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            "#,
        )
        .bind(task_id)
        .bind(error)
        .fetch_one(&self.pool)
        .await
        .context("Failed to mark task as failed")?;

        tracing::error!(
            task_id = %task_id,
            tenant_id = %task.tenant_id,
            task_type = %task.task_type,
            retry_count = task.retry_count,
            "Task failed"
        );

        Ok(task)
    }

    /// Increment retry count and reset status to pending (system method)
    #[tracing::instrument(skip(self))]
    pub async fn increment_retry(&self, task_id: Uuid) -> Result<Task> {
        let task: Task = sqlx::query_as::<Postgres, Task>(
            r#"
            UPDATE tasks
            SET status = 'pending',
                retry_count = retry_count + 1,
                started_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            "#,
        )
        .bind(task_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to increment retry count")?;

        tracing::info!(
            task_id = %task_id,
            retry_count = task.retry_count,
            max_retries = task.max_retries,
            "Task retry scheduled"
        );

        Ok(task)
    }

    /// Check if all dependencies are completed (system method)
    #[tracing::instrument(skip(self))]
    pub async fn check_dependencies_completed(&self, depends_on: &[Uuid]) -> Result<bool> {
        if depends_on.is_empty() {
            return Ok(true);
        }

        let count: i64 = sqlx::query_scalar::<Postgres, i64>(
            r#"
            SELECT COUNT(*)
            FROM tasks
            WHERE id = ANY($1)
                AND status = 'completed'
            "#,
        )
        .bind(depends_on)
        .fetch_one(&self.pool)
        .await
        .context("Failed to check dependencies")?;

        Ok(count == depends_on.len() as i64)
    }

    /// Get aggregated task statistics for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn get_stats(&self, tenant_id: Uuid) -> Result<TaskStats> {
        use sqlx::Row;
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'pending') as pending,
                COUNT(*) FILTER (WHERE status = 'running') as running,
                COUNT(*) FILTER (WHERE status = 'completed') as completed,
                COUNT(*) FILTER (WHERE status = 'failed') as failed,
                COUNT(*) FILTER (WHERE status = 'scheduled') as scheduled,
                COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled
            FROM tasks
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch task stats")?;

        Ok(TaskStats {
            total: row.get::<Option<i64>, _>("total").unwrap_or(0),
            pending: row.get::<Option<i64>, _>("pending").unwrap_or(0),
            running: row.get::<Option<i64>, _>("running").unwrap_or(0),
            completed: row.get::<Option<i64>, _>("completed").unwrap_or(0),
            failed: row.get::<Option<i64>, _>("failed").unwrap_or(0),
            scheduled: row.get::<Option<i64>, _>("scheduled").unwrap_or(0),
            cancelled: row.get::<Option<i64>, _>("cancelled").unwrap_or(0),
        })
    }

    /// Cancel a pending or scheduled task
    #[tracing::instrument(skip(self))]
    pub async fn cancel_task(&self, tenant_id: Uuid, task_id: Uuid) -> Result<Task> {
        let task: Task = sqlx::query_as::<Postgres, Task>(
            r#"
            UPDATE tasks
            SET status = 'cancelled',
                updated_at = NOW()
            WHERE tenant_id = $1
                AND id = $2
                AND status IN ('pending', 'scheduled')
            RETURNING
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(task_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to cancel task - task not found or not in cancellable state")?;

        tracing::info!(
            task_id = %task_id,
            tenant_id = %tenant_id,
            "Task cancelled"
        );

        Ok(task)
    }

    /// Retry a failed task
    #[tracing::instrument(skip(self))]
    pub async fn retry_task(&self, tenant_id: Uuid, task_id: Uuid) -> Result<Task> {
        let task: Task = sqlx::query_as::<Postgres, Task>(
            r#"
            UPDATE tasks
            SET status = 'pending',
                retry_count = 0,
                started_at = NULL,
                completed_at = NULL,
                result = NULL,
                updated_at = NOW()
            WHERE tenant_id = $1
                AND id = $2
                AND status = 'failed'
            RETURNING
                id,
                tenant_id,
                task_type,
                status,
                priority,
                payload,
                result,
                scheduled_at,
                started_at,
                completed_at,
                retry_count,
                max_retries,
                timeout_seconds,
                depends_on,
                created_at,
                updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(task_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to retry task - task not found or not in failed state")?;

        tracing::info!(
            task_id = %task_id,
            tenant_id = %tenant_id,
            "Task manually retried"
        );

        Ok(task)
    }

    /// Delete finished tasks (completed, failed, cancelled) older than the given number of days.
    /// Used for automatic cleanup to prevent unbounded growth of the tasks table.
    /// Returns the number of rows deleted.
    #[tracing::instrument(skip(self))]
    pub async fn delete_old_finished_tasks(&self, older_than_days: i32) -> Result<u64> {
        use sqlx::Row;

        let result = sqlx::query(
            r#"
            WITH deleted AS (
                DELETE FROM tasks
                WHERE status IN ('completed', 'failed', 'cancelled')
                    AND COALESCE(completed_at, updated_at) < NOW() - ($1 * interval '1 day')
                RETURNING id
            )
            SELECT COUNT(*)::bigint FROM deleted
            "#,
        )
        .bind(older_than_days)
        .fetch_one(&self.pool)
        .await
        .context("Failed to delete old finished tasks")?;

        let count: i64 = result.get(0);
        let count = count.max(0) as u64;

        if count > 0 {
            tracing::info!(
                count = count,
                older_than_days = older_than_days,
                "Deleted old finished tasks"
            );
        }

        Ok(count)
    }
}
