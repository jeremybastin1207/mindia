use crate::auth::models::TenantContext;
use crate::error::ErrorResponse;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::StreamExt;
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
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let video: mindia_core::models::Video = match state
        .video
        .repository
        .get_video(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, video_id = %id, "Failed to fetch video");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "Failed to fetch video",
                    "DATABASE_ERROR",
                )),
            )
        })? {
        Some(v) => v,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Video not found", "NOT_FOUND")),
            ));
        }
    };

    let master_playlist_key = video.hls_master_playlist.as_ref().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "Video is still processing or failed",
                "PROCESSING_INCOMPLETE",
            )),
        )
    })?;

    // Download playlist as stream using Storage trait (works with any backend: S3, local, etc.)
    let stream = state
        .media
        .storage
        .download_stream(master_playlist_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %master_playlist_key, "Failed to fetch playlist from storage");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Failed to fetch playlist", "STORAGE_ERROR")),
            )
        })?;

    // Wrap storage stream for axum Body
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "Failed to build response",
                    "INTERNAL_ERROR",
                )),
            )
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
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let video: mindia_core::models::Video = match state
        .video
        .repository
        .get_video(tenant_ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, video_id = %id, "Failed to fetch video");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "Failed to fetch video",
                    "DATABASE_ERROR",
                )),
            )
        })? {
        Some(v) => v,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Video not found", "NOT_FOUND")),
            ));
        }
    };

    if video.hls_master_playlist.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "Video is still processing or failed",
                "PROCESSING_INCOMPLETE",
            )),
        ));
    }

    let playlist_key = format!("uploads/{}/{}/index.m3u8", id, variant);

    let stream = state
        .media
        .storage
        .download_stream(&playlist_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %playlist_key, "Failed to fetch variant playlist from storage");
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Variant not found", "NOT_FOUND")),
            )
        })?;

    // Wrap storage stream for axum Body
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "Failed to build response",
                    "INTERNAL_ERROR",
                )),
            )
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
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate segment filename to prevent path traversal
    if segment.contains("..") || segment.contains('/') || segment.contains('\\') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("Invalid segment name", "BAD_REQUEST")),
        ));
    }

    let segment_key = format!("uploads/{}/{}/{}", id, variant, segment);

    let _video: mindia_core::models::Video = match state
        .video
        .repository
        .get_video_by_id_unchecked(id)
        .await
        .map_err(|_e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "Failed to fetch video",
                    "DATABASE_ERROR",
                )),
            )
        })? {
        Some(v) => v,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Video not found", "NOT_FOUND")),
            ));
        }
    };

    let stream = state
        .media
        .storage
        .download_stream(&segment_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %segment_key, "Failed to fetch segment from storage");
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Segment not found", "NOT_FOUND")),
            )
        })?;

    // Wrap storage stream for axum Body
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "Failed to build response",
                    "INTERNAL_ERROR",
                )),
            )
        })
}
