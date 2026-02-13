use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use chrono::{Duration, Utc};
use mindia_core::models::presigned_upload::{
    CompleteUploadRequest, CompleteUploadResponse, PresignedUploadRequest, PresignedUploadResponse,
};
use mindia_core::AppError;
use std::sync::Arc;
use uuid::Uuid;

/// Generate a presigned URL for direct S3 upload
#[utoipa::path(
    post,
    path = "/api/v0/uploads/presigned",
    tag = "uploads",
    request_body = PresignedUploadRequest,
    responses(
        (status = 200, description = "Presigned URL generated", body = PresignedUploadResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state, request),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        media_type = %request.media_type,
        store = %request.store,
        operation = "generate_presigned_url"
    )
)]
pub async fn generate_presigned_url(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(request): Json<PresignedUploadRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Presigned URLs are only available for S3 storage backend
    let s3_config = state.s3.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "Presigned URLs are only available when using S3 storage backend. Please use regular upload endpoints for local filesystem storage.".to_string()
        )
    })?;

    // Validate media type
    let media_type = request.media_type.to_lowercase();
    if !["image", "video", "audio", "document"].contains(&media_type.as_str()) {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "Invalid media_type: {}. Must be one of: image, video, audio, document",
            media_type
        ))));
    }

    // Validate store parameter
    let store_behavior = request.store.to_lowercase();
    if !["0", "1", "auto"].contains(&store_behavior.as_str()) {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Invalid store parameter. Must be '0', '1', or 'auto'".to_string(),
        )));
    }

    let _store_permanently = match store_behavior.as_str() {
        "1" => true,
        "0" => false,
        "auto" => state.config.auto_store_enabled(),
        _ => state.config.auto_store_enabled(),
    };

    // Generate upload ID
    let upload_id = Uuid::new_v4();
    let file_id = Uuid::new_v4();

    // Generate S3 key
    let extension = request
        .filename
        .split('.')
        .next_back()
        .unwrap_or("bin")
        .to_lowercase();
    let unique_filename = format!("{}.{}", file_id, extension);
    let s3_key = format!("uploads/{}", unique_filename);

    // Generate presigned URL (expires in 15 minutes)
    let expires_in_seconds = 15 * 60;
    let expires_at = Utc::now() + Duration::seconds(expires_in_seconds as i64);

    let presigned_url = s3_config
        .service
        .generate_presigned_put_url(
            &s3_config.bucket,
            &s3_key,
            &request.content_type,
            expires_in_seconds,
        )
        .await
        .map_err(|e| AppError::S3(format!("Failed to generate presigned URL: {}", e)))?;

    // Create upload session in database
    let upload_repo = mindia_db::PresignedUploadRepository::new(state.db.pool.clone());
    upload_repo
        .create_upload_session(
            tenant_ctx.tenant_id,
            upload_id,
            request.filename.clone(),
            request.content_type.clone(),
            request.file_size,
            media_type.clone(),
            s3_key.clone(),
            store_behavior.clone(),
            expires_at,
            request.metadata.clone(),
            None, // chunk_size (for non-chunked uploads)
            None, // chunk_count (for non-chunked uploads)
        )
        .await?;

    tracing::info!(
        upload_id = %upload_id,
        tenant_id = %tenant_ctx.tenant_id,
        filename = %request.filename,
        "Generated presigned URL for direct upload"
    );

    Ok(Json(PresignedUploadResponse {
        upload_id,
        presigned_url,
        s3_key,
        expires_at,
        fields: None,
    }))
}

