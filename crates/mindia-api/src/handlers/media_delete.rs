use crate::auth::models::TenantContext;
use crate::error::{error_response_with_event, ErrorResponse, HttpAppError};
use crate::middleware::audit;
use crate::middleware::WideEventCtx;
use crate::state::AppState;
use crate::utils::ip_extraction::extract_client_ip;
use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use mindia_core::models::{
    Media, MediaType, WebhookDataInfo, WebhookEventType, WebhookInitiatorInfo,
};
use mindia_core::AppError;
use std::sync::Arc;
use uuid::Uuid;

/// Helper function to delete video HLS files
/// Uses Storage trait abstraction to support both S3 and local filesystem storage
pub(crate) async fn delete_video_hls_files(
    storage: &Arc<dyn mindia_services::Storage>,
    media_id: Uuid,
    hls_master_playlist: Option<&String>,
) {
    if let Some(master_playlist) = hls_master_playlist {
        if let Err(e) = storage.delete(master_playlist).await {
            tracing::error!(
                error = %e,
                storage_key = %master_playlist,
                "Failed to delete master playlist from storage"
            );
        }

        let base_key = format!("uploads/{}", media_id);
        let variants = vec!["360p", "480p", "720p", "1080p"];

        for variant in variants {
            let variant_playlist = format!("{}/{}/index.m3u8", base_key, variant);
            if let Err(e) = storage.delete(&variant_playlist).await {
                tracing::debug!(
                    error = %e,
                    storage_key = %variant_playlist,
                    "Variant playlist not found or already deleted"
                );
            }

            // Max 999 segments per variant
            for i in 0..999 {
                let segment_key = format!("{}/{}/segment_{:03}.ts", base_key, variant, i);
                match storage.delete(&segment_key).await {
                    Ok(_) => {}
                    Err(_) => break, // No more segments in this variant
                }
            }
        }
    }
}

/// Helper function to delete audio embeddings
pub(crate) async fn delete_audio_embeddings(
    embedding_repository: &mindia_db::EmbeddingRepository,
    tenant_id: Uuid,
    audio_id: Uuid,
) {
    if let Err(e) = embedding_repository
        .delete_embedding(tenant_id, audio_id)
        .await
    {
        tracing::warn!(
            error = %e,
            audio_id = %audio_id,
            tenant_id = %tenant_id,
            "Failed to delete audio embeddings"
        );
    }
}

