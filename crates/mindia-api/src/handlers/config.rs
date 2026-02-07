use crate::state::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3Config>,
    pub upload: UploadConfig,
    pub database: DatabaseConfig,
    pub cors: CorsConfig,
    pub clamav: ClamAVConfig,
}

#[derive(Serialize, ToSchema)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
}

#[derive(Serialize, ToSchema)]
pub struct UploadConfig {
    pub max_file_size_mb: usize,
    pub max_file_size_bytes: usize,
    pub allowed_extensions: Vec<String>,
    pub allowed_content_types: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct DatabaseConfig {
    pub max_connections: u32,
    pub timeout_seconds: u64,
}

#[derive(Serialize, ToSchema)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ClamAVConfig {
    pub enabled: bool,
}

#[utoipa::path(
    get,
    path = "/api/v0/config",
    tag = "config",
    responses(
        (status = 200, description = "Service configuration", body = ConfigResponse)
    )
)]
pub async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config = ConfigResponse {
        s3: state.s3.as_ref().map(|s3| S3Config {
            bucket: s3.bucket.clone(),
            region: s3.region.clone(),
        }),
        upload: UploadConfig {
            max_file_size_mb: state.media.image_max_file_size / 1024 / 1024,
            max_file_size_bytes: state.media.image_max_file_size,
            allowed_extensions: state.media.image_allowed_extensions.clone(),
            allowed_content_types: state.media.image_allowed_content_types.clone(),
        },
        database: DatabaseConfig {
            max_connections: state.database.max_connections,
            timeout_seconds: state.database.timeout_seconds,
        },
        cors: CorsConfig {
            allowed_origins: state.security.cors_origins.clone(),
        },
        clamav: ClamAVConfig {
            enabled: state.security.clamav_enabled,
        },
    };

    Json(config)
}
