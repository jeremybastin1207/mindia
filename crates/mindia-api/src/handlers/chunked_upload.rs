//! Chunked upload handlers for large file uploads.
//!
//! Resumable uploads via presigned chunk URLs; completion assembles chunks and creates the media record.

use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use crate::utils::upload::handle_clamav_scan;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{Duration, Utc};
use mindia_core::models::presigned_upload::{CompleteUploadRequest, CompleteUploadResponse};
use mindia_core::models::MediaType;
use mindia_core::{AppError, StorageBackend};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Request to start a chunked upload
#[derive(Debug, Deserialize, ToSchema)]
pub struct StartChunkedUploadRequest {
    /// Original filename
    pub filename: String,
    /// Content type (MIME type)
    pub content_type: String,
    /// Total file size in bytes
    pub file_size: u64,
    /// Media type (image, video, audio, document)
    pub media_type: String,
    /// Size of each chunk in bytes
    pub chunk_size: u64,
    /// Storage behavior: "0" (temporary), "1" (permanent), "auto"
    #[serde(default = "default_store")]
    pub store: String,
    /// Optional custom metadata
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_store() -> String {
    "auto".to_string()
}

/// Response for starting chunked upload
#[derive(Debug, Serialize, ToSchema)]
pub struct StartChunkedUploadResponse {
    /// Upload session ID
    pub session_id: Uuid,
    /// Total number of chunks
    pub chunk_count: i32,
    /// Chunk size in bytes
    pub chunk_size: u64,
    /// Presigned URLs for each chunk (indexed by chunk index)
    pub chunk_urls: Vec<ChunkUrl>,
}

/// Presigned URL for a single chunk
#[derive(Debug, Serialize, ToSchema)]
pub struct ChunkUrl {
    /// Chunk index (0-based)
    pub index: i32,
    /// Presigned URL for uploading this chunk
    pub url: String,
    /// S3 key for this chunk
    pub s3_key: String,
}

/// Request to upload a single chunk
#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadChunkRequest {
    /// Chunk index (0-based)
    pub chunk_index: i32,
}

/// Response for chunk upload progress
#[derive(Debug, Serialize, ToSchema)]
pub struct ChunkedUploadProgressResponse {
    /// Session ID
    pub session_id: Uuid,
    /// Total file size
    pub total_size: u64,
    /// Bytes uploaded so far
    pub uploaded_size: u64,
    /// Number of chunks uploaded
    pub chunks_uploaded: i32,
    /// Total number of chunks
    pub total_chunks: i32,
    /// Upload progress percentage (0-100)
    pub progress_percent: f64,
    /// Session status
    pub status: String,
}

