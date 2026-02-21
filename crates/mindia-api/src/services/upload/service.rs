//! Unified media upload service
//!
//! This service provides a unified upload pipeline that handles all media types
//! through a configurable workflow: extract → validate → scan → process → store → persist → track → notify

use std::sync::Arc;

use axum::extract::Multipart;
use chrono::DateTime;
use mindia_core::models::{ContentModerationPayload, GenerateEmbeddingPayload, Priority, TaskType};
use mindia_core::AppError;
use uuid::Uuid;

use crate::middleware::audit;
use crate::state::AppState;
use crate::utils::upload::{
    extract_multipart_file, handle_clamav_scan, sanitize_filename, validate_content_type,
    validate_extension_content_type_match, validate_file_extension, validate_file_size,
};

use super::traits::{MediaProcessor, MediaUploadConfig};
use super::types::ValidatedFile;

/// Unified media upload service
///
/// This service orchestrates the complete upload workflow for all media types
/// through a configurable pipeline with type-specific processors.
pub struct MediaUploadService {
    state: Arc<AppState>,
}

impl MediaUploadService {
    /// Create a new MediaUploadService
    pub fn new(state: &Arc<AppState>) -> Self {
        Self {
            state: state.clone(),
        }
    }

    /// Complete upload workflow: extract → validate → scan → process → store → track
    ///
    /// # Type Parameters
    /// - `M`: Metadata type returned by the processor
    ///
    /// # Arguments
    /// - `tenant_id`: Tenant ID for the upload
    /// - `multipart`: Multipart form data containing the file
    /// - `config`: Media type configuration (max size, allowed types, etc.)
    /// - `processor`: Media processor for type-specific operations
    /// - `store_permanently`: Whether to store permanently
    /// - `expires_at`: Expiration time if not permanent
    /// - `store_behavior`: Store behavior string ('0', '1', or 'auto')
    ///
    /// # Returns
    /// `UploadData` containing all information needed to create the entity, plus extracted metadata
    #[allow(clippy::too_many_arguments)]
    pub async fn upload<M>(
        &self,
        tenant_id: Uuid,
        multipart: Multipart,
        config: &dyn MediaUploadConfig,
        processor: Box<dyn MediaProcessor<Metadata = M> + Send + Sync + 'static>,
        store_permanently: bool,
        expires_at: Option<DateTime<chrono::Utc>>,
        store_behavior: String,
        audit_user_id: Option<Uuid>,
        audit_client_ip: Option<String>,
    ) -> Result<(super::types::UploadData, M), AppError>
    where
        M: Send + 'static,
    {
        // 1. Extract and validate file
        let mut validated = self.extract_and_validate(multipart, config).await?;

        // 2. Extract metadata using processor
        let metadata = processor
            .extract_metadata(&validated.data)
            .await
            .map_err(|e| {
                AppError::InvalidInput(format!("Invalid {} file: {}", config.media_type_name(), e))
            })?;

        // 3. Scan security (ClamAV)
        validated = self.scan_security(validated).await?;

        // 4. Sanitize using processor (EXIF removal, etc.)
        validated.data = processor.sanitize(validated.data).await?;

        // 5. Upload to storage
        let upload_data = self
            .upload_to_storage(
                tenant_id,
                validated,
                store_permanently,
                expires_at,
                store_behavior,
                audit_user_id,
                audit_client_ip,
            )
            .await?;

        // 6. Track usage metrics
        self.track_usage(tenant_id, upload_data.file_size as usize)
            .await?;

        Ok((upload_data, metadata))
    }

    /// Extract and validate file from multipart request
    async fn extract_and_validate(
        &self,
        multipart: Multipart,
        config: &dyn MediaUploadConfig,
    ) -> Result<ValidatedFile, AppError> {
        let (file_data, original_filename, content_type) =
            extract_multipart_file(multipart).await?;

        validate_file_size(file_data.len(), config.max_file_size())?;
        validate_content_type(&content_type, config.allowed_content_types())?;
        let extension = validate_file_extension(&original_filename, config.allowed_extensions())?;
        validate_extension_content_type_match(&original_filename, &content_type)?;

        Ok(ValidatedFile {
            data: file_data,
            original_filename,
            content_type,
            extension,
        })
    }