/// Complete a direct upload after file has been uploaded to S3
#[utoipa::path(
    post,
    path = "/api/v0/uploads/complete",
    tag = "uploads",
    request_body = CompleteUploadRequest,
    responses(
        (status = 200, description = "Upload completed", body = CompleteUploadResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 404, description = "Upload session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state, request),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        upload_id = %request.upload_id,
        operation = "complete_upload"
    )
)]
pub async fn complete_upload(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(request): Json<CompleteUploadRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Presigned uploads are only available for S3 storage backend
    let s3_config = state.s3.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "Presigned upload completion is only available when using S3 storage backend."
                .to_string(),
        )
    })?;

    // Get upload session
    let upload_repo = mindia_db::PresignedUploadRepository::new(state.db.pool.clone());
    let session = upload_repo
        .get_upload_session(tenant_ctx.tenant_id, request.upload_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Upload session not found: {}", request.upload_id))
        })?;

    // Check if session is still valid
    if session.expires_at < Utc::now() {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Upload session has expired".to_string(),
        )));
    }

    if session.status != "pending" {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "Upload session is not pending (status: {})",
            session.status
        ))));
    }

    // Verify file exists in storage (for presigned uploads, this should be S3)
    let file_exists = state
        .media
        .storage
        .exists(&session.s3_key)
        .await
        .map_err(|e| AppError::S3(format!("Failed to check file existence: {}", e)))?;

    if !file_exists {
        return Err(HttpAppError::from(AppError::NotFound(format!(
            "File not found in storage: {}",
            session.s3_key
        ))));
    }

    // Get file size from session
    let file_size = session.file_size;

    // Generate storage URL (S3-compatible)
    let s3_url = if let Some(ref endpoint) = s3_config.endpoint_url {
        // For S3-compatible providers, construct URL from endpoint
        // Remove trailing slash if present
        let base_url = endpoint.trim_end_matches('/');
        // Use path-style for compatibility: {endpoint}/{bucket}/{key}
        format!("{}/{}/{}", base_url, s3_config.bucket, session.s3_key)
    } else {
        // Standard AWS S3 URL format
        format!(
            "https://{}.s3.{}.amazonaws.com/{}",
            s3_config.bucket, s3_config.region, session.s3_key
        )
    };

    // Resolve store_permanently
    let store_permanently = match session.store_behavior.as_str() {
        "1" => true,
        "0" => false,
        "auto" => state.config.auto_store_enabled(),
        _ => state.config.auto_store_enabled(),
    };

    // Calculate expiration time
    let expires_at = if !store_permanently {
        Some(Utc::now() + Duration::hours(24))
    } else {
        None
    };

    // Extract file ID from S3 key (format: uploads/{uuid}.{ext})
    // The filename in the session is the original filename, but the S3 key has the UUID
    let file_id = session
        .s3_key
        .split('/')
        .next_back()
        .and_then(|f| f.split('.').next())
        .and_then(|id| Uuid::parse_str(id).ok())
        .unwrap_or_else(|| {
            // If we can't parse, generate a new UUID
            let new_id = Uuid::new_v4();
            tracing::warn!(
                s3_key = %session.s3_key,
                "Could not extract UUID from S3 key, generated new ID"
            );
            new_id
        });

    // Create media record based on type
    let media_id = match session.media_type.as_str() {
        "image" => {
            // For images, we need to download and process to get dimensions
            // For now, create without dimensions (they can be updated later)
            let image = state
                .media
                .repository
                .create_media(
                    tenant_ctx.tenant_id,
                    file_id,
                    mindia_core::models::MediaType::Image,
                    session.filename.clone(),
                    session.s3_key.clone(),
                    s3_url.clone(),
                    session.content_type.clone(),
                    file_size,
                    None, // width
                    None, // height
                    None, // duration
                    None, // processing_status
                    None, // hls_master_playlist
                    None, // variants
                    None, // bitrate
                    None, // sample_rate
                    None, // channels
                    None, // page_count
                    session.store_behavior.clone(),
                    store_permanently,
                    expires_at,
                    None,                                  // folder_id
                    request.metadata.or(session.metadata), // metadata
                )
                .await?;
            image.id()
        }
        "video" => {
            let video = state
                .media
                .repository
                .create_media(
                    tenant_ctx.tenant_id,
                    file_id,
                    mindia_core::models::MediaType::Video,
                    session.filename.clone(),
                    session.s3_key.clone(),
                    s3_url.clone(),
                    session.content_type.clone(),
                    file_size,
                    None,                                                 // width
                    None,                                                 // height
                    None,                                                 // duration
                    Some(mindia_core::models::ProcessingStatus::Pending), // processing_status
                    None,                                                 // hls_master_playlist
                    None,                                                 // variants
                    None,                                                 // bitrate
                    None,                                                 // sample_rate
                    None,                                                 // channels
                    None,                                                 // page_count
                    session.store_behavior.clone(),
                    store_permanently,
                    expires_at,
                    None,                                  // folder_id
                    request.metadata.or(session.metadata), // metadata
                )
                .await?;
            video.id()
        }
        "audio" => {
            let audio = state
                .media
                .repository
                .create_media(
                    tenant_ctx.tenant_id,
                    file_id,
                    mindia_core::models::MediaType::Audio,
                    session.filename.clone(),
                    session.s3_key.clone(),
                    s3_url.clone(),
                    session.content_type.clone(),
                    file_size,
                    None, // width
                    None, // height
                    None, // duration
                    None, // processing_status
                    None, // hls_master_playlist
                    None, // variants
                    None, // bitrate
                    None, // sample_rate
                    None, // channels
                    None, // page_count
                    session.store_behavior.clone(),
                    store_permanently,
                    expires_at,
                    None,                                  // folder_id
                    request.metadata.or(session.metadata), // metadata
                )
                .await?;
            audio.id()
        }
        "document" => {
            let document = state
                .media
                .repository
                .create_media(
                    tenant_ctx.tenant_id,
                    file_id,
                    mindia_core::models::MediaType::Document,
                    session.filename.clone(),
                    session.s3_key.clone(),
                    s3_url.clone(),
                    session.content_type.clone(),
                    file_size,
                    None, // width
                    None, // height
                    None, // duration
                    None, // processing_status
                    None, // hls_master_playlist
                    None, // variants
                    None, // bitrate
                    None, // sample_rate
                    None, // channels
                    None, // page_count
                    session.store_behavior.clone(),
                    store_permanently,
                    expires_at,
                    None,                                  // folder_id
                    request.metadata.or(session.metadata), // metadata
                )
                .await?;
            document.id()
        }
        _ => {
            return Err(HttpAppError::from(AppError::InvalidInput(format!(
                "Unsupported media type: {}",
                session.media_type
            ))));
        }
    };

    // Mark upload session as completed
    upload_repo
        .mark_completed(request.upload_id, media_id)
        .await?;

    tracing::info!(
        upload_id = %request.upload_id,
        media_id = %media_id,
        tenant_id = %tenant_ctx.tenant_id,
        "Direct upload completed"
    );

    // Trigger webhook for file.uploaded event
    let webhook_data = mindia_core::models::WebhookDataInfo {
        id: media_id,
        filename: session.filename.clone(),
        url: s3_url.clone(),
        content_type: session.content_type.clone(),
        file_size,
        entity_type: session.media_type.clone(),
        uploaded_at: Some(Utc::now()),
        deleted_at: None,
        stored_at: if store_permanently {
            Some(Utc::now())
        } else {
            None
        },
        processing_status: None,
        error_message: None,
    };

    let webhook_initiator = mindia_core::models::WebhookInitiatorInfo {
        initiator_type: String::from("upload"),
        id: tenant_ctx.tenant_id,
    };

    // Fire webhook asynchronously
    let webhook_service = state.webhooks.webhook_service.clone();
    let tenant_id = tenant_ctx.tenant_id;
    tokio::spawn(async move {
        if let Err(e) = webhook_service
            .trigger_event(
                tenant_id,
                mindia_core::models::WebhookEventType::FileUploaded,
                webhook_data,
                webhook_initiator,
            )
            .await
        {
            tracing::warn!(
                error = %e,
                tenant_id = %tenant_id,
                "Failed to trigger webhook for direct upload"
            );
        }
    });

    Ok(Json(CompleteUploadResponse {
        id: media_id,
        url: s3_url,
        content_type: session.content_type,
        file_size,
        uploaded_at: Utc::now(),
    }))
}