/// Start a chunked upload session
#[utoipa::path(
    post,
    path = "/api/v0/uploads/chunked/start",
    tag = "uploads",
    request_body = StartChunkedUploadRequest,
    responses(
        (status = 200, description = "Chunked upload started", body = StartChunkedUploadResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn start_chunked_upload(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Json(request): Json<StartChunkedUploadRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Chunked uploads with presigned URLs are only available for S3 storage backend
    if state.media.storage.backend_type() != StorageBackend::S3 {
        return Err(HttpAppError::from(AppError::BadRequest(
            "Chunked uploads with presigned URLs are only available when using S3 storage backend. Please use regular upload endpoints for local filesystem storage.".to_string(),
        )));
    }

    let media_type = request.media_type.to_lowercase();
    if !["image", "video", "audio", "document"].contains(&media_type.as_str()) {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "Invalid media_type: {}. Must be one of: image, video, audio, document",
            media_type
        ))));
    }

    let store_behavior = request.store.to_lowercase();
    if !["0", "1", "auto"].contains(&store_behavior.as_str()) {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Invalid store parameter. Must be '0', '1', or 'auto'".to_string(),
        )));
    }

    // Max file size and chunk count limits (align with multipart limits per media type)
    const MAX_CHUNK_COUNT: i32 = 10_000;
    let media_type_enum = match media_type.as_str() {
        "image" => MediaType::Image,
        "video" => MediaType::Video,
        "audio" => MediaType::Audio,
        "document" => MediaType::Document,
        _ => {
            return Err(HttpAppError::from(AppError::InvalidInput(
                "Invalid media type: must be image, video, audio, or document".to_string(),
            )));
        }
    };
    let limits = state.media.limits_for(media_type_enum);
    if request.file_size > limits.max_file_size as u64 {
        return Err(HttpAppError::from(AppError::PayloadTooLarge(format!(
            "File size exceeds maximum allowed for {} ({} MB)",
            media_type,
            limits.max_file_size / 1024 / 1024
        ))));
    }
    if request.chunk_size == 0 {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "chunk_size must be greater than 0".to_string(),
        )));
    }
    let chunk_count = request.file_size.div_ceil(request.chunk_size) as i32;
    if chunk_count > MAX_CHUNK_COUNT {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "Chunk count {} exceeds maximum {}; use a larger chunk_size",
            chunk_count, MAX_CHUNK_COUNT
        ))));
    }

    // Generate session ID and file ID
    let session_id = Uuid::new_v4();
    let file_id = Uuid::new_v4();

    // Generate base S3 key
    let _extension = request
        .filename
        .split('.')
        .next_back()
        .unwrap_or("bin")
        .to_lowercase();
    let base_s3_key = format!("uploads/chunked/{}/{}", session_id, file_id);

    // Generate presigned URLs for all chunks
    let expires_in_seconds = 15 * 60; // 15 minutes
    let expires_at = Utc::now() + Duration::seconds(expires_in_seconds as i64);

    let mut chunk_urls = Vec::new();
    let expires_in = std::time::Duration::from_secs(expires_in_seconds);
    for i in 0..chunk_count {
        let chunk_s3_key = format!("{}.chunk.{}", base_s3_key, i);
        let presigned_url = state
            .media
            .storage
            .presigned_put_url(&chunk_s3_key, &request.content_type, expires_in)
            .await
            .map_err(|e| AppError::S3(format!("Failed to generate presigned URL: {}", e)))?;

        chunk_urls.push(ChunkUrl {
            index: i,
            url: presigned_url,
            s3_key: chunk_s3_key,
        });
    }

    let upload_repo = &state.db.presigned_upload_repository;
    upload_repo
        .create_upload_session(
            tenant_ctx.tenant_id,
            session_id,
            request.filename.clone(),
            request.content_type.clone(),
            request.file_size,
            media_type.clone(),
            base_s3_key.clone(), // Store base key, final key will be set on completion
            store_behavior.clone(),
            expires_at,
            request.metadata.clone(),
            Some(request.chunk_size),
            Some(chunk_count),
        )
        .await?;

    tracing::info!(
        session_id = %session_id,
        tenant_id = %tenant_ctx.tenant_id,
        filename = %request.filename,
        chunk_count = chunk_count,
        "Started chunked upload session"
    );

    Ok(Json(StartChunkedUploadResponse {
        session_id,
        chunk_count,
        chunk_size: request.chunk_size,
        chunk_urls,
    }))
}

/// Record a chunk upload completion
#[utoipa::path(
    put,
    path = "/api/v0/uploads/chunked/{session_id}/chunk/{chunk_index}",
    tag = "uploads",
    params(
        ("session_id" = Uuid, Path, description = "Upload session ID"),
        ("chunk_index" = i32, Path, description = "Chunk index (0-based)")
    ),
    request_body = UploadChunkRequest,
    responses(
        (status = 200, description = "Chunk uploaded successfully"),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn record_chunk_upload(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path((session_id, chunk_index)): Path<(Uuid, i32)>,
    Json(request): Json<UploadChunkRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    let upload_repo = &state.db.presigned_upload_repository;
    let session = upload_repo
        .get_upload_session(tenant_ctx.tenant_id, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Upload session not found: {}", session_id)))?;

    if request.chunk_index != chunk_index {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "Chunk index mismatch: path has {}, body has {}",
            chunk_index, request.chunk_index
        ))));
    }

    // Calculate chunk S3 key
    let chunk_s3_key = format!("{}.chunk.{}", session.s3_key, chunk_index);

    // Verify chunk exists in S3
    let chunk_exists = state
        .media
        .storage
        .exists(&chunk_s3_key)
        .await
        .map_err(|e| AppError::S3(format!("Failed to check chunk existence: {}", e)))?;

    if !chunk_exists {
        return Err(HttpAppError::from(AppError::NotFound(format!(
            "Chunk not found in S3: {}",
            chunk_s3_key
        ))));
    }

    let chunk_size = session.chunk_size.unwrap_or(0) as i64;

    upload_repo
        .record_chunk(session_id, chunk_index, chunk_s3_key, chunk_size)
        .await?;

    // Update session status to uploading if it's still pending
    if session.status == "pending" {
        upload_repo.update_status(session_id, "uploading").await?;
    }

    tracing::info!(
        session_id = %session_id,
        chunk_index = chunk_index,
        "Chunk upload recorded"
    );

    Ok(StatusCode::OK)
}

