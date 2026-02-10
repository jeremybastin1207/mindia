//! Workflow and workflow execution repositories

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use mindia_core::models::{Workflow, WorkflowExecution, WorkflowExecutionStatus};

#[derive(Clone)]
pub struct WorkflowRepository {
    pool: PgPool,
}

impl WorkflowRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        tenant_id: Uuid,
        name: &str,
        description: Option<&str>,
        enabled: bool,
        steps: serde_json::Value,
        trigger_on_upload: bool,
        stop_on_failure: bool,
        media_types: Option<&[String]>,
        folder_ids: Option<&[Uuid]>,
        content_types: Option<&[String]>,
        metadata_filter: Option<serde_json::Value>,
    ) -> Result<Workflow> {
        let now = Utc::now();
        let w = sqlx::query_as::<Postgres, Workflow>(
            r#"
            INSERT INTO workflows (
                tenant_id, name, description, enabled, steps,
                trigger_on_upload, stop_on_failure,
                media_types, folder_ids, content_types, metadata_filter,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, tenant_id, name, description, enabled, steps,
                trigger_on_upload, stop_on_failure,
                media_types, folder_ids, content_types, metadata_filter,
                created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(name)
        .bind(description)
        .bind(enabled)
        .bind(&steps)
        .bind(trigger_on_upload)
        .bind(stop_on_failure)
        .bind(media_types)
        .bind(folder_ids)
        .bind(content_types)
        .bind(metadata_filter.as_ref())
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create workflow")?;
        Ok(w)
    }

    pub async fn get(&self, tenant_id: Uuid, workflow_id: Uuid) -> Result<Option<Workflow>> {
        let w = sqlx::query_as::<Postgres, Workflow>(
            r#"
            SELECT id, tenant_id, name, description, enabled, steps,
                trigger_on_upload, stop_on_failure,
                media_types, folder_ids, content_types, metadata_filter,
                created_at, updated_at
            FROM workflows
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get workflow")?;
        Ok(w)
    }

    pub async fn list(&self, tenant_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Workflow>> {
        let rows = sqlx::query_as::<Postgres, Workflow>(
            r#"
            SELECT id, tenant_id, name, description, enabled, steps,
                trigger_on_upload, stop_on_failure,
                media_types, folder_ids, content_types, metadata_filter,
                created_at, updated_at
            FROM workflows
            WHERE tenant_id = $1
            ORDER BY name ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list workflows")?;
        Ok(rows)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        tenant_id: Uuid,
        workflow_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        enabled: Option<bool>,
        steps: Option<serde_json::Value>,
        trigger_on_upload: Option<bool>,
        stop_on_failure: Option<bool>,
        media_types: Option<Option<Vec<String>>>,
        folder_ids: Option<Option<Vec<Uuid>>>,
        content_types: Option<Option<Vec<String>>>,
        metadata_filter: Option<Option<serde_json::Value>>,
    ) -> Result<Option<Workflow>> {
        let now = Utc::now();
        // Build dynamic update: only non-None fields
        let w = sqlx::query_as::<Postgres, Workflow>(
            r#"
            UPDATE workflows
            SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                enabled = COALESCE($5, enabled),
                steps = COALESCE($6, steps),
                trigger_on_upload = COALESCE($7, trigger_on_upload),
                stop_on_failure = COALESCE($8, stop_on_failure),
                media_types = COALESCE($9, media_types),
                folder_ids = COALESCE($10, folder_ids),
                content_types = COALESCE($11, content_types),
                metadata_filter = COALESCE($12, metadata_filter),
                updated_at = $13
            WHERE tenant_id = $1 AND id = $2
            RETURNING id, tenant_id, name, description, enabled, steps,
                trigger_on_upload, stop_on_failure,
                media_types, folder_ids, content_types, metadata_filter,
                created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(workflow_id)
        .bind(name)
        .bind(description)
        .bind(enabled)
        .bind(steps.as_ref())
        .bind(trigger_on_upload)
        .bind(stop_on_failure)
        .bind(media_types.and_then(|x| x).as_deref())
        .bind(folder_ids.and_then(|x| x).as_deref())
        .bind(content_types.and_then(|x| x).as_deref())
        .bind(metadata_filter.and_then(|x| x).as_ref())
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to update workflow")?;
        Ok(w)
    }

    pub async fn delete(&self, tenant_id: Uuid, workflow_id: Uuid) -> Result<bool> {
        let r = sqlx::query(r#"DELETE FROM workflows WHERE tenant_id = $1 AND id = $2"#)
            .bind(tenant_id)
            .bind(workflow_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete workflow")?;
        Ok(r.rows_affected() > 0)
    }

    /// Find enabled workflows that have trigger_on_upload and match the given upload criteria.
    /// media_type must match if workflow has media_types set; folder_id must be in folder_ids if set; etc.
    pub async fn match_workflows_for_upload(
        &self,
        tenant_id: Uuid,
        media_type: &str,
        folder_id: Option<Uuid>,
        content_type: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Vec<Workflow>> {
        let rows = sqlx::query_as::<Postgres, Workflow>(
            r#"
            SELECT id, tenant_id, name, description, enabled, steps,
                trigger_on_upload, stop_on_failure,
                media_types, folder_ids, content_types, metadata_filter,
                created_at, updated_at
            FROM workflows
            WHERE tenant_id = $1
                AND enabled = true
                AND trigger_on_upload = true
                AND (
                    media_types IS NULL
                    OR media_types = '{}'
                    OR $2::text = ANY(media_types)
                )
                AND (
                    folder_ids IS NULL
                    OR folder_ids = '{}'
                    OR ($3::uuid IS NOT NULL AND $3 = ANY(folder_ids))
                )
                AND (
                    content_types IS NULL
                    OR content_types = '{}'
                    OR $4::text = ANY(content_types)
                )
            ORDER BY name ASC
            "#,
        )
        .bind(tenant_id)
        .bind(media_type)
        .bind(folder_id)
        .bind(content_type)
        .fetch_all(&self.pool)
        .await
        .context("Failed to match workflows for upload")?;

        // Filter by metadata_filter: if set, metadata must contain the required keys/values
        let filtered: Vec<Workflow> = if let Some(meta) = metadata {
            rows.into_iter()
                .filter(|w| match &w.metadata_filter {
                    None => true,
                    Some(f) => {
                        let obj = match f.as_object() {
                            Some(o) => o,
                            None => return true,
                        };
                        if obj.is_empty() {
                            return true;
                        }
                        let m = match meta.as_object() {
                            Some(m) => m,
                            None => return false,
                        };
                        obj.iter().all(|(k, v)| m.get(k) == Some(v))
                    }
                })
                .collect()
        } else {
            rows.into_iter()
                .filter(|w| {
                    w.metadata_filter.is_none()
                        || w.metadata_filter
                            .as_ref()
                            .map(|v| v.as_object().map(|o| o.is_empty()).unwrap_or(true))
                            .unwrap_or(true)
                })
                .collect()
        };
        Ok(filtered)
    }
}

#[derive(Clone)]
pub struct WorkflowExecutionRepository {
    pool: PgPool,
}

impl WorkflowExecutionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        workflow_id: Uuid,
        tenant_id: Uuid,
        media_id: Uuid,
        task_ids: Vec<Uuid>,
        stop_on_failure: bool,
    ) -> Result<WorkflowExecution> {
        let status = if task_ids.is_empty() {
            WorkflowExecutionStatus::Completed
        } else {
            WorkflowExecutionStatus::Pending
        };
        let now = Utc::now();
        let e = sqlx::query_as::<Postgres, WorkflowExecution>(
            r#"
            INSERT INTO workflow_executions (
                workflow_id, tenant_id, media_id, status, task_ids, current_step, stop_on_failure,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 0, $6, $7, $8)
            RETURNING id, workflow_id, tenant_id, media_id, status, task_ids, current_step, stop_on_failure,
                created_at, updated_at
            "#,
        )
        .bind(workflow_id)
        .bind(tenant_id)
        .bind(media_id)
        .bind(status)
        .bind(&task_ids)
        .bind(stop_on_failure)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create workflow execution")?;
        Ok(e)
    }

    pub async fn get(&self, execution_id: Uuid) -> Result<Option<WorkflowExecution>> {
        let e = sqlx::query_as::<Postgres, WorkflowExecution>(
            r#"
            SELECT id, workflow_id, tenant_id, media_id, status, task_ids, current_step, stop_on_failure,
                created_at, updated_at
            FROM workflow_executions
            WHERE id = $1
            "#,
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get workflow execution")?;
        Ok(e)
    }

    pub async fn get_by_tenant_and_id(
        &self,
        tenant_id: Uuid,
        execution_id: Uuid,
    ) -> Result<Option<WorkflowExecution>> {
        let e = sqlx::query_as::<Postgres, WorkflowExecution>(
            r#"
            SELECT id, workflow_id, tenant_id, media_id, status, task_ids, current_step, stop_on_failure,
                created_at, updated_at
            FROM workflow_executions
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get workflow execution")?;
        Ok(e)
    }

    /// Find workflow execution that contains the given task_id (for post-completion hook).
    pub async fn get_by_task_id(&self, task_id: Uuid) -> Result<Option<WorkflowExecution>> {
        let e = sqlx::query_as::<Postgres, WorkflowExecution>(
            r#"
            SELECT id, workflow_id, tenant_id, media_id, status, task_ids, current_step, stop_on_failure,
                created_at, updated_at
            FROM workflow_executions
            WHERE $1 = ANY(task_ids)
            LIMIT 1
            "#,
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get workflow execution by task id")?;
        Ok(e)
    }

    pub async fn list_by_workflow(
        &self,
        tenant_id: Uuid,
        workflow_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkflowExecution>> {
        let rows = sqlx::query_as::<Postgres, WorkflowExecution>(
            r#"
            SELECT id, workflow_id, tenant_id, media_id, status, task_ids, current_step, stop_on_failure,
                created_at, updated_at
            FROM workflow_executions
            WHERE tenant_id = $1 AND workflow_id = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(workflow_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list workflow executions")?;
        Ok(rows)
    }

    pub async fn update_task_ids(&self, execution_id: Uuid, task_ids: &[Uuid]) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE workflow_executions
            SET task_ids = $2, updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(execution_id)
        .bind(task_ids)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to update workflow execution task_ids")?;
        Ok(())
    }

    pub async fn update_status(
        &self,
        execution_id: Uuid,
        status: WorkflowExecutionStatus,
        current_step: Option<i32>,
    ) -> Result<()> {
        let now = Utc::now();
        if let Some(step) = current_step {
            sqlx::query(
                r#"
                UPDATE workflow_executions
                SET status = $2, current_step = $3, updated_at = $4
                WHERE id = $1
                "#,
            )
            .bind(execution_id)
            .bind(status)
            .bind(step)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to update workflow execution status")?;
        } else {
            sqlx::query(
                r#"
                UPDATE workflow_executions
                SET status = $2, updated_at = $3
                WHERE id = $1
                "#,
            )
            .bind(execution_id)
            .bind(status)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to update workflow execution status")?;
        }
        Ok(())
    }

    /// Derive workflow execution status from its task statuses and update the record.
    pub async fn update_status_from_tasks(
        &self,
        execution_id: Uuid,
    ) -> Result<Option<WorkflowExecutionStatus>> {
        use sqlx::Row;
        let exec = match self.get(execution_id).await? {
            Some(e) => e,
            None => return Ok(None),
        };
        if exec.task_ids.is_empty() {
            return Ok(Some(exec.status));
        }
        let task_ids = &exec.task_ids;
        let _stop_on_failure = exec.stop_on_failure;

        let rows = sqlx::query(
            r#"
            SELECT id, status
            FROM tasks
            WHERE id = ANY($1)
            ORDER BY array_position($1, id)
            "#,
        )
        .bind(task_ids)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch task statuses")?;

        let statuses: Vec<String> = rows.iter().map(|r| r.get::<String, _>("status")).collect();

        let (new_status, current_step) = derive_workflow_status(&statuses);
        self.update_status(execution_id, new_status, Some(current_step))
            .await
            .context("Failed to update workflow execution status")?;
        Ok(Some(new_status))
    }
}

fn derive_workflow_status(task_statuses: &[String]) -> (WorkflowExecutionStatus, i32) {
    #[allow(unused_assignments)]
    let mut current_step = 0i32;
    for (i, s) in task_statuses.iter().enumerate() {
        current_step = i as i32;
        match s.as_str() {
            "completed" => continue,
            "failed" => {
                return (WorkflowExecutionStatus::Failed, current_step);
            }
            "cancelled" => {
                return (WorkflowExecutionStatus::Cancelled, current_step);
            }
            "pending" | "scheduled" | "running" => {
                return (WorkflowExecutionStatus::Running, current_step);
            }
            _ => continue,
        }
    }
    (
        WorkflowExecutionStatus::Completed,
        task_statuses.len().saturating_sub(1).max(0) as i32,
    )
}
