use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

use mindia_core::models::ProcessingStatus;
use mindia_processing::{VideoOrchestrator, VideoOrchestratorConfig};

use crate::state::AppState;
use crate::video_storage_impl::GenericVideoStorage;

#[derive(Debug, Clone)]
pub enum VideoJob {
    #[allow(dead_code)]
    TranscodeVideo { video_id: Uuid },
}

pub struct VideoJobQueue {
    tx: mpsc::Sender<VideoJob>,
}

impl VideoJobQueue {
    /// Create a new video job queue with bounded channel
    ///
    /// # Arguments
    /// * `state` - Application state
    /// * `max_concurrent` - Maximum number of concurrent jobs
    ///
    /// The channel bound is configurable via `VIDEO_JOB_QUEUE_SIZE` environment variable
    /// (default: 1000). If the queue is full, `submit()` will return an error.
    pub fn new(state: Arc<AppState>, max_concurrent: usize) -> Self {
        // Allow queue size to be configured via environment variable (default: 1000)
        let queue_size = std::env::var("VIDEO_JOB_QUEUE_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1000)
            .max(1); // Ensure at least 1

        let (tx, rx) = mpsc::channel(queue_size);

        // Spawn worker pool
        tokio::spawn(async move {
            Self::worker_pool(rx, state, max_concurrent).await;
        });

        tracing::info!(
            queue_size = queue_size,
            max_concurrent = max_concurrent,
            "Video job queue initialized with bounded channel"
        );

        Self { tx }
    }

    /// No-op queue used when video is enabled but no job queue is configured.
    pub fn dummy() -> Self {
        let (tx, _rx) = mpsc::channel(1);
        Self { tx }
    }

    #[tracing::instrument(skip(self), fields(job.type = "transcode"))]
    #[allow(dead_code)]
    pub fn submit(&self, job: VideoJob) -> Result<()> {
        match &job {
            VideoJob::TranscodeVideo { video_id } => {
                tracing::info!(video_id = %video_id, "Enqueuing video transcode job");
            }
        }
        // Try to send, but handle the case where the queue is full
        self.tx.try_send(job).map_err(|e| match &e {
            tokio::sync::mpsc::error::TrySendError::Full(_) => {
                tracing::warn!("Video job queue is full, rejecting job");
                anyhow::anyhow!("Video job queue is full, please try again later")
            }
            _ => anyhow::anyhow!("Failed to submit video job: {}", e),
        })?;
        Ok(())
    }

    /// Submit a job asynchronously, waiting if the queue is full
    ///
    /// This method will block if the queue is full, unlike `submit()` which returns an error.
    /// Use this when you want to ensure the job is queued even if it means waiting.
    #[tracing::instrument(skip(self), fields(job.type = "transcode"))]
    #[allow(dead_code)]
    pub async fn submit_async(&self, job: VideoJob) -> Result<()> {
        match &job {
            VideoJob::TranscodeVideo { video_id } => {
                tracing::info!(video_id = %video_id, "Enqueuing video transcode job (async)");
            }
        }
        self.tx
            .send(job)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to submit video job: {}", e))?;
        Ok(())
    }

    async fn worker_pool(
        mut rx: mpsc::Receiver<VideoJob>,
        state: Arc<AppState>,
        max_concurrent: usize,
    ) {
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        while let Some(job) = rx.recv().await {
            let permit = semaphore.clone().acquire_owned().await;
            let state = state.clone();

            tokio::spawn(async move {
                let _permit = permit;
                if let Err(e) = Self::process_job(job, state).await {
                    tracing::error!(error = %e, "Job processing failed");
                }
            });
        }
    }

    #[tracing::instrument(skip(state), fields(job.type = "transcode"))]
    async fn process_job(job: VideoJob, state: Arc<AppState>) -> Result<()> {
        match job {
            VideoJob::TranscodeVideo { video_id } => {
                Self::process_transcode_job(video_id, state).await
            }
        }
    }

    #[tracing::instrument(skip(state), fields(video.id = %video_id, job.status = tracing::field::Empty))]
    async fn process_transcode_job(video_id: Uuid, state: Arc<AppState>) -> Result<()> {
        let start = std::time::Instant::now();
        tracing::info!(video_id = %video_id, "Starting video transcode job");

        let video: mindia_core::models::Video = state
            .media.repository
            .get_video_by_id_unchecked(video_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .ok_or_else(|| anyhow::anyhow!("Video not found"))?;

        if let Err(e) = state
            .media.repository
            .update_video_processing_status(video.tenant_id, video_id, ProcessingStatus::Processing)
            .await
        {
            tracing::error!(video_id = %video_id, error = %e, "Failed to update status to processing");
            return Err(e.into());
        }

        let storage = Arc::new(GenericVideoStorage::new(state.media.storage.clone()));
        let config = VideoOrchestratorConfig {
            ffmpeg_path: state.media.ffmpeg_path.clone(),
            hls_segment_duration: state.media.hls_segment_duration,
            hls_variants: state.media.hls_variants.clone(),
            capacity_monitor_interval_secs: state.config.capacity_monitor_interval_secs(),
        };
        let orchestrator = VideoOrchestrator::new(
            state.media.repository.clone(),
            storage,
            state.capacity_checker.clone(),
            config,
        );
        let result = orchestrator.process_video(video_id).await;

        let elapsed = start.elapsed();

        match result {
            Ok(_) => {
                tracing::Span::current().record("job.status", "success");
                tracing::info!(
                    video_id = %video_id,
                    duration_ms = elapsed.as_millis(),
                    duration_secs = elapsed.as_secs_f64(),
                    "Video transcode completed successfully"
                );
                Ok(())
            }
            Err(e) => {
                tracing::Span::current().record("job.status", "failed");
                tracing::error!(
                    video_id = %video_id,
                    error = %e,
                    duration_ms = elapsed.as_millis(),
                    "Video transcode failed"
                );

                // Update status to failed
                if let Err(update_err) = state
                    .media.repository
                    .update_video_processing_status(
                        video.tenant_id,
                        video_id,
                        ProcessingStatus::Failed,
                    )
                    .await
                {
                    tracing::error!(
                        video_id = %video_id,
                        error = %update_err,
                        "Failed to update status to failed"
                    );
                }

                Err(e)
            }
        }
    }
}

impl Clone for VideoJobQueue {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}
