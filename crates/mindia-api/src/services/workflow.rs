//! Workflow service: CRUD, validation, and trigger workflows on upload or manually.

use anyhow::{Context, Result};
use mindia_core::models::{
    PluginExecutionPayload, Priority, TaskType, Workflow, WorkflowExecution, WorkflowStep,
};
use mindia_db::media::{WorkflowExecutionRepository, WorkflowRepository};
use mindia_worker::TaskQueue;
use uuid::Uuid;

const MAX_WORKFLOW_NAME_LEN: usize = 255;
const MAX_WORKFLOW_STEPS: usize = 50;

#[derive(Clone)]
pub struct WorkflowService {
    workflow_repo: WorkflowRepository,
    execution_repo: WorkflowExecutionRepository,
    task_queue: TaskQueue,
}

impl WorkflowService {
    pub fn new(
        workflow_repo: WorkflowRepository,
        execution_repo: WorkflowExecutionRepository,
        task_queue: TaskQueue,
    ) -> Self {
        Self {
            workflow_repo,
            execution_repo,
            task_queue,
        }
    }

    fn validate_workflow_name(name: &str) -> Result<()> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Workflow name cannot be empty");
        }
        if trimmed.len() > MAX_WORKFLOW_NAME_LEN {
            anyhow::bail!(
                "Workflow name must be at most {} characters",
                MAX_WORKFLOW_NAME_LEN
            );
        }
        Ok(())
    }

    fn validate_workflow_steps(steps: &serde_json::Value) -> Result<()> {
        let arr = steps
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Workflow steps must be a JSON array"))?;
        if arr.is_empty() {
            anyhow::bail!("Workflow must have at least one step");
        }
        if arr.len() > MAX_WORKFLOW_STEPS {
            anyhow::bail!("Workflow may have at most {} steps", MAX_WORKFLOW_STEPS);
        }
        let _: Vec<WorkflowStep> =
            serde_json::from_value(steps.clone()).context("Invalid workflow steps")?;
        Ok(())
    }

    /// Create a workflow after validating name and steps.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_workflow(
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
        Self::validate_workflow_name(name)?;
        Self::validate_workflow_steps(&steps)?;
        self.workflow_repo
            .create(
                tenant_id,
                name,
                description,
                enabled,
                steps,
                trigger_on_upload,
                stop_on_failure,
                media_types,
                folder_ids,
                content_types,
                metadata_filter,
            )
            .await
    }

    pub async fn list_workflows(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Workflow>> {
        self.workflow_repo.list(tenant_id, limit, offset).await
    }

    pub async fn get_workflow(
        &self,
        tenant_id: Uuid,
        workflow_id: Uuid,
    ) -> Result<Option<Workflow>> {
        self.workflow_repo.get(tenant_id, workflow_id).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_workflow(
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
        if let Some(ref n) = name {
            Self::validate_workflow_name(n)?;
        }
        if let Some(ref s) = steps {
            Self::validate_workflow_steps(s)?;
        }
        self.workflow_repo
            .update(
                tenant_id,
                workflow_id,
                name.as_deref(),
                description.as_deref(),
                enabled,
                steps,
                trigger_on_upload,
                stop_on_failure,
                media_types,
                folder_ids,
                content_types,
                metadata_filter,
            )
            .await
    }

    pub async fn delete_workflow(&self, tenant_id: Uuid, workflow_id: Uuid) -> Result<bool> {
        self.workflow_repo.delete(tenant_id, workflow_id).await
    }

    pub async fn list_workflow_executions(
        &self,
        tenant_id: Uuid,
        workflow_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkflowExecution>> {
        self.execution_repo
            .list_by_workflow(tenant_id, workflow_id, limit, offset)
            .await
    }

    pub async fn get_workflow_execution(
        &self,
        tenant_id: Uuid,
        execution_id: Uuid,
    ) -> Result<Option<WorkflowExecution>> {
        self.execution_repo
            .get_by_tenant_and_id(tenant_id, execution_id)
            .await
    }

    /// Trigger a workflow on a media item: submit all step tasks, then create execution in a transaction.
    pub async fn trigger_workflow(
        &self,
        tenant_id: Uuid,
        workflow_id: Uuid,
        media_id: Uuid,
    ) -> Result<WorkflowExecution> {
        let workflow = self
            .workflow_repo
            .get(tenant_id, workflow_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found or access denied"))?;
        if !workflow.enabled {
            anyhow::bail!("Workflow is disabled");
        }
        let steps: Vec<WorkflowStep> = serde_json::from_value(workflow.steps.clone())
            .context("Invalid workflow steps JSON")?;
        if steps.is_empty() {
            anyhow::bail!("Workflow has no steps");
        }
        let stop_on_failure = workflow.stop_on_failure;

        let mut task_ids = Vec::with_capacity(steps.len());
        let mut depends_on: Option<Vec<Uuid>> = None;

        for step in steps {
            let plugin_name = match (step.action.as_str(), step.plugin_name.as_deref()) {
                ("plugin", Some(name)) => name.to_string(),
                _ => continue,
            };
            let payload = PluginExecutionPayload {
                plugin_name: plugin_name.clone(),
                media_id,
                tenant_id,
            };
            let payload_json =
                serde_json::to_value(&payload).context("Serialize plugin payload")?;
            let task_id = self
                .task_queue
                .submit_task(
                    tenant_id,
                    TaskType::PluginExecution,
                    payload_json,
                    Priority::Normal,
                    None,
                    depends_on.clone(),
                    stop_on_failure,
                )
                .await
                .context("Submit workflow step task")?;
            task_ids.push(task_id);
            depends_on = Some(vec![task_id]);
        }

        let execution = self
            .execution_repo
            .create_in_transaction(
                workflow_id,
                tenant_id,
                media_id,
                task_ids.clone(),
                stop_on_failure,
            )
            .await
            .context("Create workflow execution in transaction")?;

        let mut out = execution;
        out.task_ids = task_ids;
        Ok(out)
    }

    /// Find workflows matching the upload and trigger them (called from notify_upload).
    pub async fn match_and_trigger(
        &self,
        tenant_id: Uuid,
        media_id: Uuid,
        media_type: &str,
        folder_id: Option<Uuid>,
        content_type: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Vec<WorkflowExecution>> {
        let workflows = self
            .workflow_repo
            .match_workflows_for_upload(tenant_id, media_type, folder_id, content_type, metadata)
            .await
            .context("Match workflows for upload")?;
        let mut executions = Vec::with_capacity(workflows.len());
        for w in workflows {
            match self.trigger_workflow(tenant_id, w.id, media_id).await {
                Ok(exec) => executions.push(exec),
                Err(e) => {
                    tracing::warn!(
                        workflow_id = %w.id,
                        workflow_name = %w.name,
                        error = %e,
                        "Failed to trigger workflow on upload"
                    );
                }
            }
        }
        Ok(executions)
    }
}
