use crate::auth::models::TenantContext;
use crate::error::{error_response_with_event, ErrorResponse, HttpAppError};
use crate::middleware::json_response_with_event;
use crate::services::upload::{
    AudioMetadata, AudioProcessorImpl, MediaLimitsConfig, MediaProcessor, MediaUploadService,
};
use crate::state::AppState;
use crate::telemetry::wide_event::WideEvent;
use crate::utils::ip_extraction::ClientIpOpt;
use crate::utils::transaction::with_transaction;
use crate::utils::upload::parse_store_parameter;
use axum::{
    extract::{Multipart, Query, State},
    response::Response,
    Json,
};
use chrono::Utc;
use mindia_core::models::{AudioResponse, MediaType};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct StoreQuery {
    #[serde(default = "default_store")]
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
#[tracing::instrument(
    skip(state, multipart),
    fields(
        tenant_id = %tenant_ctx.tenant_id,
        user_id = ?tenant_ctx.user_id,
        store = %query.store,
        operation = "upload_audio"
    )
)]
pub async fn upload_audio(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    Query(query): Query<StoreQuery>,
    ClientIpOpt(client_ip): ClientIpOpt,
    multipart: Multipart,
) -> Result<Response, HttpAppError> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut wide_event = WideEvent::new(
        request_id,
        state.config.otel_service_name().to_string(),
        state.config.environment().to_string(),
        "POST".to_string(),
        "/api/v0/audios".to_string(),
        Utc::now(),
    );
    wide_event.with_tenant_context(&tenant_ctx);
    wide_event.with_business_context(|ctx| {
        ctx.media_type = Some("audio".to_string());
        ctx.operation = Some("upload".to_string());
    });

    let (store_permanently, expires_at) =
        match parse_store_parameter(&query.store, state.config.auto_store_enabled()) {
            Ok(result) => result,
            Err(e) => return Ok(error_response_with_event(HttpAppError::from(e), wide_event)),
        };
    let store_behavior = query.store.clone();

    let audio_limits = state.media.limits_for(MediaType::Audio);
    let config = MediaLimitsConfig {
        limits: &audio_limits,
        media_type_name: "audio",
    };

    let ffprobe_path = state.config.ffmpeg_path().replace("ffmpeg", "ffprobe");
    let service = MediaUploadService::new(&state);
    let processor: Box<dyn MediaProcessor<Metadata = AudioMetadata> + Send + Sync> =
        Box::new(AudioProcessorImpl::new(ffprobe_path));

    let (upload_data, metadata) = match service
        .upload(
            tenant_ctx.tenant_id,
            multipart,
            &config,
            processor,
            store_permanently,
            expires_at,
            store_behavior.clone(),
            Some(tenant_ctx.user_id),
            client_ip,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => return Ok(error_response_with_event(HttpAppError::from(e), wide_event)),
    };

    let audio = match with_transaction(&state.db.pool, |tx| {
        let repo = state.media.repository.clone();
        let ud = upload_data.clone();
        let meta = metadata.clone();
        let sb = store_behavior.clone();
        Box::pin(async move {
            repo.create_audio_from_storage_tx(
                tx,
                ud.tenant_id,
                ud.file_id,
                ud.uuid_filename,
                ud.safe_original_filename,
                ud.content_type,
                ud.file_size,
                meta.duration,
                meta.bitrate,
                meta.sample_rate,
                meta.channels,
                sb,
                store_permanently,
                expires_at,
                None,
                ud.storage_key,
                ud.storage_url,
            )
            .await
        })
    })
    .await
    {
        Ok(a) => {
            wide_event.with_business_context(|ctx| {
                ctx.media_id = Some(a.id);
            });
            a
        }
        Err(e) => {
            let storage_key = upload_data.storage_key.clone();
            let storage = state.media.storage.clone();
            tokio::spawn(async move {
                if let Err(cleanup_err) = storage.delete(&storage_key).await {
                    tracing::warn!(
                        error = %cleanup_err,
                        storage_key = %storage_key,
                        "Failed to cleanup storage after DB error"
                    );
                }
            });
            return Ok(error_response_with_event(HttpAppError::from(e), wide_event));
        }
    };

    Ok(json_response_with_event(
        Json(AudioResponse::from(audio)),
        wide_event,
    ))
}
