//! Chunked upload handlers for large file uploads
//!
//! This module contains planned features for resumable uploads

// This module contains planned features not yet fully implemented
#![allow(dead_code)]

use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{Duration, Utc};
use mindia_core::models::presigned_upload::{CompleteUploadRequest, CompleteUploadResponse};
use mindia_core::AppError;
use mindia_services::{CompletedMultipartUpload, CompletedPart};
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
    let s3_config = state.s3.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "Chunked uploads with presigned URLs are only available when using S3 storage backend. Please use regular upload endpoints for local filesystem storage.".to_string()
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

    // Calculate chunk count
    let chunk_count = request.file_size.div_ceil(request.chunk_size) as i32;

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
    for i in 0..chunk_count {
        let chunk_s3_key = format!("{}.chunk.{}", base_s3_key, i);
        let presigned_url = s3_config
            .service
            .generate_presigned_put_url(
                &s3_config.bucket,
                &chunk_s3_key,
                &request.content_type,
                expires_in_seconds,
            )
            .await
            .map_err(|e| AppError::S3(format!("Failed to generate presigned URL: {}", e)))?;

        chunk_urls.push(ChunkUrl {
            index: i,
            url: presigned_url,
            s3_key: chunk_s3_key,
        });
    }

    // Create upload session in database
    let upload_repo = mindia_db::PresignedUploadRepository::new(state.db.pool.clone());
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
    // Get upload session
    let upload_repo = mindia_db::PresignedUploadRepository::new(state.db.pool.clone());
    let session = upload_repo
        .get_upload_session(tenant_ctx.tenant_id, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Upload session not found: {}", session_id)))?;

    // Verify chunk index matches
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

    // Get chunk size (we'll use chunk_size from session for now)
    let chunk_size = session.chunk_size.unwrap_or(0) as i64;

    // Record chunk upload
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
    // Get upload session
    let upload_repo = mindia_db::PresignedUploadRepository::new(state.db.pool.clone());
    let session = upload_repo
        .get_upload_session(tenant_ctx.tenant_id, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Upload session not found: {}", session_id)))?;

    // Get uploaded chunks
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

    // Get upload session
    let upload_repo = mindia_db::PresignedUploadRepository::new(state.db.pool.clone());
    let session = upload_repo
        .get_upload_session(tenant_ctx.tenant_id, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Upload session not found: {}", session_id)))?;

    // Check if all chunks are uploaded
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

    // Assemble chunks into final file using S3 multipart upload with copy_part
    // This performs server-side copying without downloading chunks to memory

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

    // Create multipart upload
    let create_result = s3_config
        .service
        .client()
        .create_multipart_upload()
        .bucket(&s3_config.bucket)
        .key(&final_s3_key)
        .content_type(&session.content_type)
        .send()
        .await
        .map_err(|e| AppError::S3(format!("Failed to create multipart upload: {}", e)))?;

    let upload_id = create_result
        .upload_id()
        .ok_or_else(|| AppError::S3("No upload ID returned from S3".to_string()))?;

    // Copy each chunk as a part using copy_part (server-side, no download)
    let mut parts = Vec::new();
    let mut part_number = 1u32;
    let mut total_bytes = 0u64;

    for chunk in chunks.iter() {
        // Get chunk size for range copy
        let chunk_size = chunk.size;
        total_bytes += chunk_size as u64;

        // URL-encode the copy source per AWS S3 API requirements
        let encoded_key = urlencoding::encode(&chunk.s3_key);
        let copy_source = format!("{}/{}", s3_config.bucket, encoded_key);

        // Copy chunk as a part (server-side operation)
        let copy_part_result = s3_config
            .service
            .client()
            .upload_part_copy()
            .bucket(&s3_config.bucket)
            .key(&final_s3_key)
            .upload_id(upload_id)
            .part_number(part_number as i32)
            .copy_source(&copy_source)
            .send()
            .await
            .map_err(|e| {
                AppError::S3(format!(
                    "Failed to copy chunk {} as part {}: {}",
                    chunk.s3_key, part_number, e
                ))
            })?;

        let etag = copy_part_result
            .copy_part_result()
            .and_then(|r| r.e_tag())
            .ok_or_else(|| AppError::S3(format!("No ETag returned for part {}", part_number)))?
            .to_string();

        let completed_part = CompletedPart::builder()
            .part_number(part_number as i32)
            .e_tag(etag)
            .build();

        parts.push(completed_part);
        part_number += 1;
    }

    // Complete multipart upload
    let completed_parts = CompletedMultipartUpload::builder()
        .set_parts(Some(parts))
        .build();

    s3_config
        .service
        .client()
        .complete_multipart_upload()
        .bucket(&s3_config.bucket)
        .key(&final_s3_key)
        .upload_id(upload_id)
        .multipart_upload(completed_parts)
        .send()
        .await
        .map_err(|e| AppError::S3(format!("Failed to complete multipart upload: {}", e)))?;

    tracing::info!(
        session_id = %session_id,
        final_key = %final_s3_key,
        total_bytes = total_bytes,
        parts = part_number - 1,
        "Chunked upload assembled using server-side multipart copy"
    );

    // Delete chunk files from storage (cleanup)
    for chunk in chunks.iter() {
        let _ = state.media.storage.delete(&chunk.s3_key).await;
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

    // Create media record
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
                    session.file_size,
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
                    session.file_size,
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
                    session.file_size,
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
                    session.file_size,
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
        file_size: session.file_size,
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
        file_size: session.file_size,
        uploaded_at: Utc::now(),
    }))
}
