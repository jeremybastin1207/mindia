//! Application setup and initialization
//!
//! This module contains all application initialization logic extracted from main.rs
//! for better organization and testability.

pub mod database;
pub mod routes;
pub mod server;
pub mod services;
pub mod storage;
pub mod validation;

use crate::state::AppState;
use anyhow::{Context, Result};
use mindia_core::Config;
use std::sync::Arc;

/// Initialize the entire application
pub async fn initialize_app(config: Config) -> Result<(Arc<AppState>, axum::Router)> {
    // Validate configuration first - fail fast on misconfiguration
    validation::validate_config(&config).context("Configuration validation failed")?;

    // Initialize telemetry first
    crate::telemetry::init_telemetry(
        config.otel_enabled(),
        config.otel_endpoint().map(|s| s.to_string()),
        config.otel_service_name().to_string(),
        config.otel_service_version().to_string(),
        config.otel_protocol().to_string(),
        config.environment().to_string(),
        config.otel_sampler().to_string(),
        config.otel_sample_ratio(),
        config.otel_metrics_interval_secs(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to initialize telemetry: {}", e))?;

    tracing::info!("Configuration loaded and validated successfully");

    // Setup database
    let pool = database::setup_database(&config).await?;

    // Setup storage
    let (s3_config, storage) = storage::setup_storage(&config).await?;

    // Initialize all services and repositories
    let state = services::initialize_services(&config, pool, s3_config, storage).await?;

    // Setup routes
    let router = routes::setup_routes(&config, state.clone()).await?;

    Ok((state, router))
}