#[utoipa::path(
    delete,
    path = "/api/v0/media/{id}",
    tag = "media",
    params(
        ("id" = Uuid, Path, description = "Media ID")
    ),
    responses(
        (status = 204, description = "Media deleted successfully"),
        (status = 404, description = "Media not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    skip(state, wide_event, request),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        media_id = %id,
        operation = "delete_media"
    )
)]
pub async fn delete_media(
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    mut wide_event: WideEventCtx,
    request: Request,
) -> axum::response::Response {
    // Enrich wide event with business context
    wide_event.0.with_business_context(|ctx| {
        ctx.media_id = Some(id);
        ctx.operation = Some("delete".to_string());
    });
    // Fetch media via repository to get type and HLS/embedding info for cleanup
    let media_result: Media = match state.media.repository.get(tenant_ctx.tenant_id, id).await {
        Ok(Some(media)) => media,
        Ok(None) => {
            return error_response_with_event(
                HttpAppError::from(AppError::NotFound("Media not found".to_string())),
                wide_event.0,
            );
        }
        Err(e) => {
            tracing::debug!(error = %e, media_id = %id, "Database error fetching media for deletion");
            return error_response_with_event(HttpAppError::from(e), wide_event.0);
        }
    };

    let media_type = media_result.media_type();
    let hls_master_playlist = match &media_result {
        Media::Video(v) => v.hls_master_playlist.clone(),
        _ => None,
    };

    // Type-specific cleanup BEFORE deleting
    match media_type {
        MediaType::Video => {
            delete_video_hls_files(&state.media.storage, id, hls_master_playlist.as_ref()).await;
        }
        MediaType::Audio => {
            delete_audio_embeddings(&state.db.embedding_repository, tenant_ctx.tenant_id, id).await;
        }
        MediaType::Image | MediaType::Document => {}
    }

    // Extract client IP and user agent for audit logging
    let trusted_proxy_count = std::env::var("TRUSTED_PROXY_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let socket_addr = request.extensions().get::<std::net::SocketAddr>().copied();
    let client_ip = Some(extract_client_ip(
        request.headers(),
        socket_addr.as_ref(),
        trusted_proxy_count,
    ));

    // Delete from storage and database (MediaRepository.delete() queries the row again)
    if let Err(e) = state
        .media
        .repository
        .delete(tenant_ctx.tenant_id, id)
        .await
    {
        tracing::debug!(error = %e, media_id = %id, "Failed to delete media");

        // Check if it's a not found error (already deleted)
        if let AppError::Database(db_err) = &e {
            if db_err.to_string().contains("not found") {
                return error_response_with_event(
                    HttpAppError::from(AppError::NotFound("Media not found".to_string())),
                    wide_event.0,
                );
            }
        }

        return error_response_with_event(HttpAppError::from(e), wide_event.0);
    }

    // Log file deletion for audit
    let filename = match &media_result {
        mindia_core::models::Media::Image(img) => img.original_filename.clone(),
        mindia_core::models::Media::Video(vid) => vid.original_filename.clone(),
        mindia_core::models::Media::Audio(aud) => aud.original_filename.clone(),
        mindia_core::models::Media::Document(doc) => doc.original_filename.clone(),
    };
    audit::log_file_deleted(
        tenant_ctx.tenant_id,
        Some(tenant_ctx.user_id),
        id,
        filename,
        client_ip,
    );

    // Enrich wide event with media type
    let media_type_str = match media_type {
        MediaType::Image => "image",
        MediaType::Video => "video",
        MediaType::Audio => "audio",
        MediaType::Document => "document",
    };
    wide_event.0.with_business_context(|ctx| {
        ctx.media_type = Some(media_type_str.to_string());
    });

    // Prepare webhook data
    let (
        entity_type,
        original_filename,
        s3_url,
        content_type,
        file_size,
        uploaded_at,
        processing_status,
    ) = match &media_result {
        mindia_core::models::Media::Image(img) => (
            "image".to_string(),
            img.original_filename.clone(),
            img.storage_url().to_string(),
            img.content_type.clone(),
            img.file_size,
            Some(img.uploaded_at),
            None,
        ),
        mindia_core::models::Media::Video(vid) => (
            "video".to_string(),
            vid.original_filename.clone(),
            vid.storage_url().to_string(),
            vid.content_type.clone(),
            vid.file_size,
            Some(vid.uploaded_at),
            Some(vid.processing_status.to_string()),
        ),
        mindia_core::models::Media::Audio(aud) => (
            "audio".to_string(),
            aud.original_filename.clone(),
            aud.storage_url().to_string(),
            aud.content_type.clone(),
            aud.file_size,
            Some(aud.uploaded_at),
            None,
        ),
        mindia_core::models::Media::Document(doc) => (
            "document".to_string(),
            doc.original_filename.clone(),
            doc.storage_url().to_string(),
            doc.content_type.clone(),
            doc.file_size,
            Some(doc.uploaded_at),
            None,
        ),
    };

    let webhook_data = WebhookDataInfo {
        id,
        filename: original_filename,
        url: s3_url,
        content_type,
        file_size,
        entity_type,
        uploaded_at,
        deleted_at: Some(Utc::now()),
        stored_at: None,
        processing_status,
        error_message: None,
    };

    let webhook_initiator = WebhookInitiatorInfo {
        initiator_type: String::from("delete"),
        id: tenant_ctx.tenant_id,
    };

    // Fire webhook asynchronously (don't block on response)
    let webhook_service = state.webhooks.webhook_service.clone();
    let tenant_id = tenant_ctx.tenant_id;
    tokio::spawn(async move {
        if let Err(e) = webhook_service
            .trigger_event(
                tenant_id,
                WebhookEventType::FileDeleted,
                webhook_data,
                webhook_initiator,
            )
            .await
        {
            tracing::warn!(
                error = %e,
                tenant_id = %tenant_id,
                "Failed to trigger webhook for media deletion"
            );
        }
    });

    // Return 204 No Content with enriched event
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .extensions_mut()
        .insert(crate::middleware::wide_event::WideEventExtension(
            wide_event.0,
        ));
    response
}
