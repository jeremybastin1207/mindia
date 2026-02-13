use std::sync::Arc;

use axum::{
    extract::{Multipart, Query, State},
    response::Response,
    Json,
};
use mindia_core::models::ImageResponse;
use serde::Deserialize;

use crate::auth::models::TenantContext;
use crate::error::{error_response_with_event, ErrorResponse, HttpAppError};
use crate::middleware::json_response_with_event;
use crate::utils::ip_extraction::ClientIpOpt;
use crate::utils::transaction::with_transaction;
use crate::services::upload::{
    ImageMetadata, ImageProcessorImpl, MediaProcessor, MediaUploadConfig, MediaUploadService,
};
use crate::state::{AppState, MediaConfig};
use crate::telemetry::wide_event::WideEvent;
use crate::utils::upload::parse_store_parameter;
use chrono::Utc;

#[derive(Debug, Deserialize)]
pub struct StoreQuery {
    #[serde(default = "default_store")]
    store: String,
    // Note: folder_id support removed - folder assignment should be done via separate API endpoint
}

fn default_store() -> String {
    "auto".to_string()
}

/// Upload image handler
///
/// Orchestrates the image upload process by delegating to MediaUploadService
/// for file validation, security scanning, storage, and event notifications.
///
/// # Arguments
/// * `query` - Query parameters including storage behavior ('0', '1', or 'auto')
/// * `state` - Application state containing services and configuration
/// * `tenant_ctx` - Authenticated tenant context from middleware
/// * `multipart` - Multipart form data containing the image file
///
/// # Returns
/// `ImageResponse` with uploaded image metadata on success (HTTP 201 Created)
///
/// # Errors
/// - `AppError::InvalidInput` - Invalid file or parameters
/// - `AppError::PayloadTooLarge` - File exceeds size limit
/// - `AppError::S3` - Storage upload failure
/// - `AppError::Internal` - Internal processing error
#[utoipa::path(
    post,
    path = "/api/v0/images",
    tag = "images",
    params(
        ("store" = Option<String>, Query, description = "Storage behavior: '0' (temporary), '1' (permanent), 'auto' (default)")
    ),
    request_body(content = inline(Object), content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Image uploaded successfully", body = ImageResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 413, description = "File too large", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state, multipart),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        store = %query.store,
        operation = "upload_image"
    )
)]
pub async fn upload_image(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(query): Query<StoreQuery>,
    ClientIpOpt(client_ip): ClientIpOpt,
    multipart: Multipart,
) -> Result<Response, HttpAppError> {
    // Create a wide event for this request (middleware will merge with its event)
    // We can't use WideEventCtx extractor with Multipart, so we create our own
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut wide_event = WideEvent::new(
        request_id,
        state.config.otel_service_name().to_string(),
        state.config.environment().to_string(),
        "POST".to_string(),
        "/api/v0/images".to_string(),
        Utc::now(),
    );
    wide_event.with_tenant_context(&tenant_ctx);

    struct ImageUploadConfig<'a> {
        media: &'a MediaConfig,
    }

    impl MediaUploadConfig for ImageUploadConfig<'_> {
        fn max_file_size(&self) -> usize {
            self.media.image_max_file_size
        }

        fn allowed_extensions(&self) -> &[String] {
            &self.media.image_allowed_extensions
        }

        fn allowed_content_types(&self) -> &[String] {
            &self.media.image_allowed_content_types
        }

        fn media_type_name(&self) -> &'static str {
            "image"
        }
    }

    wide_event.with_business_context(|ctx| {
        ctx.media_type = Some("image".to_string());
        ctx.operation = Some("upload".to_string());
    });

    let (store_permanently, expires_at) =
        match parse_store_parameter(&query.store, state.config.auto_store_enabled()) {
            Ok(result) => result,
            Err(e) => return Ok(error_response_with_event(HttpAppError::from(e), wide_event)),
        };
    let store_behavior = query.store.clone();

    let service = MediaUploadService::new(&state);
    let config = ImageUploadConfig {
        media: &state.media,
    };
    let processor: Box<dyn MediaProcessor<Metadata = ImageMetadata> + Send + Sync> =
        Box::new(ImageProcessorImpl::new(state.media.remove_exif));

    let (upload_data, metadata) = match service
        .upload(
            tenant_ctx.tenant_id,
            multipart,
            &config,
            processor,
            store_permanently,
            expires_at,
            store_behavior.clone(),
            Some(tenant_ctx.user_id),
            client_ip,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => return Ok(error_response_with_event(HttpAppError::from(e), wide_event)),
    };

    let image = match with_transaction(&state.db.pool, |tx| {
        let repo = state.media.repository.clone();
        let ud = upload_data.clone();
        let meta = metadata.clone();
        async move {
            repo.create_image_from_storage_tx(
                tx,
                ud.tenant_id,
                ud.file_id,
                ud.uuid_filename,
                ud.safe_original_filename,
                ud.content_type,
                ud.file_size,
                Some(meta.dimensions.width as i32),
                Some(meta.dimensions.height as i32),
                ud.store_behavior,
                ud.store_permanently,
                ud.expires_at,
                None,
                ud.storage_key,
                ud.storage_url,
            )
            .await
        }
    })
    .await
    {
        Ok(image) => image,
        Err(e) => {
            let storage_key = upload_data.storage_key.clone();
            let storage = state.media.storage.clone();
            tokio::spawn(async move {
                if let Err(cleanup_err) = storage.delete(&storage_key).await {
                    tracing::warn!(
                        error = %cleanup_err,
                        storage_key = %storage_key,
                        "Failed to cleanup storage file after DB error"
                    );
                }
            });
            return Ok(error_response_with_event(HttpAppError::from(e), wide_event));
        }
    };

    wide_event.with_business_context(|ctx| {
        ctx.media_id = Some(image.id);
        ctx.file_size = Some(image.file_size as u64);
    });

    service.notify_upload(
        image.id,
        image.tenant_id,
        "image",
        image.original_filename.clone(),
        upload_data.storage_url.clone(),
        upload_data.storage_key.clone(),
        image.content_type.clone(),
        image.file_size,
        image.uploaded_at,
        image.store_permanently,
        None,
        None,
    );

    Ok(json_response_with_event(
        Json(ImageResponse::from(image)),
        wide_event,
    ))
}
