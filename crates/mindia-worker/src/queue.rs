//! Task queue: worker pool, LISTEN/NOTIFY or polling, retry, and submission.
//!
//! Shutdown: [`TaskQueue::shutdown`] signals the pool to stop; it does not wait for
//! in-flight tasks. For graceful shutdown, coordinate with your runtime and allow
//! time for running tasks to finish before process exit.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::sleep;
use uuid::Uuid;

use mindia_core::models::{Priority, Task, TaskType};
use mindia_core::{CapacityGate, TaskError};
use mindia_db::TaskRepository;
use mindia_infra::RateLimiter;

use crate::context::TaskHandlerContext;

/// Channel name for PostgreSQL LISTEN/NOTIFY when a new task is created.
pub const TASK_NOTIFY_CHANNEL: &str = "mindia_new_task";

/// Maximum delay in seconds before retrying a failed task. Caps exponential backoff
/// so that high retry counts do not produce excessively long delays.
pub const MAX_RETRY_BACKOFF_SECS: u64 = 300;

/// Computes backoff in seconds for a given retry count (exponential with cap).
#[inline]
pub(crate) fn compute_retry_backoff_seconds(retry_count: i32) -> u64 {
    (2_u64.pow(retry_count as u32)).min(MAX_RETRY_BACKOFF_SECS)
}

/// Optional sender to notify when a task finishes (e.g. for workflow execution updates).
/// Not cloned into the worker; use with TaskQueue::new_with_task_finished.
pub type TaskFinishedSender = mpsc::Sender<(Uuid, mindia_core::models::TaskStatus)>;

#[derive(Clone)]
pub struct TaskQueueConfig {
    pub max_workers: usize,
    pub poll_interval_ms: u64,
    pub default_timeout_seconds: i32,
    pub max_retries: i32,
    /// Interval in seconds between runs of the stale task reaper.
    pub stale_task_reap_interval_secs: u64,
    /// Grace period in seconds added to task timeout before reaping stale running tasks.
    pub stale_task_grace_period_secs: i64,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            max_workers: 4,
            poll_interval_ms: 1000,
            default_timeout_seconds: 3600,
            max_retries: 3,
            stale_task_reap_interval_secs: 60,
            stale_task_grace_period_secs: 300,
        }
    }
}

pub struct TaskQueue {
    repository: TaskRepository,
    rate_limiter: RateLimiter,
    config: TaskQueueConfig,
    shutdown_tx: mpsc::Sender<()>,
}

impl TaskQueue {
    /// Create a new TaskQueue with a weak reference to the dispatch context.
    ///
    /// If `pool` is `Some`, the worker uses PostgreSQL LISTEN/NOTIFY to wake immediately
    /// when tasks are created, in addition to polling at `poll_interval_ms`.
    /// If `pool` is `None`, only polling is used.
    ///
    /// If `capacity_gate` is `Some`, the worker checks it before claiming tasks. When the
    /// gate returns false (e.g. high CPU/memory), the worker skips claiming for that cycle.
    pub fn new(
        repository: TaskRepository,
        rate_limiter: RateLimiter,
        config: TaskQueueConfig,
        context: Weak<dyn TaskHandlerContext>,
        pool: Option<sqlx::PgPool>,
        capacity_gate: Option<Arc<dyn CapacityGate>>,
        task_finished_tx: Option<TaskFinishedSender>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let repo_clone = repository.clone();
        let limiter_clone = rate_limiter.clone();
        let config_clone = config.clone();

        tokio::spawn(async move {
            Self::worker_pool(
                repo_clone,
                limiter_clone,
                config_clone,
                context,
                shutdown_rx,
                pool,
                capacity_gate,
                task_finished_tx,
            )
            .await;
        });

        Self {
            repository,
            rate_limiter,
            config,
            shutdown_tx,
        }
    }

    /// Creates a TaskQueue that does not spawn a worker.
    /// Use for temporary state that is dropped before the real queue; tasks submitted here
    /// are written to the DB and will be picked up by the real worker.
    pub fn new_no_worker(
        repository: TaskRepository,
        rate_limiter: RateLimiter,
        config: TaskQueueConfig,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        drop(shutdown_rx);
        Self {
            repository,
            rate_limiter,
            config,
            shutdown_tx,
        }
    }