    /// Scan file with ClamAV if enabled
    async fn scan_security(&self, file: ValidatedFile) -> Result<ValidatedFile, AppError> {
        if self.state.security.clamav_enabled {
            #[cfg(feature = "clamav")]
            if let Some(ref clamav) = self.state.security.clamav {
                tracing::debug!("Scanning file with ClamAV");
                let scan_result = clamav.scan_bytes(&file.data).await;
                handle_clamav_scan(scan_result, &file.original_filename).await?;
            }
        }
        Ok(file)
    }

    /// Upload file to storage
    ///
    /// This method handles storage upload and returns all data needed to create
    /// the database entity. Database persistence is handled by the caller.
    #[allow(clippy::too_many_arguments)]
    async fn upload_to_storage(
        &self,
        tenant_id: Uuid,
        file: ValidatedFile,
        store_permanently: bool,
        expires_at: Option<DateTime<chrono::Utc>>,
        store_behavior: String,
        audit_user_id: Option<Uuid>,
        audit_client_ip: Option<String>,
    ) -> Result<super::types::UploadData, AppError> {
        let file_uuid = Uuid::new_v4();
        let safe_original_filename = sanitize_filename(&file.original_filename)?;
        let uuid_filename = format!("{}.{}", file_uuid, file.extension);
        let file_size = file.data.len();

        tracing::info!(
                file_uuid = %file_uuid,
                original_filename = %safe_original_filename,
                file_size = file_size,
            "Processing upload"
        );

        let (storage_key, storage_url) = self
            .state
            .media
            .storage
            .upload(
                tenant_id,
                &uuid_filename,
                &file.content_type,
                file.data, // Move instead of clone
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, file_uuid = %file_uuid, "Failed to upload to storage");
                AppError::S3(format!("Failed to upload file: {}", e))
            })?;

        tracing::info!(
                file_uuid = %file_uuid,
                storage_url = %storage_url,
            "Upload to storage successful"
        );

        audit::log_file_upload(
            tenant_id,
            audit_user_id,
            file_uuid,
            safe_original_filename.clone(),
            file_size as i64,
            file.content_type.clone(),
            audit_client_ip,
        );

        Ok(super::types::UploadData {
            tenant_id,
            file_id: file_uuid,
            uuid_filename,
            safe_original_filename,
            storage_key,
            storage_url,
            content_type: file.content_type,
            file_size: file_size as i64,
            store_behavior,
            store_permanently,
            expires_at,
        })
    }

    /// Track usage metrics for uploaded file (no-op; billing/usage removed).
    pub async fn track_usage(&self, _tenant_id: Uuid, _file_size: usize) -> Result<(), AppError> {
        Ok(())
    }

    /// Notify about upload completion (webhooks, tasks)
    ///
    /// This method triggers webhooks and queues tasks (embeddings, moderation)
    /// asynchronously without blocking the upload response.
    #[allow(clippy::too_many_arguments)]
    pub fn notify_upload(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
        entity_type: &'static str,
        filename: String,
        storage_url: String,
        storage_key: String,
        content_type: String,
        file_size: i64,
        uploaded_at: DateTime<chrono::Utc>,
        store_permanently: bool,
        _folder_id: Option<Uuid>,
        _metadata: Option<serde_json::Value>,
    ) {
        // Trigger webhook for upload
        let webhook_data = mindia_core::models::WebhookDataInfo {
            id: entity_id,
            filename: filename.clone(),
            url: storage_url.clone(),
            content_type: content_type.clone(),
            file_size,
            entity_type: entity_type.to_string(),
            uploaded_at: Some(uploaded_at),
            deleted_at: None,
            stored_at: if store_permanently {
                Some(uploaded_at)
            } else {
                None
            },
            processing_status: None,
            error_message: None,
        };
        let webhook_initiator = mindia_core::models::WebhookInitiatorInfo {
            initiator_type: String::from("upload"),
            id: tenant_id,
        };
        let webhook_service = self.state.webhooks.webhook_service.clone();
        let tenant_id_clone = tenant_id;
        tokio::spawn(async move {
            if let Err(e) = webhook_service
                .trigger_event(
                    tenant_id_clone,
                    mindia_core::models::WebhookEventType::FileUploaded,
                    webhook_data,
                    webhook_initiator,
                )
                .await
            {
                tracing::warn!(
                    error = %e,
                    tenant_id = %tenant_id_clone,
                    "Failed to trigger webhook for upload"
                );
            }
        });

        if self.state.semantic_search.is_some() {
            let embedding_payload = GenerateEmbeddingPayload {
                entity_id,
                entity_type: entity_type.to_string(),
                s3_url: storage_url.clone(),
            };

            if let Ok(embedding_payload_json) = serde_json::to_value(&embedding_payload) {
                let task_queue = self.state.tasks.task_queue.clone();
                let tenant_id_clone = tenant_id;
                let entity_id_clone = entity_id;
                tokio::spawn(async move {
                    match task_queue
                        .submit_task(
                            tenant_id_clone,
                            TaskType::GenerateEmbedding,
                            embedding_payload_json,
                            Priority::Low,
                            None,
                            None,
                            false,
                        )
                        .await
                    {
                        Ok(task_id) => {
                            tracing::info!(
                                entity_id = %entity_id_clone,
                                task_id = %task_id,
                                "Embedding generation task queued successfully"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                entity_id = %entity_id_clone,
                                "Failed to queue embedding generation task"
                            );
                        }
                    }
                });
            } else {
                tracing::error!(
                    entity_id = %entity_id,
                    "Failed to serialize embedding payload"
                );
            }
        }

        let moderation_enabled = self.state.config.content_moderation_enabled();

        if !moderation_enabled {
            tracing::debug!(
                entity_id = %entity_id,
                "Content moderation disabled (CONTENT_MODERATION_ENABLED not set to true)"
            );
        }

        // AWS Rekognition video moderation requires S3; skip queuing when using local storage
        let queue_moderation =
            moderation_enabled && (entity_type != "video" || self.state.s3.is_some());
        if moderation_enabled && !queue_moderation {
            tracing::debug!(
                entity_id = %entity_id,
                "Skipping content moderation task for video (S3 not configured)"
            );
        }

        let moderation_payload = ContentModerationPayload {
            media_id: entity_id,
            media_type: entity_type.to_string(),
            s3_key: storage_key.clone(),
            s3_url: storage_url.clone(),
        };

        if queue_moderation {
            if let Ok(moderation_payload_json) = serde_json::to_value(&moderation_payload) {
                let task_queue = self.state.tasks.task_queue.clone();
                let tenant_id_clone = tenant_id;
                let entity_id_clone = entity_id;
                tokio::spawn(async move {
                    if let Err(e) = task_queue
                        .submit_task(
                            tenant_id_clone,
                            TaskType::ContentModeration,
                            moderation_payload_json,
                            Priority::Normal,
                            None,
                            None,
                            false,
                        )
                        .await
                    {
                        tracing::warn!(
                            error = %e,
                            entity_id = %entity_id_clone,
                            "Failed to queue content moderation task"
                        );
                    }
                });
            } else {
                tracing::error!(
                    entity_id = %entity_id,
                    "Failed to serialize moderation payload"
                );
            }
        }

        #[cfg(feature = "workflow")]
        {
            let workflow_service = self.state.workflows.workflow_service.clone();
            let tenant_id_w = tenant_id;
            let entity_id_w = entity_id;
            let entity_type_w = entity_type.to_string();
            let content_type_w = content_type.clone();
            let folder_id_w = _folder_id;
            let metadata_w = _metadata.clone();
            tokio::spawn(async move {
                if let Err(e) = workflow_service
                    .match_and_trigger(
                        tenant_id_w,
                        entity_id_w,
                        &entity_type_w,
                        folder_id_w,
                        &content_type_w,
                        metadata_w.as_ref(),
                    )
                    .await
                {
                    tracing::warn!(
                        error = %e,
                        entity_id = %entity_id_w,
                        "Failed to match and trigger workflows on upload"
                    );
                }
            });
        }
    }
}
