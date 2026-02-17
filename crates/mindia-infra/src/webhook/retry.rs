use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::webhook::service::WebhookService;
use mindia_db::WebhookRetryRepository;

/// Configuration for webhook retry service
#[derive(Clone)]
pub struct WebhookRetryServiceConfig {
    pub poll_interval_seconds: u64,
    pub batch_size: i64,
    pub max_concurrent_retries: usize,
}

impl Default for WebhookRetryServiceConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: 30,  // Check for due retries every 30 seconds
            batch_size: 100,            // Process up to 100 retries per batch
            max_concurrent_retries: 10, // Process up to 10 retries concurrently
        }
    }
}

/// Background service that processes webhook retry queue
#[derive(Clone)]
#[allow(dead_code)] // Fields are used in Clone and may be used in future implementations
pub struct WebhookRetryService {
    retry_repo: WebhookRetryRepository,
    webhook_service: Arc<WebhookService>,
    config: WebhookRetryServiceConfig,
    shutdown_tx: mpsc::Sender<()>,
}

impl WebhookRetryService {
    pub fn new(
        retry_repo: WebhookRetryRepository,
        webhook_service: Arc<WebhookService>,
        config: WebhookRetryServiceConfig,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        // Spawn background worker
        let repo_clone = retry_repo.clone();
        let service_clone = webhook_service.clone();
        let config_clone = config.clone();

        tokio::spawn(async move {
            Self::worker_loop(repo_clone, service_clone, config_clone, shutdown_rx).await;
        });

        Self {
            retry_repo,
            webhook_service,
            config,
            shutdown_tx,
        }
    }

    /// Main worker loop that processes retry queue
    async fn worker_loop(
        retry_repo: WebhookRetryRepository,
        webhook_service: Arc<WebhookService>,
        config: WebhookRetryServiceConfig,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) {
        let mut poll_interval = interval(Duration::from_secs(config.poll_interval_seconds));

        tracing::info!(
            poll_interval_seconds = config.poll_interval_seconds,
            batch_size = config.batch_size,
            "Webhook retry service started"
        );

        loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    if let Err(e) = Self::process_retry_batch(
                        &retry_repo,
                        &webhook_service,
                        &config,
                    ).await {
                        tracing::error!(
                            error = %e,
                            "Error processing retry batch"
                        );
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Webhook retry service shutting down");
                    break;
                }
            }
        }
    }

    /// Process a batch of due retries
    async fn process_retry_batch(
        retry_repo: &WebhookRetryRepository,
        webhook_service: &WebhookService,
        config: &WebhookRetryServiceConfig,
    ) -> Result<()> {
        // Get due retries
        let due_retries = retry_repo
            .get_due_retries(config.batch_size)
            .await
            .context("Failed to get due retries")?;

        if due_retries.is_empty() {
            return Ok(());
        }

        tracing::info!(
            retry_count = due_retries.len(),
            "Processing webhook retries"
        );

        // Process retries with concurrency limit
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrent_retries));
        let mut handles = vec![];

        for retry_item in due_retries {
            let webhook_service = webhook_service.clone();
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("semaphore closed");

            let handle = tokio::spawn(async move {
                let event_id = retry_item.webhook_event_id;
                let result = webhook_service.process_retry(event_id).await;

                drop(permit); // Release semaphore permit

                match result {
                    Ok(success) => {
                        if success {
                            tracing::debug!(
                                event_id = %event_id,
                                "Retry processed successfully"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            event_id = %event_id,
                            error = %e,
                            "Failed to process retry"
                        );
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all retries to complete
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Gracefully shutdown the retry service
    pub async fn shutdown(&self) {
        if let Err(e) = self.shutdown_tx.send(()).await {
            tracing::warn!(
                error = %e,
                "Failed to send shutdown signal to webhook retry service"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = WebhookRetryServiceConfig::default();
        assert_eq!(config.poll_interval_seconds, 30);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_concurrent_retries, 10);
    }
}
