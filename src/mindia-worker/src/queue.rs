//! Task queue: worker pool, LISTEN/NOTIFY or polling, retry, and submission.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::sleep;
use uuid::Uuid;

use mindia_core::models::{Priority, Task, TaskType};
use mindia_core::TaskError;
use mindia_db::TaskRepository;
use mindia_infra::RateLimiter;

use crate::context::TaskHandlerContext;

/// Channel name for PostgreSQL LISTEN/NOTIFY when a new task is created.
pub const TASK_NOTIFY_CHANNEL: &str = "mindia_new_task";

#[derive(Clone)]
pub struct TaskQueueConfig {
    pub max_workers: usize,
    pub poll_interval_ms: u64,
    pub default_timeout_seconds: i32,
    pub max_retries: i32,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            max_workers: 4,
            poll_interval_ms: 1000,
            default_timeout_seconds: 3600,
            max_retries: 3,
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
    pub fn new(
        repository: TaskRepository,
        rate_limiter: RateLimiter,
        config: TaskQueueConfig,
        context: Weak<dyn TaskHandlerContext>,
        pool: Option<sqlx::PgPool>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let repo_clone = repository.clone();
        let limiter_clone = rate_limiter.clone();
        let config_clone = TaskQueueConfig {
            max_workers: config.max_workers,
            poll_interval_ms: config.poll_interval_ms,
            default_timeout_seconds: config.default_timeout_seconds,
            max_retries: config.max_retries,
        };

        tokio::spawn(async move {
            Self::worker_pool(
                repo_clone,
                limiter_clone,
                config_clone,
                context,
                shutdown_rx,
                pool,
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
    pub async fn submit_task(
        &self,
        tenant_id: Uuid,
        task_type: TaskType,
        payload: serde_json::Value,
        priority: Priority,
        scheduled_at: Option<DateTime<Utc>>,
        depends_on: Option<Vec<Uuid>>,
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

    async fn worker_pool(
        repository: TaskRepository,
        rate_limiter: RateLimiter,
        config: TaskQueueConfig,
        context: Weak<dyn TaskHandlerContext>,
        mut shutdown_rx: mpsc::Receiver<()>,
        pool: Option<sqlx::PgPool>,
    ) {
        let use_listen = pool.is_some();
        tracing::info!(
            max_workers = config.max_workers,
            poll_interval_ms = config.poll_interval_ms,
            listen_notify = use_listen,
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

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("Task queue worker pool shutting down");
                    break;
                }
                _ = notify_rx.recv() => {
                    // Woken by LISTEN/NOTIFY; try to claim one task immediately.
                    Self::claim_and_dispatch_one(
                        &repository,
                        &rate_limiter,
                        &semaphore,
                        &context,
                    ).await;
                }
                _ = sleep(poll_interval) => {
                    Self::claim_and_dispatch_one(
                        &repository,
                        &rate_limiter,
                        &semaphore,
                        &context,
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
    ) {
        match repository.claim_next_task().await {
            Ok(Some(task)) => {
                let permit = match semaphore.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        tracing::debug!("No workers available, task will be retried");
                        let _ = repository
                            .update_status(task.id, mindia_core::models::TaskStatus::Pending)
                            .await;
                        return;
                    }
                };

                let repo = repository.clone();
                let limiter = rate_limiter.clone();
                let ctx = context.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(e) = Self::process_task_with_retry(task, repo, limiter, ctx).await {
                        tracing::error!(error = %e, "Task processing failed after retries");
                    }
                });
            }
            Ok(None) => {
                tracing::trace!("No tasks available in queue");
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to claim task from queue");
            }
        }
    }

    #[tracing::instrument(skip(repository, rate_limiter, context), fields(task.id = %task.id, task.type = %task.task_type))]
    async fn process_task_with_retry(
        task: Task,
        repository: TaskRepository,
        rate_limiter: RateLimiter,
        context: Weak<dyn TaskHandlerContext>,
    ) -> Result<()> {
        if let Some(ref depends_on) = task.depends_on {
            let deps_completed = repository
                .check_dependencies_completed(depends_on)
                .await
                .context("Failed to check dependencies")?;

            if !deps_completed {
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
                    tracing::error!(
                        task_id = %task.id,
                        "Task failed with unrecoverable error, will not retry"
                    );
                    return Err(e);
                }

                // Retry if the error is recoverable and we haven't exceeded max retries
                if task.can_retry() {
                    let backoff_seconds = 2_u64.pow(task.retry_count as u32);
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
                    Err(anyhow::anyhow!("Task execution timed out"))
                }
            }
        }
    }

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
            config: TaskQueueConfig {
                max_workers: self.config.max_workers,
                poll_interval_ms: self.config.poll_interval_ms,
                default_timeout_seconds: self.config.default_timeout_seconds,
                max_retries: self.config.max_retries,
            },
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}
