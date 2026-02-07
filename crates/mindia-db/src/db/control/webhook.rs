use chrono::{DateTime, Duration, Utc};
use mindia_core::models::{
    Webhook, WebhookDeliveryStatus, WebhookEventLog, WebhookEventType, WebhookRetryQueueItem,
};
use mindia_core::AppError;
use sqlx::types::JsonValue;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

/// Repository for managing webhook configurations
#[derive(Clone)]
pub struct WebhookRepository {
    pool: PgPool,
}

impl WebhookRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new webhook configuration
    #[tracing::instrument(skip(self), fields(db.table = "webhooks", db.operation = "insert"))]
    pub async fn create(
        &self,
        tenant_id: Uuid,
        url: String,
        event_type: WebhookEventType,
        signing_secret: Option<String>,
        description: Option<String>,
    ) -> Result<Webhook, AppError> {
        let webhook = sqlx::query_as::<Postgres, Webhook>(
            r#"
            INSERT INTO webhooks (tenant_id, url, event_type, signing_secret, description, is_active)
            VALUES ($1, $2, $3, $4, $5, true)
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(&url)
        .bind(&event_type)
        .bind(&signing_secret)
        .bind(&description)
        .fetch_one(&self.pool)
        .await?;

        Ok(webhook)
    }

    /// Get webhook by ID
    #[tracing::instrument(skip(self), fields(db.table = "webhooks", db.operation = "select", db.record_id = %id))]
    pub async fn get_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Webhook>, AppError> {
        let webhook = sqlx::query_as::<Postgres, Webhook>(
            "SELECT * FROM webhooks WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(webhook)
    }

    /// List all webhooks for a tenant
    #[tracing::instrument(skip(self), fields(db.table = "webhooks", db.operation = "select"))]
    pub async fn list_by_tenant(&self, tenant_id: Uuid) -> Result<Vec<Webhook>, AppError> {
        let webhooks = sqlx::query_as::<Postgres, Webhook>(
            "SELECT * FROM webhooks WHERE tenant_id = $1 ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(webhooks)
    }

    /// Find active webhooks by tenant and event type
    #[tracing::instrument(skip(self), fields(db.table = "webhooks", db.operation = "select"))]
    pub async fn find_active_by_event(
        &self,
        tenant_id: Uuid,
        event_type: WebhookEventType,
    ) -> Result<Vec<Webhook>, AppError> {
        let webhooks = sqlx::query_as::<Postgres, Webhook>(
            r#"
            SELECT * FROM webhooks 
            WHERE tenant_id = $1 AND event_type = $2 AND is_active = true
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(&event_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(webhooks)
    }

    /// Update webhook configuration
    #[tracing::instrument(skip(self), fields(db.table = "webhooks", db.operation = "update", db.record_id = %id))]
    pub async fn update(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        url: Option<String>,
        signing_secret: Option<String>,
        is_active: Option<bool>,
        description: Option<String>,
    ) -> Result<Webhook, AppError> {
        let webhook = sqlx::query_as::<Postgres, Webhook>(
            r#"
            UPDATE webhooks 
            SET url = COALESCE($3, url),
                signing_secret = COALESCE($4, signing_secret),
                is_active = COALESCE($5, is_active),
                description = COALESCE($6, description),
                updated_at = NOW()
            WHERE tenant_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .bind(url)
        .bind(signing_secret)
        .bind(is_active)
        .bind(description)
        .fetch_one(&self.pool)
        .await?;

        Ok(webhook)
    }

    /// Deactivate webhook (called after max retries exceeded)
    #[tracing::instrument(skip(self), fields(db.table = "webhooks", db.operation = "update", db.record_id = %id))]
    pub async fn deactivate(&self, id: Uuid, reason: String) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE webhooks 
            SET is_active = false,
                deactivated_at = NOW(),
                deactivation_reason = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(&reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete webhook
    #[tracing::instrument(skip(self), fields(db.table = "webhooks", db.operation = "delete", db.record_id = %id))]
    pub async fn delete(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM webhooks WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// Repository for managing webhook event logs
#[derive(Clone)]
pub struct WebhookEventRepository {
    pool: PgPool,
}

impl WebhookEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new webhook event log
    #[tracing::instrument(skip(self, payload), fields(db.table = "webhook_events", db.operation = "insert"))]
    pub async fn create(
        &self,
        webhook_id: Uuid,
        tenant_id: Uuid,
        event_type: WebhookEventType,
        payload: JsonValue,
    ) -> Result<WebhookEventLog, AppError> {
        let event = sqlx::query_as::<Postgres, WebhookEventLog>(
            r#"
            INSERT INTO webhook_events (webhook_id, tenant_id, event_type, payload, status, retry_count)
            VALUES ($1, $2, $3, $4, 'pending', 0)
            RETURNING *
            "#,
        )
        .bind(webhook_id)
        .bind(tenant_id)
        .bind(&event_type)
        .bind(&payload)
        .fetch_one(&self.pool)
        .await?;

        Ok(event)
    }

    /// Update event log after delivery attempt
    #[tracing::instrument(skip(self), fields(db.table = "webhook_events", db.operation = "update", db.record_id = %id))]
    pub async fn update_delivery_status(
        &self,
        id: Uuid,
        status: WebhookDeliveryStatus,
        response_status_code: Option<i32>,
        response_body: Option<String>,
        error_message: Option<String>,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE webhook_events 
            SET status = $2,
                response_status_code = $3,
                response_body = $4,
                error_message = $5,
                sent_at = COALESCE(sent_at, $6),
                completed_at = CASE WHEN $2 IN ('success', 'failed') THEN $6 ELSE completed_at END
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(&status)
        .bind(response_status_code)
        .bind(response_body)
        .bind(error_message)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Increment retry count
    #[tracing::instrument(skip(self), fields(db.table = "webhook_events", db.operation = "update", db.record_id = %id))]
    pub async fn increment_retry_count(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE webhook_events SET retry_count = retry_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get event log by ID
    #[tracing::instrument(skip(self), fields(db.table = "webhook_events", db.operation = "select", db.record_id = %id))]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<WebhookEventLog>, AppError> {
        let event = sqlx::query_as::<Postgres, WebhookEventLog>(
            "SELECT * FROM webhook_events WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(event)
    }

    /// List event logs for a webhook
    #[tracing::instrument(skip(self), fields(db.table = "webhook_events", db.operation = "select"))]
    pub async fn list_by_webhook(
        &self,
        webhook_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WebhookEventLog>, AppError> {
        let events = sqlx::query_as::<Postgres, WebhookEventLog>(
            r#"
            SELECT * FROM webhook_events 
            WHERE webhook_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(webhook_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// List event logs for a tenant
    #[tracing::instrument(skip(self), fields(db.table = "webhook_events", db.operation = "select"))]
    pub async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WebhookEventLog>, AppError> {
        let events = sqlx::query_as::<Postgres, WebhookEventLog>(
            r#"
            SELECT * FROM webhook_events 
            WHERE tenant_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }
}

/// Repository for managing webhook retry queue
#[derive(Clone)]
pub struct WebhookRetryRepository {
    pool: PgPool,
}

impl WebhookRetryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Add a failed webhook to the retry queue
    #[tracing::instrument(skip(self), fields(db.table = "webhook_retry_queue", db.operation = "insert"))]
    pub async fn enqueue(
        &self,
        webhook_event_id: Uuid,
        webhook_id: Uuid,
        tenant_id: Uuid,
        retry_count: i32,
        next_retry_at: DateTime<Utc>,
        last_error: Option<String>,
    ) -> Result<WebhookRetryQueueItem, AppError> {
        let item = sqlx::query_as::<Postgres, WebhookRetryQueueItem>(
            r#"
            INSERT INTO webhook_retry_queue 
            (webhook_event_id, webhook_id, tenant_id, retry_count, next_retry_at, last_error, last_attempt_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (webhook_event_id) 
            DO UPDATE SET 
                retry_count = $4,
                next_retry_at = $5,
                last_error = $6,
                last_attempt_at = NOW(),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(webhook_event_id)
        .bind(webhook_id)
        .bind(tenant_id)
        .bind(retry_count)
        .bind(next_retry_at)
        .bind(last_error)
        .fetch_one(&self.pool)
        .await?;

        Ok(item)
    }

    /// Get items due for retry
    /// Uses FOR UPDATE SKIP LOCKED to prevent multiple instances from processing the same retry
    #[tracing::instrument(skip(self), fields(db.table = "webhook_retry_queue", db.operation = "select"))]
    pub async fn get_due_retries(
        &self,
        limit: i64,
    ) -> Result<Vec<WebhookRetryQueueItem>, AppError> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query_as::<Postgres, WebhookRetryQueueItem>(
            r#"
            SELECT * FROM webhook_retry_queue 
            WHERE next_retry_at <= $1 AND retry_count < max_retries
            ORDER BY next_retry_at ASC 
            LIMIT $2
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await;

        match result {
            Ok(items) => {
                tx.commit().await?;
                Ok(items)
            }
            Err(e) => {
                tx.rollback().await.ok(); // Ignore rollback errors
                Err(anyhow::anyhow!("Failed to fetch webhook retry queue items: {}", e).into())
            }
        }
    }

    /// Remove item from retry queue (after successful delivery or max retries)
    #[tracing::instrument(skip(self), fields(db.table = "webhook_retry_queue", db.operation = "delete"))]
    pub async fn dequeue(&self, webhook_event_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM webhook_retry_queue WHERE webhook_event_id = $1")
            .bind(webhook_event_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update retry item after attempt
    #[tracing::instrument(skip(self), fields(db.table = "webhook_retry_queue", db.operation = "update"))]
    pub async fn update_after_attempt(
        &self,
        webhook_event_id: Uuid,
        retry_count: i32,
        next_retry_at: DateTime<Utc>,
        last_error: Option<String>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE webhook_retry_queue 
            SET retry_count = $2,
                next_retry_at = $3,
                last_error = $4,
                last_attempt_at = NOW(),
                updated_at = NOW()
            WHERE webhook_event_id = $1
            "#,
        )
        .bind(webhook_event_id)
        .bind(retry_count)
        .bind(next_retry_at)
        .bind(last_error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Helper function to calculate next retry time based on exponential backoff retry schedule
pub fn calculate_next_retry_time(retry_count: i32) -> Duration {
    match retry_count {
        0 => Duration::minutes(1),  // Retry 1: 1 minute
        1 => Duration::minutes(5),  // Retry 2: 5 minutes
        2 => Duration::minutes(10), // Retry 3: 10 minutes
        3 => Duration::minutes(30), // Retry 4: 30 minutes
        4 => Duration::minutes(60), // Retry 5: 60 minutes
        _ => Duration::hours(1),    // Retry 6+: 1 hour
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_next_retry_time_first_retry() {
        let d = calculate_next_retry_time(0);
        assert_eq!(d.num_minutes(), 1);
    }

    #[test]
    fn test_calculate_next_retry_time_second_retry() {
        let d = calculate_next_retry_time(1);
        assert_eq!(d.num_minutes(), 5);
    }

    #[test]
    fn test_calculate_next_retry_time_third_retry() {
        let d = calculate_next_retry_time(2);
        assert_eq!(d.num_minutes(), 10);
    }

    #[test]
    fn test_calculate_next_retry_time_fourth_retry() {
        let d = calculate_next_retry_time(3);
        assert_eq!(d.num_minutes(), 30);
    }

    #[test]
    fn test_calculate_next_retry_time_fifth_retry() {
        let d = calculate_next_retry_time(4);
        assert_eq!(d.num_minutes(), 60);
    }

    #[test]
    fn test_calculate_next_retry_time_sixth_and_beyond() {
        assert_eq!(calculate_next_retry_time(5).num_hours(), 1);
        assert_eq!(calculate_next_retry_time(10).num_hours(), 1);
        assert_eq!(calculate_next_retry_time(72).num_hours(), 1);
    }
}
