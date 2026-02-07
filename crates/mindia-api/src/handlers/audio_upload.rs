use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;
use axum::{
    extract::{Multipart, Query, State},
    Json,
};
use mindia_core::models::AudioResponse;
use mindia_core::AppError;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct StoreQuery {
    #[serde(default = "default_store")]
    #[allow(dead_code)]
    store: String,
}

fn default_store() -> String {
    "auto".to_string()
}

#[utoipa::path(
    post,
    path = "/api/v0/audios",
    tag = "audios",
    params(
        ("store" = Option<String>, Query, description = "Storage behavior: '0' (temporary), '1' (permanent), 'auto' (default)")
    ),
    request_body(content = inline(Object), content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Audio uploaded successfully", body = AudioResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 413, description = "File too large", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn upload_audio(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(query): Query<StoreQuery>,
    multipart: Multipart,
) -> Result<Json<AudioResponse>, HttpAppError> {
    let _ = (state, tenant_ctx, query, multipart);
    Err(HttpAppError::from(AppError::Internal(
        "audio upload temporarily disabled".to_string(),
    )))
}
