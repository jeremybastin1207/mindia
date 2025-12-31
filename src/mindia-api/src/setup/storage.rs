//! Storage setup and initialization

use anyhow::Result;
use mindia_core::{Config, StorageBackend};
use mindia_services::S3Service;
use mindia_storage::{create_storage, Storage};
use std::sync::Arc;

/// Setup storage services (S3 service is optional, only for S3 backend)
pub async fn setup_storage(config: &Config) -> Result<(Option<S3Service>, Arc<dyn Storage>)> {
    // Initialize storage abstraction first
    tracing::info!("Initializing storage abstraction...");
    let storage = create_storage(config).await?;
    let backend_type = storage.backend_type();
    tracing::info!(
        backend = ?backend_type,
        "Storage abstraction initialized successfully"
    );

    // Only initialize S3Service if using S3 backend
    let s3_service = if backend_type == StorageBackend::S3 {
        tracing::info!("Initializing S3 service...");
        let s3_bucket = config
            .s3_bucket()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("S3_BUCKET not configured (required for S3 backend)"))?;
        let s3_region = config
            .s3_region()
            .map(String::from)
            .or_else(|| config.aws_region().map(String::from))
            .ok_or_else(|| {
                anyhow::anyhow!("S3_REGION or AWS_REGION not configured (required for S3 backend)")
            })?;
        let s3_service = S3Service::new(Some(s3_bucket), s3_region).await?;
        tracing::info!("S3 service initialized successfully");
        Some(s3_service)
    } else {
        tracing::info!(
            "S3 service not required for storage backend: {:?}",
            backend_type
        );
        None
    };

    Ok((s3_service, storage))
}
