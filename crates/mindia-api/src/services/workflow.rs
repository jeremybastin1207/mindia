//! Workflow service: trigger workflows on upload or manually, create task chains.

use anyhow::{Context, Result};
use mindia_core::models::{
    PluginExecutionPayload, Priority, TaskType, Workflow, WorkflowExecution, WorkflowStep,
};
use mindia_db::{WorkflowExecutionRepository, WorkflowRepository};
use mindia_worker::TaskQueue;
use uuid::Uuid;

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
            .create_in_transaction(workflow_id, tenant_id, media_id, task_ids.clone(), stop_on_failure)
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
