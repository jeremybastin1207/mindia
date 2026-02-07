//! Plugin task handler for executing plugins asynchronously

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::plugins::{PluginContext, PluginExecutionStatus, PluginRegistry};
use crate::state::AppState;
use crate::task_handlers::TaskHandler;
use mindia_core::models::PluginExecutionStatus as DbPluginExecutionStatus;
use mindia_core::models::{
    PluginExecutionPayload, Task, WebhookDataInfo, WebhookEventType, WebhookInitiatorInfo,
};
use mindia_core::{EncryptionService, TaskError};
use mindia_db::{PluginConfigRepository, PluginExecutionRepository};

#[derive(Clone)]
pub struct PluginTaskHandler {
    registry: Arc<PluginRegistry>,
    config_repo: PluginConfigRepository,
    execution_repo: PluginExecutionRepository,
    encryption_service: EncryptionService,
}

impl PluginTaskHandler {
    pub fn new_with_encryption(
        registry: Arc<PluginRegistry>,
        config_repo: PluginConfigRepository,
        execution_repo: PluginExecutionRepository,
        encryption_service: EncryptionService,
    ) -> Self {
        Self {
            registry,
            config_repo,
            execution_repo,
            encryption_service,
        }
    }
}

#[async_trait]
impl TaskHandler for PluginTaskHandler {
    #[tracing::instrument(skip(self, task, state), fields(task.id = %task.id, plugin_name = tracing::field::Empty, media_id = tracing::field::Empty))]
    async fn process(&self, task: &Task, state: Arc<AppState>) -> Result<serde_json::Value> {
        // Parse payload
        let payload: PluginExecutionPayload = serde_json::from_value(task.payload.clone())
            .context("Failed to parse plugin execution payload")?;

        tracing::Span::current().record("plugin_name", &payload.plugin_name);
        tracing::Span::current().record("media_id", payload.media_id.to_string());

        tracing::info!(
            plugin_name = %payload.plugin_name,
            media_id = %payload.media_id,
            "Processing plugin execution task"
        );

        // Get plugin execution record
        // If not found, create one on the fly (handles race conditions where update_task_id failed)
        let execution = match self
            .execution_repo
            .get_execution_by_task_id(task.id)
            .await
            .context("Failed to get plugin execution record")?
        {
            Some(exec) => exec,
            None => {
                tracing::warn!(
                    task_id = %task.id,
                    plugin_name = %payload.plugin_name,
                    media_id = %payload.media_id,
                    "Plugin execution record not found, creating one"
                );
                self.execution_repo
                    .create_execution(
                        payload.tenant_id,
                        &payload.plugin_name,
                        payload.media_id,
                        Some(task.id),
                    )
                    .await
                    .context("Failed to create plugin execution record")?
            }
        };

        // Update execution status to running
        self.execution_repo
            .update_execution_status(execution.id, DbPluginExecutionStatus::Running, None)
            .await
            .context("Failed to update execution status")?;

        // Get plugin from registry
        let plugin = self
            .registry
            .get(&payload.plugin_name)
            .await
            .context("Plugin not found in registry")
            .map_err(|e| TaskError::unrecoverable(e))?;

        // Get plugin configuration - configuration errors are unrecoverable
        let plugin_config = self
            .config_repo
            .get_config(payload.tenant_id, &payload.plugin_name)
            .await
            .context("Failed to get plugin config")
            .map_err(|e| TaskError::unrecoverable(e))?
            .ok_or_else(|| {
                TaskError::unrecoverable(anyhow::anyhow!(
                    "Plugin '{}' is not configured for this tenant. Please configure the plugin with required credentials before use.",
                    payload.plugin_name
                ))
            })?;

        if !plugin_config.enabled {
            return Err(TaskError::unrecoverable(anyhow::anyhow!(
                "Plugin '{}' is not enabled for this tenant. Please enable the plugin before use.",
                payload.plugin_name
            ))
            .into());
        }

        // Decrypt the plugin configuration - decryption errors are unrecoverable
        let decrypted_config = plugin_config
            .get_full_config(&self.encryption_service)
            .context("Failed to decrypt plugin configuration")
            .map_err(|e| TaskError::unrecoverable(e))?;

        // Validate plugin configuration - validation errors are unrecoverable
        if let Err(e) = plugin.validate_config(&decrypted_config) {
            return Err(TaskError::unrecoverable(
                anyhow::anyhow!(
                    "Plugin '{}' configuration is invalid: {}. Please check the plugin configuration (API keys, credentials, etc.)",
                    payload.plugin_name,
                    e
                )
            ).into());
        }

        // Create plugin context
        // Note: db_pool is intentionally excluded to enforce use of repository methods
        // which ensure tenant isolation and authorized operations
        let context = PluginContext {
            tenant_id: payload.tenant_id,
            media_id: payload.media_id,
            storage: state.media.storage.clone(),
            media_repo: Arc::new(state.media.repository.clone()),
            file_group_repo: Arc::new(state.media.file_group_repository.clone()),
            config: decrypted_config,
        };

        // Execute plugin
        let result = plugin.execute(context).await;

        match result {
            Ok(plugin_result) => {
                match plugin_result.status {
                    PluginExecutionStatus::Success => {
                        tracing::info!(
                            plugin_name = %payload.plugin_name,
                            media_id = %payload.media_id,
                            "Plugin execution completed successfully"
                        );

                        // Some plugins update metadata directly and don't create documents
                        // Only process result for plugins that create documents (like assembly_ai)
                        let plugins_without_documents = [
                            "aws_rekognition",
                            "aws_rekognition_moderation",
                            "claude_vision",
                            "google_vision",
                        ];
                        let result_json =
                            if plugins_without_documents.contains(&payload.plugin_name.as_str()) {
                                // These plugins update metadata directly, no document creation needed
                                json!({
                                    "status": "success",
                                    "data": plugin_result.data,
                                })
                            } else {
                                // Process plugin result - create transcript file and associate with audio
                                let transcript_media_id =
                                    Self::process_plugin_result(&payload, &plugin_result, &state)
                                        .await
                                        .context("Failed to process plugin result")?;

                                // Create media group to associate audio and transcript
                                let group = state
                                    .media
                                    .file_group_repository
                                    .create_group(
                                        payload.tenant_id,
                                        vec![payload.media_id, transcript_media_id],
                                    )
                                    .await
                                    .context("Failed to create media group")?;

                                tracing::info!(
                                    group_id = %group.id,
                                    audio_id = %payload.media_id,
                                    transcript_id = %transcript_media_id,
                                    "Media group created for audio and transcript"
                                );

                                json!({
                                    "status": "success",
                                    "transcript_media_id": transcript_media_id,
                                    "group_id": group.id,
                                    "data": plugin_result.data,
                                })
                            };

                        let (unit_type, input_units, output_units, total_units, raw) =
                            match &plugin_result.usage {
                                Some(u) => (
                                    Some(u.unit_type.as_str()),
                                    u.input_units,
                                    u.output_units,
                                    Some(u.total_units),
                                    u.raw_usage.as_ref(),
                                ),
                                None => (None, None, None, None, None),
                            };

                        self.execution_repo
                            .update_execution_with_usage(
                                execution.id,
                                DbPluginExecutionStatus::Completed,
                                Some(result_json.clone()),
                                unit_type,
                                input_units,
                                output_units,
                                total_units,
                                raw,
                            )
                            .await
                            .context("Failed to update execution status")?;

                        // Trigger webhook for plugin execution completed
                        let state_clone = state.clone();
                        let tenant_id = payload.tenant_id;
                        let media_id = payload.media_id;
                        let plugin_name = payload.plugin_name.clone();
                        tokio::spawn(async move {
                            // Get media data for webhook
                            if let Ok(Some(media)) =
                                state_clone.media.repository.get(tenant_id, media_id).await
                            {
                                let (url, filename, content_type, file_size, uploaded_at) =
                                    match media {
                                        mindia_core::models::Media::Image(img) => (
                                            img.storage_url().to_string(),
                                            img.original_filename,
                                            img.content_type,
                                            img.file_size,
                                            img.uploaded_at,
                                        ),
                                        mindia_core::models::Media::Video(vid) => (
                                            vid.storage_url().to_string(),
                                            vid.original_filename,
                                            vid.content_type,
                                            vid.file_size,
                                            vid.uploaded_at,
                                        ),
                                        mindia_core::models::Media::Audio(aud) => (
                                            aud.storage_url().to_string(),
                                            aud.original_filename,
                                            aud.content_type,
                                            aud.file_size,
                                            aud.uploaded_at,
                                        ),
                                        mindia_core::models::Media::Document(doc) => (
                                            doc.storage_url().to_string(),
                                            doc.original_filename,
                                            doc.content_type,
                                            doc.file_size,
                                            doc.uploaded_at,
                                        ),
                                    };

                                let webhook_data = WebhookDataInfo {
                                    id: media_id,
                                    filename,
                                    url,
                                    content_type,
                                    file_size,
                                    entity_type: format!("plugin:{}", plugin_name),
                                    uploaded_at: Some(uploaded_at),
                                    deleted_at: None,
                                    stored_at: None,
                                    processing_status: Some("completed".to_string()),
                                    error_message: None,
                                };

                                let webhook_initiator = WebhookInitiatorInfo {
                                    initiator_type: "plugin".to_string(),
                                    id: tenant_id,
                                };

                                if let Err(e) = state_clone
                                    .webhook_service
                                    .trigger_event(
                                        tenant_id,
                                        WebhookEventType::FileProcessingCompleted,
                                        webhook_data,
                                        webhook_initiator,
                                    )
                                    .await
                                {
                                    tracing::warn!(
                                        error = %e,
                                        tenant_id = %tenant_id,
                                        media_id = %media_id,
                                        "Failed to trigger webhook for plugin execution completed"
                                    );
                                }
                            }
                        });

                        Ok(result_json)
                    }
                    PluginExecutionStatus::Failed => {
                        let error_msg = plugin_result
                            .error
                            .unwrap_or_else(|| "Plugin execution failed".to_string());

                        tracing::error!(
                            plugin_name = %payload.plugin_name,
                            media_id = %payload.media_id,
                            error = %error_msg,
                            "Plugin execution failed"
                        );

                        // Update execution status to failed
                        let error_result = json!({
                            "status": "failed",
                            "error": error_msg,
                        });

                        self.execution_repo
                            .update_execution_status(
                                execution.id,
                                DbPluginExecutionStatus::Failed,
                                Some(error_result.clone()),
                            )
                            .await
                            .ok(); // Don't fail if update fails

                        // Trigger webhook for plugin execution failed
                        let state_clone = state.clone();
                        let tenant_id = payload.tenant_id;
                        let media_id = payload.media_id;
                        let plugin_name = payload.plugin_name.clone();
                        let error_message = error_msg.clone();
                        tokio::spawn(async move {
                            // Get media data for webhook
                            if let Ok(Some(media)) =
                                state_clone.media.repository.get(tenant_id, media_id).await
                            {
                                let (url, filename, content_type, file_size, uploaded_at) =
                                    match media {
                                        mindia_core::models::Media::Image(img) => (
                                            img.storage_url().to_string(),
                                            img.original_filename,
                                            img.content_type,
                                            img.file_size,
                                            img.uploaded_at,
                                        ),
                                        mindia_core::models::Media::Video(vid) => (
                                            vid.storage_url().to_string(),
                                            vid.original_filename,
                                            vid.content_type,
                                            vid.file_size,
                                            vid.uploaded_at,
                                        ),
                                        mindia_core::models::Media::Audio(aud) => (
                                            aud.storage_url().to_string(),
                                            aud.original_filename,
                                            aud.content_type,
                                            aud.file_size,
                                            aud.uploaded_at,
                                        ),
                                        mindia_core::models::Media::Document(doc) => (
                                            doc.storage_url().to_string(),
                                            doc.original_filename,
                                            doc.content_type,
                                            doc.file_size,
                                            doc.uploaded_at,
                                        ),
                                    };

                                let webhook_data = WebhookDataInfo {
                                    id: media_id,
                                    filename,
                                    url,
                                    content_type,
                                    file_size,
                                    entity_type: format!("plugin:{}", plugin_name),
                                    uploaded_at: Some(uploaded_at),
                                    deleted_at: None,
                                    stored_at: None,
                                    processing_status: Some("failed".to_string()),
                                    error_message: Some(error_message.clone()),
                                };

                                let webhook_initiator = WebhookInitiatorInfo {
                                    initiator_type: "plugin".to_string(),
                                    id: tenant_id,
                                };

                                if let Err(e) = state_clone
                                    .webhook_service
                                    .trigger_event(
                                        tenant_id,
                                        WebhookEventType::FileProcessingFailed,
                                        webhook_data,
                                        webhook_initiator,
                                    )
                                    .await
                                {
                                    tracing::warn!(
                                        error = %e,
                                        tenant_id = %tenant_id,
                                        media_id = %media_id,
                                        "Failed to trigger webhook for plugin execution failed"
                                    );
                                }
                            }
                        });

                        // Check if this is a configuration error in the plugin result
                        if error_msg.to_lowercase().contains("api key")
                            || error_msg.to_lowercase().contains("not configured")
                            || error_msg.to_lowercase().contains("invalid configuration")
                            || error_msg.to_lowercase().contains("missing")
                        {
                            return Err(TaskError::unrecoverable(anyhow::anyhow!(
                                "Plugin execution failed due to configuration issue: {}",
                                error_msg
                            ))
                            .into());
                        }

                        Err(anyhow::anyhow!("Plugin execution failed: {}", error_msg))
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    plugin_name = %payload.plugin_name,
                    media_id = %payload.media_id,
                    error = %e,
                    "Plugin execution error"
                );

                // Update execution status to failed
                let error_result = json!({
                    "status": "failed",
                    "error": e.to_string(),
                });

                self.execution_repo
                    .update_execution_status(
                        execution.id,
                        DbPluginExecutionStatus::Failed,
                        Some(error_result),
                    )
                    .await
                    .ok(); // Don't fail if update fails

                // Trigger webhook for plugin execution error
                let state_clone = state.clone();
                let tenant_id = payload.tenant_id;
                let media_id = payload.media_id;
                let plugin_name = payload.plugin_name.clone();
                let error_message = e.to_string();
                tokio::spawn(async move {
                    // Get media data for webhook
                    if let Ok(Some(media)) =
                        state_clone.media.repository.get(tenant_id, media_id).await
                    {
                        let (url, filename, content_type, file_size, uploaded_at) = match media {
                            mindia_core::models::Media::Image(img) => (
                                img.storage_url().to_string(),
                                img.original_filename,
                                img.content_type,
                                img.file_size,
                                img.uploaded_at,
                            ),
                            mindia_core::models::Media::Video(vid) => (
                                vid.storage_url().to_string(),
                                vid.original_filename,
                                vid.content_type,
                                vid.file_size,
                                vid.uploaded_at,
                            ),
                            mindia_core::models::Media::Audio(aud) => (
                                aud.storage_url().to_string(),
                                aud.original_filename,
                                aud.content_type,
                                aud.file_size,
                                aud.uploaded_at,
                            ),
                            mindia_core::models::Media::Document(doc) => (
                                doc.storage_url().to_string(),
                                doc.original_filename,
                                doc.content_type,
                                doc.file_size,
                                doc.uploaded_at,
                            ),
                        };

                        let webhook_data = WebhookDataInfo {
                            id: media_id,
                            filename,
                            url,
                            content_type,
                            file_size,
                            entity_type: format!("plugin:{}", plugin_name),
                            uploaded_at: Some(uploaded_at),
                            deleted_at: None,
                            stored_at: None,
                            processing_status: Some("error".to_string()),
                            error_message: Some(error_message),
                        };

                        let webhook_initiator = WebhookInitiatorInfo {
                            initiator_type: "plugin".to_string(),
                            id: tenant_id,
                        };

                        if let Err(e) = state_clone
                            .webhook_service
                            .trigger_event(
                                tenant_id,
                                WebhookEventType::FileProcessingFailed,
                                webhook_data,
                                webhook_initiator,
                            )
                            .await
                        {
                            tracing::warn!(
                                error = %e,
                                tenant_id = %tenant_id,
                                media_id = %media_id,
                                "Failed to trigger webhook for plugin execution error"
                            );
                        }
                    }
                });

                Err(e)
            }
        }
    }
}

impl PluginTaskHandler {
    /// Process plugin result - create transcript file from plugin output
    async fn process_plugin_result(
        payload: &PluginExecutionPayload,
        plugin_result: &crate::plugins::PluginResult,
        state: &Arc<AppState>,
    ) -> Result<Uuid> {
        // Get the original audio file to determine storage behavior
        let audio = state
            .media
            .repository
            .get_audio(payload.tenant_id, payload.media_id)
            .await
            .context("Failed to get audio file")?
            .context("Audio file not found")?;

        // Convert plugin result data to JSON bytes
        let transcript_json = serde_json::to_string_pretty(&plugin_result.data)
            .context("Failed to serialize plugin result to JSON")?;
        let transcript_bytes = transcript_json.into_bytes();

        // Create transcript filename
        let transcript_filename = format!("{}.transcript.json", payload.media_id);
        let original_filename = "transcript.json".to_string();

        // Upload transcript to storage and create document media
        let transcript_doc = state
            .media
            .repository
            .create_document(
                payload.tenant_id,
                transcript_filename.clone(),
                original_filename.clone(),
                "application/json".to_string(),
                transcript_bytes,
                None, // page_count
                audio.store_behavior.clone(),
                audio.store_permanently,
                audio.expires_at,
                None, // folder_id
            )
            .await
            .context("Failed to create transcript document")?;

        tracing::info!(
            transcript_id = %transcript_doc.id,
            filename = %transcript_filename,
            "Transcript document created"
        );

        Ok(transcript_doc.id)
    }
}
