//! API key management handlers
//!
//! Create, list, get, and revoke API keys. Keys are tenant-scoped and can be
//! used for authentication instead of the master API key.

use crate::auth::api_key::{
    extract_key_prefix, generate_api_key, hash_api_key, ApiKeyResponse, CreateApiKeyRequest,
    CreateApiKeyResponse,
};
use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;
use mindia_core::AppError;
use mindia_db::db::control::api_key::{ApiKey, CreateApiKeyRequest as DbCreateApiKeyRequest};

/// Query parameters for listing API keys
#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// Create a new API key
#[tracing::instrument(skip(state, ctx))]
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(HttpAppError::from(AppError::BadRequest(
            "name is required".to_string(),
        )));
    }

    if let Some(days) = request.expires_in_days {
        if !(1..=3650).contains(&days) {
            return Err(HttpAppError::from(AppError::BadRequest(
                "expires_in_days must be between 1 and 3650".to_string(),
            )));
        }
    }

    let raw_key = generate_api_key();
    let key_hash = hash_api_key(&raw_key)
        .map_err(|e| HttpAppError::from(AppError::Internal(e.to_string())))?;
    let key_prefix = extract_key_prefix(&raw_key);

    let db_request = DbCreateApiKeyRequest {
        name: name.to_string(),
        description: request.description.clone(),
        expires_in_days: request.expires_in_days,
    };

    let api_key = state.db
        .api_key_repository
        .create_api_key(ctx.tenant_id, &db_request, key_hash, key_prefix)
        .await
        .map_err(HttpAppError::from)?;

    crate::middleware::audit::log_api_key_created(ctx.tenant_id, ctx.user_id, api_key.id);

    let response = CreateApiKeyResponse {
        id: api_key.id,
        api_key: raw_key,
        name: api_key.name,
        description: api_key.description,
        key_prefix: api_key.key_prefix,
        expires_at: api_key.expires_at,
        created_at: api_key.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// List API keys for the current tenant
#[tracing::instrument(skip(state, ctx))]
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Query(query): Query<ListApiKeysQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);

    let api_keys = state.db
        .api_key_repository
        .list_by_tenant(ctx.tenant_id, limit, offset)
        .await
        .map_err(HttpAppError::from)?;

    let response: Vec<ApiKeyResponse> = api_keys
        .into_iter()
        .map(|k| ApiKeyResponse::from(auth_api_key_from_db(k)))
        .collect();

    Ok(Json(response))
}

/// Get a single API key by ID
#[tracing::instrument(skip(state, ctx))]
pub async fn get_api_key(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let api_key = state.db
        .api_key_repository
        .get_by_id(ctx.tenant_id, id)
        .await
        .map_err(HttpAppError::from)?
        .ok_or_else(|| HttpAppError::from(AppError::NotFound("API key not found".to_string())))?;

    Ok(Json(ApiKeyResponse::from(auth_api_key_from_db(api_key))))
}

/// Revoke (deactivate) an API key
#[tracing::instrument(skip(state, ctx))]
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let revoked = state.db
        .api_key_repository
        .revoke_api_key(ctx.tenant_id, id)
        .await
        .map_err(HttpAppError::from)?;

    if !revoked {
        return Err(HttpAppError::from(AppError::NotFound(
            "API key not found or already revoked".to_string(),
        )));
    }

    crate::middleware::audit::log_api_key_revoked(ctx.tenant_id, ctx.user_id, id);

    #[derive(serde::Serialize)]
    struct RevokeResponse {
        message: &'static str,
        id: Uuid,
    }

    Ok(Json(RevokeResponse {
        message: "API key revoked successfully",
        id,
    }))
}

/// Convert mindia_db::ApiKey to auth::api_key::ApiKey for ApiKeyResponse::from
fn auth_api_key_from_db(k: ApiKey) -> crate::auth::api_key::ApiKey {
    crate::auth::api_key::ApiKey {
        id: k.id,
        tenant_id: k.tenant_id,
        name: k.name,
        description: k.description,
        key_hash: k.key_hash,
        key_prefix: k.key_prefix,
        last_used_at: k.last_used_at,
        expires_at: k.expires_at,
        is_active: k.is_active,
        created_at: k.created_at,
        updated_at: k.updated_at,
    }
}
