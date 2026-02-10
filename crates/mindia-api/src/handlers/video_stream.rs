use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use mindia_core::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v0/videos/{id}/stream/master.m3u8",
    tag = "videos",
    params(
        ("id" = Uuid, Path, description = "Video ID")
    ),
    responses(
        (status = 200, description = "HLS master playlist", content_type = "application/vnd.apple.mpegurl"),
        (status = 404, description = "Video not found or not processed", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn stream_master_playlist(
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpAppError> {
    let video = state
        .media
        .repository
        .get_video(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, video_id = %id, "Failed to fetch video");
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Video not found".to_string()))?;

    let master_playlist_key = video
        .hls_master_playlist
        .as_ref()
        .ok_or_else(|| AppError::NotFound("Video is still processing or failed".to_string()))?;

    let stream = state
        .media
        .storage
        .download_stream(master_playlist_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %master_playlist_key, "Failed to fetch playlist from storage");
            AppError::Internal(e.to_string())
        })?;

    let body_stream = stream.map(|result| {
        result.map_err(|e| std::io::Error::other(format!("Storage stream error: {}", e)))
    });

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from_stream(body_stream))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build master playlist response");
            AppError::Internal(e.to_string()).into()
        })
}

#[utoipa::path(
    get,
    path = "/api/v0/videos/{id}/stream/{variant}/index.m3u8",
    tag = "videos",
    params(
        ("id" = Uuid, Path, description = "Video ID"),
        ("variant" = String, Path, description = "Video quality variant (e.g., 360p, 480p, 720p, 1080p)")
    ),
    responses(
        (status = 200, description = "HLS variant playlist", content_type = "application/vnd.apple.mpegurl"),
        (status = 404, description = "Variant not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn stream_variant_playlist(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Path((id, variant)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, HttpAppError> {
    let video = state
        .media
        .repository
        .get_video(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, video_id = %id, "Failed to fetch video");
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Video not found".to_string()))?;

    if video.hls_master_playlist.is_none() {
        return Err(AppError::NotFound("Video is still processing or failed".to_string()).into());
    }

    let playlist_key = format!("uploads/{}/{}/index.m3u8", id, variant);

    let stream = state
        .media
        .storage
        .download_stream(&playlist_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %playlist_key, "Failed to fetch variant playlist from storage");
            AppError::NotFound("Variant not found".to_string())
        })?;

    let body_stream = stream.map(|result| {
        result.map_err(|e| std::io::Error::other(format!("Storage stream error: {}", e)))
    });

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from_stream(body_stream))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build variant playlist response");
            AppError::Internal(e.to_string()).into()
        })
}

#[utoipa::path(
    get,
    path = "/api/v0/videos/{id}/stream/{variant}/{segment}",
    tag = "videos",
    params(
        ("id" = Uuid, Path, description = "Video ID"),
        ("variant" = String, Path, description = "Video quality variant (e.g., 360p, 480p, 720p, 1080p)"),
        ("segment" = String, Path, description = "Segment filename (e.g., segment_000.ts)")
    ),
    responses(
        (status = 200, description = "Video segment", content_type = "video/mp2t"),
        (status = 400, description = "Invalid segment name", body = ErrorResponse),
        (status = 404, description = "Segment not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn stream_segment(
    State(state): State<Arc<AppState>>,
    Path((id, variant, segment)): Path<(Uuid, String, String)>,
) -> Result<impl IntoResponse, HttpAppError> {
    if segment.contains("..") || segment.contains('/') || segment.contains('\\') {
        return Err(AppError::BadRequest("Invalid segment name".to_string()).into());
    }

    let segment_key = format!("uploads/{}/{}/{}", id, variant, segment);

    let _video = state
        .media
        .repository
        .get_video_by_id_unchecked(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch video");
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Video not found".to_string()))?;

    let stream = state
        .media
        .storage
        .download_stream(&segment_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %segment_key, "Failed to fetch segment from storage");
            AppError::NotFound("Segment not found".to_string())
        })?;

    let body_stream = stream.map(|result| {
        result.map_err(|e| std::io::Error::other(format!("Storage stream error: {}", e)))
    });

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "video/mp2t")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from_stream(body_stream))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build segment response");
            AppError::Internal(e.to_string()).into()
        })
}
