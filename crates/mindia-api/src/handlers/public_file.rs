//! Public file route: serves a file by signed token (no auth).
//! Used so external services (e.g. Replicate) can fetch images for plugins.

use crate::error::HttpAppError;
use crate::state::AppState;
use crate::utils::public_file_token;
use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, StatusCode},
    response::Response,
};
use futures::StreamExt;
use mindia_core::AppError;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct PublicFileQuery {
    pub token: String,
}

/// Serve a file by signed token. No auth required; token proves (tenant_id, media_id) and expiry.
#[tracing::instrument(skip(state, query), fields(operation = "get_public_file"))]
pub async fn get_public_file(
    Query(query): Query<PublicFileQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, HttpAppError> {
    let token = query.token.trim();
    if token.is_empty() {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Missing token parameter".to_string(),
        )));
    }
    let secret = state.config.jwt_secret().as_bytes();
    let (tenant_id, media_id) = public_file_token::verify(token, secret)?;

    let image = state
        .media
        .repository
        .get_image(tenant_id, media_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error fetching image for public file");
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Image not found".to_string()))?;

    let stream = state
        .media
        .storage
        .download_stream(image.storage_key())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, storage_key = %image.storage_key(), "Failed to retrieve file from storage");
            AppError::Internal(e.to_string())
        })?;

    let body_stream = stream.map(|result| {
        result.map_err(|e| std::io::Error::other(format!("Storage stream error: {}", e)))
    });

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, image.content_type.as_str())
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .body(Body::from_stream(body_stream))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build response");
            HttpAppError::from(AppError::Internal(e.to_string()))
        })?;

    Ok(response)
}
