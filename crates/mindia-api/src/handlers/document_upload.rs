use crate::auth::models::TenantContext;
use crate::error::{error_response_with_event, ErrorResponse, HttpAppError};
use crate::middleware::json_response_with_event;
use crate::services::upload::{DocumentProcessorImpl, MediaUploadConfig, MediaUploadService};
use crate::state::AppState;
use crate::telemetry::wide_event::WideEvent;
use crate::utils::upload::parse_store_parameter;
use axum::{
    extract::{Multipart, Query, State},
    http::Response,
    Json,
};
use chrono::Utc;
use mindia_core::models::DocumentResponse;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct StoreQuery {
    #[serde(default = "default_store")]
    store: String,
}

fn default_store() -> String {
    "auto".to_string()
}

#[utoipa::path(
    post,
    path = "/api/v0/documents",
    tag = "documents",
    params(
        ("store" = Option<String>, Query, description = "Storage behavior: '0' (temporary), '1' (permanent), 'auto' (default)")
    ),
    request_body(content = inline(Object), content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Document uploaded successfully", body = DocumentResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 413, description = "File too large", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn upload_document(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(query): Query<StoreQuery>,
    multipart: Multipart,
) -> Result<Response<axum::body::Body>, HttpAppError> {
    // Create a wide event for this request (middleware will merge with its event)
    // We can't use WideEventCtx extractor with Multipart, so we create our own
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut wide_event = WideEvent::new(
        request_id,
        state.config.otel_service_name().to_string(),
        state.config.environment().to_string(),
        "POST".to_string(),
        "/api/v0/documents".to_string(),
        Utc::now(),
    );
    wide_event.with_tenant_context(&tenant_ctx);

    wide_event.with_business_context(|ctx| {
        ctx.media_type = Some("document".to_string());
        ctx.operation = Some("upload".to_string());
    });

    let (store_permanently, expires_at) =
        match parse_store_parameter(&query.store, state.config.auto_store_enabled()) {
            Ok(result) => result,
            Err(e) => return Ok(error_response_with_event(HttpAppError::from(e), wide_event)),
        };
    let store_behavior = query.store.clone();

    let service = MediaUploadService::new(&state);
    let processor = DocumentProcessorImpl::new();

    struct DocumentUploadConfig<'a> {
        media: &'a crate::state::MediaConfig,
    }

    impl MediaUploadConfig for DocumentUploadConfig<'_> {
        fn max_file_size(&self) -> usize {
            self.media.document_max_file_size
        }

        fn allowed_extensions(&self) -> &[String] {
            &self.media.document_allowed_extensions
        }

        fn allowed_content_types(&self) -> &[String] {
            &self.media.document_allowed_content_types
        }

        fn media_type_name(&self) -> &'static str {
            "document"
        }
    }

    let config = DocumentUploadConfig {
        media: &state.media,
    };

    let (upload_data, _metadata) = match service
        .upload(
            tenant_ctx.tenant_id,
            multipart,
            &config,
            Box::new(processor),
            store_permanently,
            expires_at,
            store_behavior.clone(),
        )
        .await
    {
        Ok(result) => result,
        Err(e) => return Ok(error_response_with_event(HttpAppError::from(e), wide_event)),
    };

    let document = match state
        .media
        .repository
        .create_document_from_storage(
            upload_data.tenant_id,
            upload_data.file_id,
            upload_data.uuid_filename.clone(),
            upload_data.safe_original_filename.clone(),
            upload_data.content_type.clone(),
            upload_data.file_size,
            None, // page_count can be added later if needed
            upload_data.store_behavior.clone(),
            upload_data.store_permanently,
            upload_data.expires_at,
            None, // folder_id
            upload_data.storage_key.clone(),
            upload_data.storage_url.clone(),
        )
        .await
    {
        Ok(document) => {
            wide_event.with_business_context(|ctx| {
                ctx.media_id = Some(document.id);
                ctx.file_size = Some(document.file_size as u64);
            });
            document
        }
        Err(e) => {
            // Cleanup storage on database failure
            let storage_key = upload_data.storage_key.clone();
            let storage = state.media.storage.clone();
            tokio::spawn(async move {
                if let Err(cleanup_err) = storage.delete(&storage_key).await {
                    tracing::debug!(
                        error = %cleanup_err,
                        storage_key = %storage_key,
                        "Failed to cleanup storage file after DB error"
                    );
                }
            });
            return Ok(error_response_with_event(HttpAppError::from(e), wide_event));
        }
    };

    service.notify_upload(
        document.id,
        document.tenant_id,
        "document",
        document.original_filename.clone(),
        upload_data.storage_url.clone(),
        upload_data.storage_key.clone(),
        document.content_type.clone(),
        document.file_size,
        document.uploaded_at,
        document.store_permanently,
    );

    Ok(json_response_with_event(
        Json(DocumentResponse::from(document)),
        wide_event,
    ))
}
