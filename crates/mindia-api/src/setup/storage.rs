//! Storage setup and initialization

use anyhow::Result;
use mindia_core::{Config, StorageBackend};
use mindia_storage::{create_storage, Storage};
use std::sync::Arc;

use crate::state::S3Config;

/// Setup storage; when backend is S3, also return bucket/region/endpoint for config and content moderation.
pub async fn setup_storage(config: &Config) -> Result<(Option<S3Config>, Arc<dyn Storage>)> {
    tracing::info!("Initializing storage abstraction...");
    let storage = create_storage(config).await?;
    let backend_type = storage.backend_type();
    tracing::info!(
        backend = ?backend_type,
        "Storage abstraction initialized successfully"
    );

    let s3_config = if backend_type == StorageBackend::S3 {
        let bucket = config.s3_bucket().map(String::from).ok_or_else(|| {
            anyhow::anyhow!("S3_BUCKET must be set when using S3 storage backend")
        })?;
        let region = config
            .s3_region()
            .map(String::from)
            .or_else(|| config.aws_region().map(String::from))
            .ok_or_else(|| {
                anyhow::anyhow!("S3_REGION or AWS_REGION must be set when using S3 storage backend")
            })?;
        let endpoint_url = config.s3_endpoint().map(String::from);
        Some(S3Config {
            bucket,
            region,
            endpoint_url,
        })
    } else {
        None
    };

    Ok((s3_config, storage))
}
