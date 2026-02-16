use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use crate::utils::ssrf_validation::validate_url_for_ssrf;
use crate::utils::upload::{
    handle_clamav_scan, parse_store_parameter, sanitize_filename, validate_content_type,
    validate_extension_content_type_match, validate_file_extension, validate_file_size,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use mindia_core::models::ImageResponse;
use mindia_core::models::{WebhookDataInfo, WebhookEventType, WebhookInitiatorInfo};
use mindia_core::AppError;
use mindia_services::ImageProcessor;
use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct UploadFromUrlQuery {
    #[serde(default = "default_store")]
    pub store: String,
    pub url: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UploadFromUrlResponse {
    pub message: String,
    pub image: ImageResponse,
}

fn default_store() -> String {
    "auto".to_string()
}

#[utoipa::path(
    post,
    path = "/api/v0/images/from-url",
    tag = "images",
    params(
        ("url" = String, Query, description = "URL of the image to download and upload"),
        ("store" = Option<String>, Query, description = "Storage behavior: '0' (temporary), '1' (permanent), 'auto' (default)")
    ),
    responses(
        (status = 200, description = "Image uploaded successfully from URL", body = UploadFromUrlResponse),
        (status = 400, description = "Invalid input or URL", body = ErrorResponse),
        (status = 413, description = "File too large", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state, query),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        url = %query.url,
        store = %query.store,
        operation = "upload_image_from_url"
    )
)]
pub async fn upload_image_from_url(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(query): Query<UploadFromUrlQuery>,
) -> Result<Response, HttpAppError> {
    let url = query.url.trim();
    if url.is_empty() {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "URL parameter is required".to_string(),
        )));
    }

    let parsed_url = reqwest::Url::parse(url)
        .map_err(|_| AppError::InvalidInput(format!("Invalid URL format: {}", url)))?;

    // Only allow HTTP/HTTPS
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Only HTTP and HTTPS URLs are allowed".to_string(),
        )));
    }

    // SSRF protection: check private IPs, localhost, DNS rebinding, and allowlist
    let allowlist = state.config.url_upload_allowlist();

    validate_url_for_ssrf(url, false, allowlist)
        .await
        .map_err(|e| {
            tracing::warn!(url = %url, error = %e, "SSRF validation failed");
            AppError::InvalidInput(format!("URL validation failed: {}", e))
        })?;

    tracing::info!(
        url = %url,
        tenant_id = %tenant_ctx.tenant_id,
        "Downloading image from URL (SSRF validated)"
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let response = client.get(url).send().await.map_err(|e| {
        tracing::error!(error = %e, url = %url, "Failed to download from URL");
        AppError::InvalidInput(format!("Failed to download from URL: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "URL returned status code: {}",
            response.status()
        ))));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("application/octet-stream")
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .trim()
        .to_string();

    let file_data = response
        .bytes()
        .await
        .map_err(|e| AppError::InvalidInput(format!("Failed to read response body: {}", e)))?
        .to_vec();

    validate_file_size(file_data.len(), state.media.image_max_file_size)?;
    validate_content_type(&content_type, &state.media.image_allowed_content_types)?;

    let original_filename = parsed_url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|name| !name.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "image.jpg".to_string());

    let extension =
        validate_file_extension(&original_filename, &state.media.image_allowed_extensions)?;
    validate_extension_content_type_match(&original_filename, &content_type)?;

    let (store_permanently, expires_at) =
        parse_store_parameter(&query.store, state.config.auto_store_enabled())?;
    let store_behavior = query.store.clone();

    if state.security.clamav_enabled {
        #[cfg(feature = "clamav")]
        if let Some(ref clamav) = state.security.clamav {
            tracing::debug!("Scanning file from URL with ClamAV");
            let scan_result = clamav.scan_bytes(&file_data).await;
            handle_clamav_scan(scan_result, &original_filename).await?;
        }
    }

    let mut processed_data = file_data.clone();
    if state.media.remove_exif {
        let file_data_for_exif = processed_data.clone();
        processed_data =
            tokio::task::spawn_blocking(move || ImageProcessor::remove_exif(&file_data_for_exif))
                .await
                .map_err(|e| AppError::Internal(format!("Failed to process image: {}", e)))?
                .map_err(|e: anyhow::Error| {
                    AppError::Internal(format!("Failed to remove EXIF data: {}", e))
                })?;
    }

    let file_data_clone = processed_data.clone();
    let dimensions = tokio::task::spawn_blocking(move || {
        ImageProcessor::validate_and_get_dimensions(&file_data_clone)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Failed to process image: {}", e)))?
    .map_err(|e: anyhow::Error| AppError::InvalidInput(format!("Invalid image file: {}", e)))?;

    let file_uuid = Uuid::new_v4();
    let safe_original_filename =
        sanitize_filename(&original_filename).map_err(HttpAppError::from)?;
    let uuid_filename = format!("{}.{}", file_uuid, extension);

    let file_size = processed_data.len();

    tracing::info!(
        file_uuid = %file_uuid,
        original_filename = %safe_original_filename,
        file_size = file_size,
        source_url = %url,
        "Processing image upload from URL"
    );

    let (storage_key, storage_url) = state
        .media
        .storage
        .upload(
            tenant_ctx.tenant_id,
            &uuid_filename,
            &content_type,
            processed_data.clone(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, file_uuid = %file_uuid, "Failed to upload to storage");
            AppError::S3(format!("Failed to upload file: {}", e))
        })?;

    let (width, height) = dimensions.unwrap_or((0, 0));

    let image = state
        .media
        .repository
        .create_image_from_storage(
            tenant_ctx.tenant_id,
            file_uuid,
            uuid_filename,
            safe_original_filename,
            content_type,
            file_size as i64,
            Some(width as i32),
            Some(height as i32),
            store_behavior,
            store_permanently,
            expires_at,
            None, // folder_id
            storage_key.clone(),
            storage_url.clone(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, file_uuid = %file_uuid, "Failed to save to database, initiating storage cleanup");

            // Best-effort cleanup: failure is logged but not propagated to caller
            let storage = state.media.storage.clone();
            let cleanup_key = storage_key.clone();
            tokio::spawn(async move {
                if let Err(cleanup_err) = storage.delete(&cleanup_key).await {
                    tracing::error!(
                        error = %cleanup_err,
                        storage_key = %cleanup_key,
                        "Failed to cleanup storage file after DB error"
                    );
                }
            });

            AppError::Internal("Failed to save image metadata".to_string())
        })?;

    let webhook_data = WebhookDataInfo {
        id: image.id,
        filename: image.original_filename.clone(),
        url: image.storage_url().to_string(),
        content_type: image.content_type.clone(),
        file_size: image.file_size,
        entity_type: String::from("image"),
        uploaded_at: Some(image.uploaded_at),
        deleted_at: None,
        stored_at: if image.store_permanently {
            Some(image.uploaded_at)
        } else {
            None
        },
        processing_status: None,
        error_message: None,
    };

    let webhook_initiator = WebhookInitiatorInfo {
        initiator_type: String::from("url_upload"),
        id: image.tenant_id,
    };

    let webhook_service = state.webhooks.webhook_service.clone();
    let tenant_id = image.tenant_id;
    tokio::spawn(async move {
        if let Err(e) = webhook_service
            .trigger_event(
                tenant_id,
                WebhookEventType::FileUploaded,
                webhook_data,
                webhook_initiator,
            )
            .await
        {
            tracing::warn!(error = %e, "Failed to trigger webhook for URL upload");
        }
    });

    let response = ImageResponse {
        id: image.id,
        url: image.storage_url().to_string(),
        filename: image.original_filename,
        content_type: image.content_type,
        file_size: image.file_size,
        width: image.width,
        height: image.height,
        uploaded_at: image.uploaded_at,
        store_behavior: image.store_behavior,
        store_permanently: image.store_permanently,
        expires_at: image.expires_at,
        folder_id: None,
        folder_name: None,
    };

    Ok((
        StatusCode::CREATED,
        Json(UploadFromUrlResponse {
            message: format!("Image successfully uploaded from URL: {}", url),
            image: response,
        }),
    )
        .into_response())
}
