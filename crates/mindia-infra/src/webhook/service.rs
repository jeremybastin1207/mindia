use anyhow::{Context, Result};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use mindia_core::models::{
    WebhookDataInfo, WebhookDeliveryStatus, WebhookEventType, WebhookHookInfo,
    WebhookInitiatorInfo, WebhookPayload,
};
use mindia_db::{
    calculate_next_retry_time, WebhookEventRepository, WebhookRepository, WebhookRetryRepository,
};

type HmacSha256 = Hmac<Sha256>;

/// Configuration for webhook service
#[derive(Clone)]
pub struct WebhookServiceConfig {
    pub timeout_seconds: u64,
    pub max_retries: i32,
    pub max_concurrent_deliveries: usize,
}

impl Default for WebhookServiceConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 72, // ~72 hour retry window
            max_concurrent_deliveries: 50,
        }
    }
}

/// Service for managing webhook delivery
#[derive(Clone)]
pub struct WebhookService {
    webhook_repo: WebhookRepository,
    event_repo: WebhookEventRepository,
    retry_repo: WebhookRetryRepository,
    http_client: Client,
    config: WebhookServiceConfig,
}

impl WebhookService {
    pub fn new(
        webhook_repo: WebhookRepository,
        event_repo: WebhookEventRepository,
        retry_repo: WebhookRetryRepository,
        config: WebhookServiceConfig,
    ) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .context("Failed to create HTTP client for webhooks")?;

