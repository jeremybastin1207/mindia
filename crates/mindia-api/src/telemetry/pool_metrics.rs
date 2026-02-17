//! Database connection pool metrics
//!
//! This module provides metrics for monitoring database connection pool health.

#![allow(dead_code)]

#[cfg(feature = "observability-opentelemetry")]
use opentelemetry::metrics::{Gauge, Meter};
use sqlx::PgPool;
use std::sync::Arc;
#[cfg(feature = "observability-opentelemetry")]
use tokio::time::{interval, Duration};

/// Connection pool metrics
#[cfg(feature = "observability-opentelemetry")]
pub struct PoolMetrics {
    pool_size: Gauge<i64>,
    idle_connections: Gauge<i64>,
    active_connections: Gauge<i64>,
}

#[cfg(feature = "observability-opentelemetry")]
impl PoolMetrics {
    pub fn new(meter: Meter) -> Self {
        let pool_size = meter
            .i64_gauge("db.pool.size")
            .with_description("Maximum number of connections in the pool")
            .build();

        let idle_connections = meter
            .i64_gauge("db.pool.idle")
            .with_description("Number of idle connections in the pool")
            .build();

        let active_connections = meter
            .i64_gauge("db.pool.active")
            .with_description("Number of active connections in the pool")
            .build();

        Self {
            pool_size,
            idle_connections,
            active_connections,
        }
    }

    /// Update metrics with current pool statistics
    pub fn update(&self, size: usize, idle: usize, active: usize) {
        self.pool_size.record(size as i64, &[]);
        self.idle_connections.record(idle as i64, &[]);
        self.active_connections.record(active as i64, &[]);
    }
}

/// Starts a background task that updates pool metrics every 30 seconds.
#[cfg(feature = "observability-opentelemetry")]
pub fn start_pool_metrics_collector(pool: Arc<PgPool>, metrics: PoolMetrics) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            // sqlx doesn't expose pool stats directly; we approximate
            // by checking pool size configuration and current connections
            let pool_size = pool.size() as usize;
            let num_idle = pool.num_idle();
            let _num_connections = num_idle; // sqlx doesn't track active separately

            // Active connections = total - idle (approximate)
            // Since sqlx doesn't expose this directly, we use 0 as a conservative estimate
            // when we can't determine it
            let active = pool_size.saturating_sub(num_idle);

            metrics.update(pool_size, num_idle, active);

            tracing::debug!(
                pool_size = pool_size,
                idle = num_idle,
                active = active,
                "Updated pool metrics"
            );
        }
    });
}

#[cfg(not(feature = "observability-opentelemetry"))]
pub struct PoolMetrics;

#[cfg(not(feature = "observability-opentelemetry"))]
impl PoolMetrics {
    pub fn new(_meter: ()) -> Self {
        Self
    }
}

#[cfg(not(feature = "observability-opentelemetry"))]
pub fn start_pool_metrics_collector(_pool: Arc<PgPool>, _metrics: PoolMetrics) {
    // No-op when OpenTelemetry is disabled
}