    /// Submit a new task to the queue.
    #[tracing::instrument(skip(self, payload))]
    #[allow(clippy::too_many_arguments)]
    pub async fn submit_task(
        &self,
        tenant_id: Uuid,
        task_type: TaskType,
        payload: serde_json::Value,
        priority: Priority,
        scheduled_at: Option<DateTime<Utc>>,
        depends_on: Option<Vec<Uuid>>,
        cancel_on_dep_failure: bool,
    ) -> Result<Uuid> {
        let task = self
            .repository
            .create_task(
                tenant_id,
                task_type.clone(),
                payload,
                priority.as_i32(),
                scheduled_at,
                Some(self.config.max_retries),
                Some(self.config.default_timeout_seconds),
                depends_on,
                cancel_on_dep_failure,
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    tenant_id = %tenant_id,
                    task_type = %task_type,
                    priority = priority.as_i32(),
                    "Failed to create task in repository"
                );
                anyhow::anyhow!("Failed to create task in repository: {}", e)
            })?;

        tracing::info!(
            task_id = %task.id,
            task_type = %task_type,
            priority = priority.as_i32(),
            "Task submitted to queue"
        );

        Ok(task.id)
    }

    #[allow(clippy::too_many_arguments)]
    async fn worker_pool(
        repository: TaskRepository,
        rate_limiter: RateLimiter,
        config: TaskQueueConfig,
        context: Weak<dyn TaskHandlerContext>,
        mut shutdown_rx: mpsc::Receiver<()>,
        pool: Option<sqlx::PgPool>,
        capacity_gate: Option<Arc<dyn CapacityGate>>,
        task_finished_tx: Option<TaskFinishedSender>,
    ) {
        let use_listen = pool.is_some();
        tracing::info!(
            max_workers = config.max_workers,
            poll_interval_ms = config.poll_interval_ms,
            listen_notify = use_listen,
            capacity_gate = capacity_gate.is_some(),
            "Task queue worker pool started"
        );

        let semaphore = Arc::new(Semaphore::new(config.max_workers));
        let poll_interval = Duration::from_millis(config.poll_interval_ms);

        // Channel to wake the main loop when LISTEN receives a NOTIFY (avoids blocking on recv when no pool).
        let (notify_tx, mut notify_rx) = mpsc::channel::<()>(16);
        if let Some(pool) = pool {
            let tx = notify_tx.clone();
            tokio::spawn(async move {
                loop {
                    match sqlx::postgres::PgListener::connect_with(&pool).await {
                        Ok(mut listener) => {
                            if let Err(e) = listener.listen(TASK_NOTIFY_CHANNEL).await {
                                tracing::warn!(error = %e, "LISTEN failed, will retry");
                                tokio::time::sleep(Duration::from_secs(5)).await;
                                continue;
                            }
                            while listener.recv().await.is_ok() {
                                let _ = tx.send(()).await;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "PgListener connect failed, will retry");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                }
            });
        }

        // Spawn stale task reaper (if interval > 0)
        let (reaper_shutdown_tx, mut reaper_shutdown_rx) = mpsc::channel::<()>(1);
        if config.stale_task_reap_interval_secs > 0 {
            let repo_for_reaper = repository.clone();
            let reap_interval = Duration::from_secs(config.stale_task_reap_interval_secs);
            let grace_period = config.stale_task_grace_period_secs;
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(reap_interval);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Err(e) = repo_for_reaper.reap_stale_running_tasks(grace_period).await {
                                tracing::error!(error = %e, "Stale task reaper failed");
                            }
                        }
                        _ = reaper_shutdown_rx.recv() => break,
                    }
                }
            });
        }

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("Task queue worker pool shutting down");
                    let _ = reaper_shutdown_tx.send(()).await;
                    break;
                }
                _ = notify_rx.recv() => {
                    Self::claim_and_dispatch_one(
                        &repository,
                        &rate_limiter,
                        &semaphore,
                        &context,
                        capacity_gate.as_deref(),
                        task_finished_tx.clone(),
                    ).await;
                }
                _ = sleep(poll_interval) => {
                    Self::claim_and_dispatch_one(
                        &repository,
                        &rate_limiter,
                        &semaphore,
                        &context,
                        capacity_gate.as_deref(),
                        task_finished_tx.clone(),
                    ).await;
                }
            }
        }

        tracing::info!("Task queue worker pool stopped");
    }

    async fn claim_and_dispatch_one(
        repository: &TaskRepository,
        rate_limiter: &RateLimiter,
        semaphore: &Arc<Semaphore>,
        context: &Weak<dyn TaskHandlerContext>,
        capacity_gate: Option<&dyn CapacityGate>,
        task_finished_tx: Option<TaskFinishedSender>,
    ) {
        if let Some(gate) = capacity_gate {
            if !gate.can_accept_task().await {
                tracing::debug!("Capacity gate closed, skipping claim");
                return;
            }
        }

        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                tracing::debug!("No workers available, skipping claim");
                return;
            }
        };

        match repository.claim_next_task().await {
            Ok(Some(task)) => {
                let repo = repository.clone();
                let limiter = rate_limiter.clone();
                let ctx = context.clone();

                let finished_tx = task_finished_tx.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(e) =
                        Self::process_task_with_retry(task, repo, limiter, ctx, finished_tx).await
                    {
                        tracing::error!(error = %e, "Task processing failed after retries");
                    }
                });
            }
            Ok(None) => {
                drop(permit);
                tracing::trace!("No tasks available in queue");
            }
            Err(e) => {
                drop(permit);
                tracing::error!(error = %e, "Failed to claim task from queue");
            }
        }
    }

    #[tracing::instrument(skip(repository, rate_limiter, context, task_finished_tx), fields(task.id = %task.id, task.type = %task.task_type))]
    async fn process_task_with_retry(
        task: Task,
        repository: TaskRepository,
        rate_limiter: RateLimiter,
        context: Weak<dyn TaskHandlerContext>,
        task_finished_tx: Option<TaskFinishedSender>,
    ) -> Result<()> {
        if let Some(ref depends_on) = task.depends_on {
            let deps_completed = repository
                .check_dependencies_completed(depends_on)
                .await
                .context("Failed to check dependencies")?;

            if !deps_completed {
                if task.cancel_on_dep_failure {
                    let dep_failed = repository
                        .check_any_dependency_failed_or_cancelled(depends_on)
                        .await
                        .context("Failed to check dependency failure")?;
                    if dep_failed {
                        tracing::info!(
                            task_id = %task.id,
                            "Dependency failed or cancelled and cancel_on_dep_failure is set, cancelling task"
                        );
                        repository
                            .update_status(task.id, mindia_core::models::TaskStatus::Cancelled)
                            .await?;
                        if let Some(ref tx) = task_finished_tx {
                            let _ = tx
                                .send((task.id, mindia_core::models::TaskStatus::Cancelled))
                                .await;
                        }
                        return Ok(());
                    }
                }
                tracing::info!(task_id = %task.id, "Task dependencies not completed, rescheduling");
                repository
                    .update_status(task.id, mindia_core::models::TaskStatus::Pending)
                    .await?;
                return Ok(());
            }
        }

        rate_limiter.acquire(&task.task_type).await;

        let ctx = context.upgrade().ok_or_else(|| {
            anyhow::anyhow!("TaskHandlerContext was dropped, cannot process task")
        })?;

        let timeout_duration = task
            .timeout_seconds
            .map(|s| Duration::from_secs(s as u64))
            .unwrap_or(Duration::from_secs(3600));

        let result = tokio::time::timeout(timeout_duration, ctx.dispatch_task(&task)).await;

        match result {
            Ok(Ok(task_result)) => {
                repository
                    .mark_completed(task.id, task_result)
                    .await
                    .context("Failed to mark task as completed")?;
                if let Some(ref tx) = task_finished_tx {
                    let _ = tx
                        .send((task.id, mindia_core::models::TaskStatus::Completed))
                        .await;
                }
                tracing::info!(task_id = %task.id, task_type = %task.task_type, "Task completed successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                // Check if this is a TaskError with unrecoverable flag
                let is_unrecoverable = e
                    .downcast_ref::<TaskError>()
                    .map(|te| !te.is_recoverable())
                    .unwrap_or(false);

                tracing::error!(
                    task_id = %task.id,
                    error = %e,
                    retry_count = task.retry_count,
                    max_retries = task.max_retries,
                    unrecoverable = is_unrecoverable,
                    "Task execution failed"
                );

                // Don't retry if the error is explicitly marked as unrecoverable
                if is_unrecoverable {
                    let error_result = json!({
                        "error": e.to_string(),
                        "retry_count": task.retry_count,
                        "unrecoverable": true,
                        "reason": "Task failed with unrecoverable error (e.g., missing configuration, invalid credentials)"
                    });
                    repository
                        .mark_failed(task.id, error_result)
                        .await
                        .context("Failed to mark task as failed")?;
                    if let Some(ref tx) = task_finished_tx {
                        let _ = tx
                            .send((task.id, mindia_core::models::TaskStatus::Failed))
                            .await;
                    }
                    tracing::error!(
                        task_id = %task.id,
                        "Task failed with unrecoverable error, will not retry"
                    );
                    return Err(e);
                }

                // Retry if the error is recoverable and we haven't exceeded max retries
                if task.can_retry() {
                    let backoff_seconds = compute_retry_backoff_seconds(task.retry_count);
                    tracing::info!(
                        task_id = %task.id,
                        retry_count = task.retry_count + 1,
                        backoff_seconds = backoff_seconds,
                        "Scheduling task retry"
                    );
                    let retried_task = repository.increment_retry(task.id).await?;
                    repository
                        .update_status(retried_task.id, mindia_core::models::TaskStatus::Scheduled)
                        .await?;
                    Ok(())
                } else {
                    let error_result = json!({
                        "error": e.to_string(),
                        "retry_count": task.retry_count,
                        "reason": "Task failed after maximum retries"
                    });
                    repository
                        .mark_failed(task.id, error_result)
                        .await
                        .context("Failed to mark task as failed")?;
                    if let Some(ref tx) = task_finished_tx {
                        let _ = tx
                            .send((task.id, mindia_core::models::TaskStatus::Failed))
                            .await;
                    }
                    tracing::error!(task_id = %task.id, "Task failed after max retries");
                    Err(e)
                }
            }
            Err(_) => {
                let error_result = json!({
                    "error": "Task execution timed out",
                    "timeout_seconds": task.timeout_seconds,
                });
                tracing::error!(
                    task_id = %task.id,
                    timeout_seconds = ?task.timeout_seconds,
                    "Task execution timed out"
                );
                if task.can_retry() {
                    repository.increment_retry(task.id).await?;
                    Ok(())
                } else {
                    repository.mark_failed(task.id, error_result).await?;
                    if let Some(ref tx) = task_finished_tx {
                        let _ = tx
                            .send((task.id, mindia_core::models::TaskStatus::Failed))
                            .await;
                    }
                    Err(anyhow::anyhow!("Task execution timed out"))
                }
            }
        }
    }

    /// Signals the worker pool to stop claiming new tasks and exit the main loop.
    ///
    /// This method returns immediately after sending the signal; it does **not** wait for
    /// the worker pool to finish or for in-flight tasks to complete. Already-spawned task
    /// handlers continue running until they complete or time out. For graceful shutdown
    /// (e.g. waiting for in-flight work before process exit), coordinate with your
    /// runtime (e.g. tokio signal handler) and consider awaiting a "pool stopped" signal
    /// or giving in-flight tasks a bounded time to finish before terminating.
    pub async fn shutdown(&self) {
        tracing::info!("Initiating task queue shutdown");
        let _ = self.shutdown_tx.send(()).await;
    }
}