/// Get upload progress
#[utoipa::path(
    get,
    path = "/api/v0/uploads/chunked/{session_id}/progress",
    tag = "uploads",
    params(
        ("session_id" = Uuid, Path, description = "Upload session ID")
    ),
    responses(
        (status = 200, description = "Upload progress", body = ChunkedUploadProgressResponse),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_chunked_upload_progress(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let upload_repo = &state.db.presigned_upload_repository;
    let session = upload_repo
        .get_upload_session(tenant_ctx.tenant_id, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Upload session not found: {}", session_id)))?;

    let chunks = upload_repo.get_chunks(session_id).await?;

    let chunks_uploaded = chunks.len() as i32;
    let total_chunks = session.chunk_count.unwrap_or(0);
    let uploaded_size = session.uploaded_size as u64;
    let total_size = session.file_size as u64;
    let progress_percent = if total_size > 0 {
        (uploaded_size as f64 / total_size as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(ChunkedUploadProgressResponse {
        session_id,
        total_size,
        uploaded_size,
        chunks_uploaded,
        total_chunks,
        progress_percent,
        status: session.status,
    }))
}

/// Complete a chunked upload by assembling chunks
#[utoipa::path(
    post,
    path = "/api/v0/uploads/chunked/{session_id}/complete",
    tag = "uploads",
    params(
        ("session_id" = Uuid, Path, description = "Upload session ID")
    ),
    request_body = CompleteUploadRequest,
    responses(
        (status = 200, description = "Upload completed", body = CompleteUploadResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn complete_chunked_upload(
    tenant_ctx: TenantContext,
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
    Json(request): Json<CompleteUploadRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Chunked upload completion is only available for S3 storage backend
    // (since chunks were uploaded via presigned URLs which are S3-only)
    let s3_config = state.s3.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "Chunked upload completion is only available when using S3 storage backend."
                .to_string(),
        )
    })?;

    // Verify session ID matches
    if request.upload_id != session_id {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "Session ID mismatch: path has {}, body has {}",
            session_id, request.upload_id
        ))));
    }

    let upload_repo = &state.db.presigned_upload_repository;
    let session = upload_repo
        .get_upload_session(tenant_ctx.tenant_id, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Upload session not found: {}", session_id)))?;

    let chunks = upload_repo.get_chunks(session_id).await?;

    let expected_chunks = session.chunk_count.unwrap_or(0) as usize;
    if chunks.len() != expected_chunks {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "Not all chunks uploaded: {}/{} chunks uploaded",
            chunks.len(),
            expected_chunks
        ))));
    }

    // Verify all chunks exist in S3
    for chunk in &chunks {
        let exists = state
            .media
            .storage
            .exists(&chunk.s3_key)
            .await
            .map_err(|e| AppError::S3(format!("Failed to check chunk existence: {}", e)))?;
        if !exists {
            return Err(HttpAppError::from(AppError::NotFound(format!(
                "Chunk not found in S3: {}",
                chunk.s3_key
            ))));
        }
    }

    // Generate final S3 key
    let extension = session
        .filename
        .split('.')
        .next_back()
        .unwrap_or("bin")
        .to_lowercase();
    let file_id = session
        .s3_key
        .split('/')
        .next_back()
        .and_then(|f| f.split('.').next())
        .and_then(|id| Uuid::parse_str(id).ok())
        .unwrap_or_else(Uuid::new_v4);
    let final_filename = format!("{}.{}", file_id, extension);
    let final_s3_key = format!("uploads/{}", final_filename);

    // Download each chunk and concatenate in memory, then upload. Total size is bounded by
    // the per-media-type limit enforced in start_chunked_upload. For very large files,
    // consider streaming assembly (e.g. S3 multipart upload with part copies) in the future.
    let mut total_bytes = 0u64;
    let mut combined = Vec::new();

    for chunk in chunks.iter() {
        let bytes = state
            .media
            .storage
            .download(&chunk.s3_key)
            .await
            .map_err(|e| {
                AppError::S3(format!(
                    "Failed to download chunk {} from storage: {}",
                    chunk.s3_key, e
                ))
            })?;

        total_bytes += bytes.len() as u64;
        combined.extend_from_slice(&bytes);
    }

    // Use actual assembled size for DB; reject if it exceeds declared size
    let declared = session.file_size.max(0) as u64;
    if total_bytes > declared {
        return Err(HttpAppError::from(AppError::InvalidInput(format!(
            "Assembled file size {} bytes exceeds declared size {} bytes",
            total_bytes, session.file_size
        ))));
    }
    let file_size_for_record = total_bytes as i64;

    // Virus scan when ClamAV is enabled (chunked path skips server-side upload pipeline)
    if state.security.clamav_enabled {
        #[cfg(feature = "clamav")]
        if let Some(ref clamav) = state.security.clamav {
            let scan_result = clamav.scan_bytes(&combined).await;
            handle_clamav_scan(scan_result, &session.filename).await?;
        }
    }

    state
        .media
        .storage
        .upload_with_key(&final_s3_key, combined, &session.content_type)
        .await
        .map_err(|e| {
            AppError::S3(format!(
                "Failed to upload assembled object {} to storage: {}",
                final_s3_key, e
            ))
        })?;

    tracing::info!(
        session_id = %session_id,
        final_key = %final_s3_key,
        total_bytes = total_bytes,
        parts = chunks.len(),
        "Chunked upload assembled by concatenating chunks via storage backend"
    );

    for chunk in chunks.iter() {
        if let Err(e) = state.media.storage.delete(&chunk.s3_key).await {
            tracing::warn!(
                error = %e,
                storage_key = %chunk.s3_key,
                "Failed to delete chunk during cleanup"
            );
        }
    }

    // Generate storage URL (S3-compatible)
    let s3_url = if let Some(ref endpoint) = s3_config.endpoint_url {
        // For S3-compatible providers, construct URL from endpoint
        // Remove trailing slash if present
        let base_url = endpoint.trim_end_matches('/');
        // Use path-style for compatibility: {endpoint}/{bucket}/{key}
        format!("{}/{}/{}", base_url, s3_config.bucket, final_s3_key)
    } else {
        // Standard AWS S3 URL format
        format!(
            "https://{}.s3.{}.amazonaws.com/{}",
            s3_config.bucket, s3_config.region, final_s3_key
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

    let media_id = match session.media_type.as_str() {
        "image" => {
            let image = state
                .media
                .repository
                .create_media(
                    tenant_ctx.tenant_id,
                    file_id,
                    mindia_core::models::MediaType::Image,
                    session.filename.clone(),
                    final_s3_key.clone(),
                    s3_url.clone(),
                    session.content_type.clone(),
                    file_size_for_record,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    session.store_behavior.clone(),
                    store_permanently,
                    expires_at,
                    None,
                    request.metadata.or(session.metadata),
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
                    final_s3_key.clone(),
                    s3_url.clone(),
                    session.content_type.clone(),
                    file_size_for_record,
                    None,
                    None,
                    None,
                    Some(mindia_core::models::ProcessingStatus::Pending),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    session.store_behavior.clone(),
                    store_permanently,
                    expires_at,
                    None,
                    request.metadata.or(session.metadata),
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
                    final_s3_key.clone(),
                    s3_url.clone(),
                    session.content_type.clone(),
                    file_size_for_record,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    session.store_behavior.clone(),
                    store_permanently,
                    expires_at,
                    None,
                    request.metadata.or(session.metadata),
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
                    final_s3_key.clone(),
                    s3_url.clone(),
                    session.content_type.clone(),
                    file_size_for_record,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    session.store_behavior.clone(),
                    store_permanently,
                    expires_at,
                    None,
                    request.metadata.or(session.metadata),
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
    upload_repo.mark_completed(session_id, media_id).await?;

    tracing::info!(
        session_id = %session_id,
        media_id = %media_id,
        tenant_id = %tenant_ctx.tenant_id,
        "Chunked upload completed"
    );

    // Trigger webhook
    let webhook_data = mindia_core::models::WebhookDataInfo {
        id: media_id,
        filename: session.filename.clone(),
        url: s3_url.clone(),
        content_type: session.content_type.clone(),
        file_size: file_size_for_record,
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
                "Failed to trigger webhook for chunked upload"
            );
        }
    });

    Ok(Json(CompleteUploadResponse {
        id: media_id,
        url: s3_url,
        content_type: session.content_type,
        file_size: file_size_for_record,
        uploaded_at: Utc::now(),
    }))
}