        Ok(Self {
            webhook_repo,
            event_repo,
            retry_repo,
            http_client,
            config,
        })
    }

    /// Trigger a webhook event
    /// This is the main entry point when an event occurs in the system
    #[tracing::instrument(skip(self, data))]
    pub async fn trigger_event(
        &self,
        tenant_id: Uuid,
        event_type: WebhookEventType,
        data: WebhookDataInfo,
        initiator: WebhookInitiatorInfo,
    ) -> Result<()> {
        // Find all active webhooks for this tenant and event type
        let webhooks = self
            .webhook_repo
            .find_active_by_event(tenant_id, event_type.clone())
            .await
            .context("Failed to find active webhooks")?;

        if webhooks.is_empty() {
            tracing::debug!(
                tenant_id = %tenant_id,
                event_type = %event_type,
                "No active webhooks found for event"
            );
            return Ok(());
        }

        tracing::info!(
            tenant_id = %tenant_id,
            event_type = %event_type,
            webhook_count = webhooks.len(),
            "Triggering webhooks for event"
        );

        // Use semaphore to limit concurrent webhook deliveries
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_deliveries,
        ));
        let mut handles = Vec::new();

        // Send webhook to each endpoint
        for webhook in webhooks {
            let payload = WebhookPayload {
                hook: WebhookHookInfo {
                    id: webhook.id,
                    event: event_type.to_string(),
                    target: webhook.url.clone(),
                    project: tenant_id,
                    created_at: webhook.created_at,
                },
                data: data.clone(),
                initiator: initiator.clone(),
            };

            // Create event log
            let payload_json =
                serde_json::to_value(&payload).context("Failed to serialize webhook payload")?;

            let event_log = self
                .event_repo
                .create(webhook.id, tenant_id, event_type.clone(), payload_json)
                .await
                .context("Failed to create webhook event log")?;

            // Send webhook asynchronously with concurrency limit (don't block on delivery)
            let service = self.clone();
            let webhook_id = webhook.id;
            let event_id = event_log.id;
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .context("Failed to acquire semaphore permit")?;

            let handle = tokio::spawn(async move {
                let result = service
                    .send_webhook_internal(
                        webhook_id,
                        event_id,
                        webhook.url,
                        webhook.signing_secret,
                        payload,
                    )
                    .await;

                drop(permit); // Release semaphore permit

                if let Err(e) = result {
                    tracing::error!(
                        webhook_id = %webhook_id,
                        event_id = %event_id,
                        error = %e,
                        "Failed to send webhook"
                    );
                }
            });

            handles.push(handle);
        }

        // Webhooks are fire-and-forget; semaphore enforces concurrency limit

        Ok(())
    }

    /// Internal method to send a webhook with retry logic
    #[tracing::instrument(skip(self, payload))]
    async fn send_webhook_internal(
        &self,
        webhook_id: Uuid,
        event_id: Uuid,
        url: String,
        signing_secret: Option<String>,
        payload: WebhookPayload,
    ) -> Result<()> {
        match self.send_webhook(&url, &signing_secret, &payload).await {
            Ok(response) => {
                // Success - update event log
                self.event_repo
                    .update_delivery_status(
                        event_id,
                        WebhookDeliveryStatus::Success,
                        Some(response.status_code),
                        Some(response.body),
                        None,
                    )
                    .await
                    .context("Failed to update event log")?;

                tracing::info!(
                    webhook_id = %webhook_id,
                    event_id = %event_id,
                    status_code = response.status_code,
                    "Webhook delivered successfully"
                );

                Ok(())
            }
            Err(e) => {
                // Failure - schedule retry
                let error_msg = e.to_string();

                self.event_repo
                    .update_delivery_status(
                        event_id,
                        WebhookDeliveryStatus::Failed,
                        None,
                        None,
                        Some(error_msg.clone()),
                    )
                    .await
                    .context("Failed to update event log")?;

                // Get webhook event to check tenant_id
                let event = self
                    .event_repo
                    .get_by_id(event_id)
                    .await?
                    .context("Event not found after creation")?;

                // Schedule retry
                let next_retry_at = Utc::now() + calculate_next_retry_time(0);
                self.retry_repo
                    .enqueue(
                        event_id,
                        webhook_id,
                        event.tenant_id,
                        0,
                        next_retry_at,
                        Some(error_msg.clone()),
                    )
                    .await
                    .context("Failed to enqueue retry")?;

                tracing::warn!(
                    webhook_id = %webhook_id,
                    event_id = %event_id,
                    error = %error_msg,
                    next_retry = %next_retry_at,
                    "Webhook delivery failed, scheduled for retry"
                );

                Ok(())
            }
        }
    }

    /// Send a webhook HTTP request
    #[tracing::instrument(skip(self, signing_secret, payload))]
    async fn send_webhook(
        &self,
        url: &str,
        signing_secret: &Option<String>,
        payload: &WebhookPayload,
    ) -> Result<WebhookResponse> {
        // Validate URL for SSRF before sending (reject private/internal hosts)
        super::ssrf::validate_url_for_ssrf(url, false, None)
            .await
            .map_err(|e| anyhow::anyhow!("Invalid webhook URL: {}", e))?;

        let body = serde_json::to_string(payload).context("Failed to serialize webhook payload")?;

        let mut request = self
            .http_client
            .post(url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mindia-Webhook/1.0");

        // Add signature if secret is provided
        if let Some(secret) = signing_secret {
            let signature = self.sign_payload(&body, secret)?;
            request = request.header("X-Webhook-Signature", format!("v1={}", signature));
        }

        let response = request
            .body(body)
            .send()
            .await
            .context("Failed to send webhook request")?;

        let status_code = response.status().as_u16() as i32;
        let response_body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read response body"));

        // Consider 2xx as success
        if (200..300).contains(&status_code) {
            Ok(WebhookResponse {
                status_code,
                body: response_body,
            })
        } else {
            Err(anyhow::anyhow!(
                "Webhook returned non-2xx status: {} - {}",
                status_code,
                response_body
            ))
        }
    }

    /// Sign webhook payload with HMAC-SHA256
    fn sign_payload(&self, body: &str, secret: &str) -> Result<String> {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).context("Invalid signing secret")?;

        mac.update(body.as_bytes());

        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());

        Ok(signature)
    }

    /// Verify webhook signature (for testing or validating received webhooks)
    pub fn verify_signature(&self, body: &str, secret: &str, signature: &str) -> Result<bool> {
        let expected_signature = self.sign_payload(body, secret)?;
        Ok(expected_signature == signature)
    }

    /// Process a single retry from the queue
    #[tracing::instrument(skip(self))]
    pub async fn process_retry(&self, event_id: Uuid) -> Result<bool> {
        // Get event log
        let event = self
            .event_repo
            .get_by_id(event_id)
            .await?
            .context("Event not found")?;

        // Get webhook config
        let webhook = self
            .webhook_repo
            .get_by_id(event.tenant_id, event.webhook_id)
            .await?
            .context("Webhook not found")?;

        // Check if webhook is still active
        if !webhook.is_active {
            tracing::warn!(
                webhook_id = %webhook.id,
                event_id = %event_id,
                "Skipping retry for inactive webhook"
            );
            self.retry_repo.dequeue(event_id).await?;
            return Ok(false);
        }

        // Parse payload
        let payload: WebhookPayload =
            serde_json::from_value(event.payload).context("Failed to parse webhook payload")?;

        // Increment retry count
        self.event_repo.increment_retry_count(event_id).await?;

        // Update status to retrying
        self.event_repo
            .update_delivery_status(event_id, WebhookDeliveryStatus::Retrying, None, None, None)
            .await?;

        // Attempt to send webhook
        match self
            .send_webhook(&webhook.url, &webhook.signing_secret, &payload)
            .await
        {
            Ok(response) => {
                // Success - remove from retry queue
                self.event_repo
                    .update_delivery_status(
                        event_id,
                        WebhookDeliveryStatus::Success,
                        Some(response.status_code),
                        Some(response.body),
                        None,
                    )
                    .await?;

                self.retry_repo.dequeue(event_id).await?;

                tracing::info!(
                    webhook_id = %webhook.id,
                    event_id = %event_id,
                    retry_count = event.retry_count + 1,
                    "Webhook retry succeeded"
                );

                Ok(true)
            }
            Err(e) => {
                let error_msg = e.to_string();
                let new_retry_count = event.retry_count + 1;

                // Check if we've exceeded max retries
                if new_retry_count >= self.config.max_retries {
                    // Max retries exceeded - deactivate webhook
                    self.webhook_repo
                        .deactivate(
                            webhook.id,
                            format!("Max retries ({}) exceeded", self.config.max_retries),
                        )
                        .await?;

                    self.event_repo
                        .update_delivery_status(
                            event_id,
                            WebhookDeliveryStatus::Failed,
                            None,
                            None,
                            Some(format!("Max retries exceeded: {}", error_msg)),
                        )
                        .await?;

                    self.retry_repo.dequeue(event_id).await?;

                    tracing::error!(
                        webhook_id = %webhook.id,
                        event_id = %event_id,
                        retry_count = new_retry_count,
                        "Max retries exceeded, webhook deactivated"
                    );

                    Ok(false)
                } else {
                    // Schedule next retry
                    let next_retry_at = Utc::now() + calculate_next_retry_time(new_retry_count);

                    self.retry_repo
                        .update_after_attempt(
                            event_id,
                            new_retry_count,
                            next_retry_at,
                            Some(error_msg.clone()),
                        )
                        .await?;

                    self.event_repo
                        .update_delivery_status(
                            event_id,
                            WebhookDeliveryStatus::Failed,
                            None,
                            None,
                            Some(error_msg.clone()),
                        )
                        .await?;

                    tracing::warn!(
                        webhook_id = %webhook.id,
                        event_id = %event_id,
                        retry_count = new_retry_count,
                        next_retry = %next_retry_at,
                        error = %error_msg,
                        "Webhook retry failed, rescheduled"
                    );

                    Ok(false)
                }
            }
        }
    }
}

struct WebhookResponse {
    status_code: i32,
    body: String,
}