impl Clone for TaskQueue {
    fn clone(&self) -> Self {
        Self {
            repository: self.repository.clone(),
            rate_limiter: self.rate_limiter.clone(),
            config: self.config.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_backoff_exponential_then_capped() {
        assert_eq!(compute_retry_backoff_seconds(0), 1);
        assert_eq!(compute_retry_backoff_seconds(1), 2);
        assert_eq!(compute_retry_backoff_seconds(2), 4);
        assert_eq!(compute_retry_backoff_seconds(8), 256);
        assert_eq!(compute_retry_backoff_seconds(9), MAX_RETRY_BACKOFF_SECS);
        assert_eq!(compute_retry_backoff_seconds(10), MAX_RETRY_BACKOFF_SECS);
    }

    #[test]
    fn unrecoverable_task_error_detected() {
        let err: anyhow::Error =
            mindia_core::TaskError::unrecoverable(anyhow::anyhow!("bad config")).into();
        let is_unrecoverable = err
            .downcast_ref::<mindia_core::TaskError>()
            .map(|te| !te.is_recoverable())
            .unwrap_or(false);
        assert!(is_unrecoverable);
    }

    #[test]
    fn recoverable_task_error_detected() {
        let err: anyhow::Error =
            mindia_core::TaskError::recoverable(anyhow::anyhow!("network")).into();
        let is_unrecoverable = err
            .downcast_ref::<mindia_core::TaskError>()
            .map(|te| !te.is_recoverable())
            .unwrap_or(false);
        assert!(!is_unrecoverable);
    }

    #[test]
    fn non_task_error_treated_as_recoverable() {
        let err: anyhow::Error = anyhow::anyhow!("generic error");
        let is_unrecoverable = err
            .downcast_ref::<mindia_core::TaskError>()
            .map(|te| !te.is_recoverable())
            .unwrap_or(false);
        assert!(!is_unrecoverable);
    }
}
