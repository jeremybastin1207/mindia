//! Webhook management handlers
//!
//! CRUD operations for webhook configurations. Webhooks receive HTTP callbacks
//! when events occur (e.g. file.uploaded, file.deleted).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use crate::state::AppState;
use crate::utils::ssrf_validation;
use mindia_core::models::{
    UpdateWebhookRequest, WebhookEventListQuery, WebhookEventLogResponse, WebhookEventType,
    WebhookResponse,
};
use mindia_core::AppError;

/// Parse event type string, accepting both core names (file.uploaded) and aliases (image.uploaded, video.uploaded, etc.)
fn parse_webhook_event_type(s: &str) -> Result<WebhookEventType, AppError> {
    // Core names
    if let Ok(et) = s.parse::<WebhookEventType>() {
        return Ok(et);
    }
    // Type-specific event names map to core events
    let normalized = match s {
        "image.uploaded" | "video.uploaded" | "audio.uploaded" | "document.uploaded" => {
            WebhookEventType::FileUploaded
        }
        "image.deleted" | "video.deleted" | "audio.deleted" | "document.deleted" => {
            WebhookEventType::FileDeleted
        }
        "video.completed" | "video.failed" => WebhookEventType::FileProcessingCompleted,
        _ => {
            return Err(AppError::BadRequest(format!(
                "Invalid webhook event type: {}",
                s
            )));
        }
    };
    Ok(normalized)
}

/// Request body for creating a webhook.
/// Accepts both core format (event_type, signing_secret) and alternative format (events array, secret).
#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequestBody {
    pub url: String,
    /// Single event type (preferred)
    #[serde(default)]
    pub event_type: Option<WebhookEventType>,
    /// Multiple event types: creates one webhook per event
    #[serde(default)]
    pub events: Option<Vec<String>>,
    #[serde(default)]
    pub signing_secret: Option<String>,
    /// Alias for signing_secret
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Create a new webhook
#[tracing::instrument(skip(state, ctx))]
pub async fn create_webhook(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Json(request): Json<CreateWebhookRequestBody>,
) -> Result<impl IntoResponse, HttpAppError> {
    let url = request.url.trim();
    if url.is_empty() {
        return Err(HttpAppError::from(AppError::BadRequest(
            "URL is required".to_string(),
        )));
    }

    // SSRF validation: reject private/internal URLs
    ssrf_validation::validate_url_for_ssrf(url, false, None)
        .await
        .map_err(|e| HttpAppError::from(AppError::BadRequest(e)))?;

    let signing_secret = request.signing_secret.or(request.secret);

    // Resolve event type(s): prefer event_type, else first from events array
    let event_types: Vec<WebhookEventType> = if let Some(et) = request.event_type {
        vec![et]
    } else if let Some(ref events) = request.events {
        if events.is_empty() {
            return Err(HttpAppError::from(AppError::BadRequest(
                "event_type or events (non-empty) is required".to_string(),
            )));
        }
        let mut parsed = Vec::with_capacity(events.len());
        for s in events {
            let et = parse_webhook_event_type(s).map_err(HttpAppError::from)?;
            parsed.push(et);
        }
        parsed
    } else {
        return Err(HttpAppError::from(AppError::BadRequest(
            "event_type or events is required".to_string(),
        )));
    };

    let mut first_webhook = None;
    for event_type in event_types {
        let webhook = state
            .webhook_repository
            .create(
                ctx.tenant_id,
                url.to_string(),
                event_type,
                signing_secret.clone(),
                request.description.clone(),
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create webhook");
                HttpAppError::from(AppError::Internal(format!(
                    "Failed to create webhook: {}",
                    e
                )))
            })?;
        if first_webhook.is_none() {
            first_webhook = Some(webhook);
        }
    }

    let webhook = first_webhook.expect("at least one webhook created");
    let response = WebhookResponse::from(webhook);
    Ok((StatusCode::CREATED, Json(response)))
}

/// List all webhooks for the tenant
#[tracing::instrument(skip(state, ctx))]
pub async fn list_webhooks(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
) -> Result<impl IntoResponse, HttpAppError> {
    let webhooks = state
        .webhook_repository
        .list_by_tenant(ctx.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list webhooks");
            HttpAppError::from(AppError::Internal("Failed to list webhooks".to_string()))
        })?;

    let responses: Vec<WebhookResponse> = webhooks.into_iter().map(WebhookResponse::from).collect();
    Ok(Json(responses))
}

/// Get a webhook by ID
#[tracing::instrument(skip(state, ctx))]
pub async fn get_webhook(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let webhook = state
        .webhook_repository
        .get_by_id(ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook");
            HttpAppError::from(AppError::Internal("Failed to get webhook".to_string()))
        })?
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    Ok(Json(WebhookResponse::from(webhook)))
}

/// Update a webhook
#[tracing::instrument(skip(state, ctx))]
pub async fn update_webhook(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateWebhookRequest>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Ensure webhook exists and belongs to tenant
    let _existing = state
        .webhook_repository
        .get_by_id(ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook");
            HttpAppError::from(AppError::Internal("Failed to get webhook".to_string()))
        })?
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    if let Some(ref url) = request.url {
        let url = url.trim();
        if !url.is_empty() {
            ssrf_validation::validate_url_for_ssrf(url, false, None)
                .await
                .map_err(|e| HttpAppError::from(AppError::BadRequest(e)))?;
        }
    }

    let webhook = state
        .webhook_repository
        .update(
            ctx.tenant_id,
            id,
            request.url,
            request.signing_secret,
            request.is_active,
            request.description,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update webhook");
            HttpAppError::from(AppError::Internal(format!(
                "Failed to update webhook: {}",
                e
            )))
        })?;

    Ok(Json(WebhookResponse::from(webhook)))
}

/// Delete a webhook
#[tracing::instrument(skip(state, ctx))]
pub async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpAppError> {
    let deleted = state
        .webhook_repository
        .delete(ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete webhook");
            HttpAppError::from(AppError::Internal("Failed to delete webhook".to_string()))
        })?;

    if !deleted {
        return Err(HttpAppError::from(AppError::NotFound(
            "Webhook not found".to_string(),
        )));
    }

    Ok(StatusCode::OK)
}

/// List delivery events for a webhook
#[tracing::instrument(skip(state, ctx))]
pub async fn list_webhook_events(
    State(state): State<Arc<AppState>>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Query(query): Query<WebhookEventListQuery>,
) -> Result<impl IntoResponse, HttpAppError> {
    // Ensure webhook exists and belongs to tenant
    let _webhook = state
        .webhook_repository
        .get_by_id(ctx.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook");
            HttpAppError::from(AppError::Internal("Failed to get webhook".to_string()))
        })?
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let events = state
        .webhook_event_repository
        .list_by_webhook(id, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list webhook events");
            HttpAppError::from(AppError::Internal(
                "Failed to list webhook events".to_string(),
            ))
        })?;

    let responses: Vec<WebhookEventLogResponse> = events
        .into_iter()
        .map(WebhookEventLogResponse::from)
        .collect();
    Ok(Json(responses))
}
