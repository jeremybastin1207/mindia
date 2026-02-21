use mindia_core::models::{
    AnalyticsQuery, AuditLogListResponse, AuditLogQuery, ContentTypeStats, RequestLog,
    RequestLogInput, StorageSummary, TrafficSummary, UrlStatistics,
};
use mindia_db::{AnalyticsRepositoryTrait, StorageMetricsRepository};
#[cfg(feature = "observability-opentelemetry")]
use opentelemetry::{
    metrics::{Counter, Gauge},
    KeyValue,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing;
use uuid::Uuid;

#[derive(Clone)]
pub struct AnalyticsService {
    repo: Arc<dyn AnalyticsRepositoryTrait + Send + Sync>,
    storage_repo: StorageMetricsRepository,
    log_sender: mpsc::Sender<RequestLogInput>,
    #[cfg(feature = "observability-opentelemetry")]
    upload_counter: Counter<u64>,
    #[cfg(feature = "observability-opentelemetry")]
    transformation_counter: Counter<u64>,
    #[cfg(feature = "observability-opentelemetry")]
    stream_counter: Counter<u64>,
    #[cfg(feature = "observability-opentelemetry")]
    storage_gauge: Gauge<i64>,
    #[cfg(feature = "observability-opentelemetry")]
    file_count_gauge: Gauge<i64>,
}

impl AnalyticsService {
    pub fn new(
        repo: Box<dyn AnalyticsRepositoryTrait + Send + Sync>,
        storage_repo: StorageMetricsRepository,
    ) -> Self {
        // Use bounded channel to prevent unbounded memory growth
        // Capacity of 10,000 logs should be sufficient for most use cases
        const CHANNEL_CAPACITY: usize = 10_000;
        let (log_sender, mut log_receiver) = mpsc::channel::<RequestLogInput>(CHANNEL_CAPACITY);

        let repo_arc: Arc<dyn AnalyticsRepositoryTrait + Send + Sync> = Arc::from(repo);
        let repo_clone = repo_arc.clone();

        // Spawn background task to process logs asynchronously
        tokio::spawn(async move {
            while let Some(log) = log_receiver.recv().await {
                if let Err(e) = repo_clone.log_request(log).await {
                    tracing::error!(error = ?e, "Failed to log request to analytics");
                }
            }
        });

        // Initialize custom business metrics
        #[cfg(feature = "observability-opentelemetry")]
        let (
            upload_counter,
            transformation_counter,
            stream_counter,
            storage_gauge,
            file_count_gauge,
        ) = {
            let meter = opentelemetry::global::meter("mindia");

            let upload_counter = meter
                .u64_counter("mindia.uploads.count")
                .with_description("Total number of uploads")
                .build();

            let transformation_counter = meter
                .u64_counter("mindia.transformations.count")
                .with_description("Total number of image transformations")
                .build();

            let stream_counter = meter
                .u64_counter("mindia.streams.count")
                .with_description("Total number of video stream requests")
                .build();

            let storage_gauge = meter
                .i64_gauge("mindia.storage.bytes")
                .with_description("Total storage used in bytes")
                .with_unit("bytes")
                .build();

            let file_count_gauge = meter
                .i64_gauge("mindia.files.count")
                .with_description("Total number of files stored")
                .build();

            (
                upload_counter,
                transformation_counter,
                stream_counter,
                storage_gauge,
                file_count_gauge,
            )
        };

        Self {
            repo: repo_arc,
            storage_repo,
            log_sender,
            #[cfg(feature = "observability-opentelemetry")]
            upload_counter,
            #[cfg(feature = "observability-opentelemetry")]
            transformation_counter,
            #[cfg(feature = "observability-opentelemetry")]
            stream_counter,
            #[cfg(feature = "observability-opentelemetry")]
            storage_gauge,
            #[cfg(feature = "observability-opentelemetry")]
            file_count_gauge,
        }
    }

    /// Record an upload event
    pub fn record_upload(&self, content_type: &str, success: bool) {
        let _ = (content_type, success);
        #[cfg(feature = "observability-opentelemetry")]
        {
            let labels = &[
                KeyValue::new("content_type", content_type.to_string()),
                KeyValue::new("success", success.to_string()),
            ];
            self.upload_counter.add(1, labels);
        }
    }

    /// Record a transformation event
    pub fn record_transformation(&self, operation: &str) {
        let _ = operation;
        #[cfg(feature = "observability-opentelemetry")]
        {
            let labels = &[KeyValue::new("operation", operation.to_string())];
            self.transformation_counter.add(1, labels);
        }
    }

    /// Record a stream request
    pub fn record_stream(&self, variant: &str) {
        let _ = variant;
        #[cfg(feature = "observability-opentelemetry")]
        {
            let labels = &[KeyValue::new("variant", variant.to_string())];
            self.stream_counter.add(1, labels);
        }
    }

    /// Update storage metrics gauges
    pub fn update_storage_metrics(&self, total_bytes: i64, total_files: i64) {
        let _ = (total_bytes, total_files);
        #[cfg(feature = "observability-opentelemetry")]
        {
            self.storage_gauge.record(total_bytes, &[]);
            self.file_count_gauge.record(total_files, &[]);
        }
    }

    /// Log a request asynchronously (non-blocking)
    /// Returns error if the channel is full (backpressure handling)
    pub fn log_request(&self, log: RequestLogInput) {
        // Try to send without blocking. If channel is full, log error and drop the log
        // This prevents backpressure from affecting request handling
        match self.log_sender.try_send(log) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!("Analytics log queue is full, dropping log entry. Consider increasing channel capacity or processing speed.");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::error!("Analytics log channel is closed, cannot log request");
            }
        }
    }

    /// Get traffic summary with optional date filtering
    pub async fn get_traffic_summary(
        &self,
        query: AnalyticsQuery,
    ) -> Result<TrafficSummary, anyhow::Error> {
        let limit = query.limit.unwrap_or(20);

        // Get overall traffic stats
        let (total_requests, total_bytes_sent, total_bytes_received, avg_response_time_ms) = self
            .repo
            .get_traffic_summary(query.start_date, query.end_date)
            .await
            .map_err(anyhow::Error::from)?;

        // Get popular URLs
        let popular_urls = self
            .repo
            .get_url_statistics(query.start_date, query.end_date, limit)
            .await
            .map_err(anyhow::Error::from)?;

        // Get requests per method
        let requests_per_method = self
            .repo
            .get_requests_per_method(query.start_date, query.end_date)
            .await
            .map_err(anyhow::Error::from)?;

        // Get requests per status code
        let requests_per_status = self
            .repo
            .get_requests_per_status(query.start_date, query.end_date)
            .await
            .map_err(anyhow::Error::from)?;

        Ok(TrafficSummary {
            total_requests,
            total_bytes_sent,
            total_bytes_received,
            avg_response_time_ms,
            requests_per_method,
            requests_per_status,
            popular_urls,
        })
    }

    /// Get statistics for a specific URL pattern
    pub async fn get_url_statistics(
        &self,
        query: AnalyticsQuery,
    ) -> Result<Vec<UrlStatistics>, anyhow::Error> {
        let limit = query.limit.unwrap_or(20);
        self.repo
            .get_url_statistics(query.start_date, query.end_date, limit)
            .await
            .map_err(anyhow::Error::from)
    }

    /// Get current storage metrics
    pub async fn get_storage_summary(&self) -> Result<StorageSummary, anyhow::Error> {
        // Try to get latest cached metrics
        if let Some(metrics) = self
            .storage_repo
            .get_latest_storage_metrics()
            .await
            .map_err(anyhow::Error::from)?
        {
            // Parse by_content_type from JSONB
            let by_content_type: HashMap<String, ContentTypeStats> =
                serde_json::from_value(metrics.by_content_type).unwrap_or_else(|_| HashMap::new());

            // Update OpenTelemetry gauges
            self.update_storage_metrics(metrics.total_storage_bytes, metrics.total_files);

            return Ok(StorageSummary {
                total_files: metrics.total_files,
                total_storage_bytes: metrics.total_storage_bytes,
                image_count: metrics.image_count,
                image_bytes: metrics.image_bytes,
                video_count: metrics.video_count,
                video_bytes: metrics.video_bytes,
                audio_count: metrics.audio_count,
                audio_bytes: metrics.audio_bytes,
                by_content_type,
            });
        }

        // If no cached metrics, calculate fresh
        let metrics = self
            .storage_repo
            .calculate_storage_metrics()
            .await
            .map_err(anyhow::Error::from)?;
        let by_content_type: HashMap<String, ContentTypeStats> =
            serde_json::from_value(metrics.by_content_type).unwrap_or_else(|_| HashMap::new());

        // Update OpenTelemetry gauges
        self.update_storage_metrics(metrics.total_storage_bytes, metrics.total_files);

        Ok(StorageSummary {
            total_files: metrics.total_files,
            total_storage_bytes: metrics.total_storage_bytes,
            image_count: metrics.image_count,
            image_bytes: metrics.image_bytes,
            video_count: metrics.video_count,
            video_bytes: metrics.video_bytes,
            audio_count: metrics.audio_count,
            audio_bytes: metrics.audio_bytes,
            by_content_type,
        })
    }

    /// Recalculate and store storage metrics snapshot
    pub async fn refresh_storage_metrics(&self) -> Result<(), anyhow::Error> {
        self.storage_repo
            .calculate_storage_metrics()
            .await
            .map_err(anyhow::Error::from)?;
        Ok(())
    }

    /// List audit logs with filtering and pagination
    pub async fn list_audit_logs(
        &self,
        tenant_id: Option<Uuid>,
        query: AuditLogQuery,
    ) -> Result<AuditLogListResponse, anyhow::Error> {
        // Get total count for pagination
        let total = self
            .repo
            .count_audit_logs(tenant_id, &query)
            .await
            .map_err(anyhow::Error::from)?;

        // Get the logs
        let logs = self
            .repo
            .list_audit_logs(tenant_id, query.clone())
            .await
            .map_err(anyhow::Error::from)?;

        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        Ok(AuditLogListResponse {
            logs,
            total,
            limit,
            offset,
        })
    }

    /// Get a single audit log by ID
    pub async fn get_audit_log(
        &self,
        id: i64,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<RequestLog>, anyhow::Error> {
        self.repo
            .get_audit_log_by_id(id, tenant_id)
            .await
            .map_err(anyhow::Error::from)
    }
}

/// Start a background task to periodically refresh storage metrics
pub fn start_storage_metrics_refresh(
    analytics: Arc<AnalyticsService>,
    interval_hours: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(interval_hours * 3600));

        loop {
            interval.tick().await;

            tracing::info!("Refreshing storage metrics snapshot");

            if let Err(e) = analytics.refresh_storage_metrics().await {
                tracing::error!(error = %e, "Failed to refresh storage metrics");
            } else {
                tracing::info!("Storage metrics refreshed successfully");
            }
        }
    })
}
