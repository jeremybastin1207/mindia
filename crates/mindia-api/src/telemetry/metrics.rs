//! OpenTelemetry metrics helpers
//!
//! This module provides helper functions to create common metrics
//! using the global meter provider.

#![allow(dead_code)] // Metric types used when wiring observability in other crates or future use

#[cfg(feature = "observability-opentelemetry")]
use opentelemetry::{
    metrics::{Counter, Histogram, Meter, UpDownCounter},
    KeyValue,
};

/// Create HTTP request metrics
#[cfg(feature = "observability-opentelemetry")]
pub fn create_http_metrics(meter: Meter) -> HttpMetrics {
    HttpMetrics::new(meter)
}

/// HTTP request metrics
#[cfg(feature = "observability-opentelemetry")]
pub struct HttpMetrics {
    pub request_counter: Counter<u64>,
    pub request_duration: Histogram<f64>,
    pub active_requests: UpDownCounter<i64>,
    pub error_counter: Counter<u64>,
}

#[cfg(feature = "observability-opentelemetry")]
impl HttpMetrics {
    pub fn new(meter: Meter) -> Self {
        let request_counter = meter
            .u64_counter("http.server.request.count")
            .with_description("Total number of HTTP requests")
            .build();

        let request_duration = meter
            .f64_histogram("http.server.request.duration")
            .with_description("HTTP request duration in seconds")
            .with_unit("s")
            .build();

        let active_requests = meter
            .i64_up_down_counter("http.server.active_requests")
            .with_description("Number of active HTTP requests")
            .build();

        let error_counter = meter
            .u64_counter("http.server.errors.count")
            .with_description("Total number of HTTP errors (4xx and 5xx responses)")
            .build();

        Self {
            request_counter,
            request_duration,
            active_requests,
            error_counter,
        }
    }
}

/// Database query metrics
#[cfg(feature = "observability-opentelemetry")]
pub struct DatabaseMetrics {
    pub query_counter: Counter<u64>,
    pub query_duration: Histogram<f64>,
    pub active_queries: UpDownCounter<i64>,
}

#[cfg(feature = "observability-opentelemetry")]
impl DatabaseMetrics {
    pub fn new(meter: Meter) -> Self {
        let query_counter = meter
            .u64_counter("db.client.queries.count")
            .with_description("Total number of database queries")
            .build();

        let query_duration = meter
            .f64_histogram("db.client.queries.duration")
            .with_description("Database query duration in seconds")
            .with_unit("s")
            .build();

        let active_queries = meter
            .i64_up_down_counter("db.client.active_queries")
            .with_description("Number of active database queries")
            .build();

        Self {
            query_counter,
            query_duration,
            active_queries,
        }
    }

    pub fn record_query_start(&self, operation: &str, table: &str) {
        self.active_queries.add(
            1,
            &[
                KeyValue::new("db.operation", operation.to_string()),
                KeyValue::new("db.sql.table", table.to_string()),
            ],
        );
    }

    pub fn record_query_end(&self, operation: &str, table: &str, duration: f64) {
        let labels = &[
            KeyValue::new("db.operation", operation.to_string()),
            KeyValue::new("db.sql.table", table.to_string()),
        ];

        self.query_counter.add(1, labels);
        self.query_duration.record(duration, labels);
        self.active_queries.add(-1, labels);
    }
}

/// S3 operation metrics
#[cfg(feature = "observability-opentelemetry")]
pub struct S3Metrics {
    pub operation_counter: Counter<u64>,
    pub operation_duration: Histogram<f64>,
    pub bytes_transferred: Counter<u64>,
}

#[cfg(feature = "observability-opentelemetry")]
impl S3Metrics {
    pub fn new(meter: Meter) -> Self {
        let operation_counter = meter
            .u64_counter("aws.s3.operations.count")
            .with_description("Total number of S3 operations")
            .build();

        let operation_duration = meter
            .f64_histogram("aws.s3.operations.duration")
            .with_description("S3 operation duration in seconds")
            .with_unit("s")
            .build();

        let bytes_transferred = meter
            .u64_counter("aws.s3.bytes.transferred")
            .with_description("Total bytes transferred to/from S3")
            .with_unit("By")
            .build();

        Self {
            operation_counter,
            operation_duration,
            bytes_transferred,
        }
    }

    pub fn record_operation(
        &self,
        operation: &str,
        bucket: &str,
        duration: f64,
        bytes: Option<u64>,
    ) {
        let labels = &[
            KeyValue::new("aws.s3.operation", operation.to_string()),
            KeyValue::new("aws.s3.bucket", bucket.to_string()),
        ];

        self.operation_counter.add(1, labels);
        self.operation_duration.record(duration, labels);

        if let Some(bytes) = bytes {
            self.bytes_transferred.add(bytes, labels);
        }
    }
}

/// Task queue metrics
#[cfg(feature = "observability-opentelemetry")]
pub struct TaskQueueMetrics {
    pub task_counter: Counter<u64>,
    pub task_duration: Histogram<f64>,
    pub active_tasks: UpDownCounter<i64>,
    pub task_failures: Counter<u64>,
}

#[cfg(feature = "observability-opentelemetry")]
impl TaskQueueMetrics {
    pub fn new(meter: Meter) -> Self {
        let task_counter = meter
            .u64_counter("mindia.task_queue.tasks.count")
            .with_description("Total number of tasks processed")
            .build();

        let task_duration = meter
            .f64_histogram("mindia.task_queue.tasks.duration")
            .with_description("Task processing duration in seconds")
            .with_unit("s")
            .build();

        let active_tasks = meter
            .i64_up_down_counter("mindia.task_queue.active_tasks")
            .with_description("Number of active tasks")
            .build();

        let task_failures = meter
            .u64_counter("mindia.task_queue.tasks.failures")
            .with_description("Total number of failed tasks")
            .build();

        Self {
            task_counter,
            task_duration,
            active_tasks,
            task_failures,
        }
    }

    pub fn record_task_start(&self, task_type: &str) {
        self.active_tasks
            .add(1, &[KeyValue::new("task.type", task_type.to_string())]);
    }

    pub fn record_task_end(&self, task_type: &str, duration: f64, success: bool) {
        let labels = &[KeyValue::new("task.type", task_type.to_string())];

        self.task_counter.add(1, labels);
        self.task_duration.record(duration, labels);
        self.active_tasks.add(-1, labels);

        if !success {
            self.task_failures.add(1, labels);
        }
    }
}

#[cfg(feature = "observability-opentelemetry")]
pub fn get_meter(name: &'static str) -> Meter {
    opentelemetry::global::meter(name)
}
