//! Upload pipeline: validate → scan → process → store.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use mindia_storage::Storage;

use super::traits::{UploadConfig, UploadProcessor, VirusScanner};
use super::types::{UploadData, ValidatedFile};

fn validate_file_size(file_size: usize, max_size: usize) -> Result<()> {
    if file_size > max_size {
        anyhow::bail!(
            "File size exceeds maximum allowed size of {} MB",
            max_size / 1024 / 1024
        );
    }
    Ok(())
}

fn validate_content_type(content_type: &str, allowed: &[String]) -> Result<()> {
    if !allowed
        .iter()
        .any(|ct| content_type.to_lowercase().contains(&ct.to_lowercase()))
    {
        anyhow::bail!("Invalid content type. Allowed: {}", allowed.join(", "));
    }
    Ok(())
}

fn validate_extension(filename: &str, allowed: &[String]) -> Result<String> {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    if !allowed.contains(&ext) {
        anyhow::bail!("Invalid extension. Allowed: {}", allowed.join(", "));
    }
    Ok(ext)
}

fn sanitize_filename(filename: &str) -> String {
    const MAX: usize = 255;
    let path = std::path::Path::new(filename);
    let base = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename);
    if base.contains("..") {
        return "invalid_filename".to_string();
    }
    let s: String = base
        .chars()
        .take(MAX)
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.trim().is_empty() || s.len() < 3 {
        "file".to_string()
    } else {
        s
    }
}

/// Run the upload pipeline: validate → scan → process → store.
#[allow(clippy::too_many_arguments)]
pub async fn upload_pipeline<M>(
    tenant_id: Uuid,
    data: Vec<u8>,
    original_filename: String,
    content_type: String,
    config: &dyn UploadConfig,
    processor: Arc<dyn UploadProcessor<Metadata = M> + Send + Sync>,
    storage: Arc<dyn Storage>,
    scanner: Option<Arc<dyn VirusScanner>>,
    store_behavior: String,
    store_permanently: bool,
    expires_at: Option<DateTime<Utc>>,
    bucket: String,
) -> Result<(UploadData, M)>
where
    M: Send + 'static,
{
    validate_file_size(data.len(), config.max_file_size())?;
    validate_content_type(&content_type, config.allowed_content_types())?;
    let extension = validate_extension(&original_filename, config.allowed_extensions())?;

    let mut validated = ValidatedFile {
        data,
        original_filename: original_filename.clone(),
        content_type: content_type.clone(),
        extension: extension.clone(),
    };

    let metadata = processor
        .extract_metadata(&validated.data)
        .await
        .context("Metadata extraction failed")?;

    if let Some(s) = &scanner {
        s.scan(&validated.data).await.context("Virus scan failed")?;
    }

    validated.data = processor
        .sanitize(std::mem::take(&mut validated.data))
        .await
        .context("Sanitization failed")?;

    let file_id = Uuid::new_v4();
    let safe = sanitize_filename(&validated.original_filename);
    let uuid_filename = format!("{}.{}", file_id, validated.extension);
    let file_size = validated.data.len();

    let (storage_key, storage_url) = storage
        .upload(
            tenant_id,
            &uuid_filename,
            &validated.content_type,
            validated.data,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Storage upload failed")?;

    Ok((
        UploadData {
            tenant_id,
            file_id,
            uuid_filename,
            safe_original_filename: safe,
            storage_key,
            bucket,
            storage_url,
            content_type: validated.content_type,
            file_size: file_size as i64,
            store_behavior,
            store_permanently,
            expires_at,
        },
        metadata,
    ))
}
