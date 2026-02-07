//! Route configuration and setup

use crate::constants::API_PREFIX;
use crate::handlers;
use crate::middleware::{
    analytics_middleware,
    idempotency::{idempotency_middleware, setup_idempotency_state},
    rate_limit::{rate_limit_middleware, HttpRateLimiter},
    request_id_middleware,
    security_headers::{security_headers_middleware, SecurityHeadersConfig},
    wide_event_middleware,
};
use crate::state::AppState;
use axum::{
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use mindia_core::Config;
use std::sync::Arc;
use std::time::Duration;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::auth::middleware::AuthState;

/// Setup all application routes
pub async fn setup_routes(
    config: &Config,
    state: Arc<AppState>,
) -> Result<Router<()>, anyhow::Error> {
    let cors = setup_cors(config)?;
    let auth_state = setup_auth_middleware(config, &state)?;
    let rate_limiter = setup_rate_limiter(config);
    // Setup idempotency (24 hour TTL)
    let idempotency_state = setup_idempotency_state(86400);

    // Public routes (no authentication required)
    let public_routes = public_routes(state.clone());

    // Protected routes (require authentication)
    // State is applied in protected_routes() for handlers with Multipart to work
    let protected_routes =
        protected_routes(state.clone()).layer(axum::middleware::from_fn_with_state(
            Arc::new(auth_state.clone()),
            crate::auth::middleware::auth_middleware,
        ));

    // Merge routes and apply middleware
    let app_state_routes = public_routes.merge(protected_routes);

    // Setup HTTP metrics when OpenTelemetry is enabled
    let trace_layer = {
        #[cfg(feature = "observability-opentelemetry")]
        {
            if config.otel_enabled() {
                // Create HTTP metrics using global meter provider
                let meter = opentelemetry::global::meter("mindia");
                let http_metrics = HttpMetrics::new(meter);

                // Create custom trace layer with metrics
                TraceLayer::new_for_http()
                    .make_span_with(CustomMakeSpan::new(http_metrics.clone()))
                    .on_request(CustomOnRequest)
                    .on_response(CustomOnResponse::new(http_metrics.clone()))
                    .on_failure(CustomOnFailure::new(http_metrics))
            } else {
                // Use basic trace layer when OpenTelemetry is disabled
                TraceLayer::new_for_http()
            }
        }
        #[cfg(not(feature = "observability-opentelemetry"))]
        {
            // Use basic trace layer when OpenTelemetry feature is not enabled
            TraceLayer::new_for_http()
        }
    };

    // Setup security headers config
    let cdn_domains = std::env::var("CDN_DOMAINS")
        .unwrap_or_else(|_| String::new())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let security_headers_config = Arc::new(SecurityHeadersConfig::new(
        cdn_domains,
        config.is_production(),
    ));

    // Server-level concurrency limit to protect against resource exhaustion under extreme load
    let http_concurrency_limit = std::env::var("HTTP_CONCURRENCY_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10_000)
        .max(1);
    tracing::info!(
        http_concurrency_limit = http_concurrency_limit,
        "HTTP concurrency limit layer enabled"
    );

    let app = app_state_routes
        .nest(
            "/docs",
            utoipa_rapidoc::RapiDoc::new("/api/openapi.json")
                .path("/docs")
                .into(),
        )
        .layer(ConcurrencyLimitLayer::new(http_concurrency_limit))
        .layer(RequestBodyLimitLayer::new(
            config
                .max_video_size_bytes()
                .max(config.max_file_size_bytes())
                .max(config.max_document_size_bytes())
                .max(config.max_audio_size_bytes()),
        ))
        .layer(cors)
        .layer(trace_layer)
        .layer(axum::middleware::from_fn(request_id_middleware))
        .layer(axum::middleware::from_fn_with_state(
            security_headers_config,
            security_headers_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            idempotency_state.clone(),
            idempotency_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            wide_event_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            analytics_middleware,
        ))
        .with_state(state);

    Ok(app)
}

/// Setup CORS configuration
fn setup_cors(config: &Config) -> Result<CorsLayer, anyhow::Error> {
    let cors = if config.cors_origins().contains(&"*".to_string()) {
        tracing::warn!("CORS configured to allow all origins - not recommended for production");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers(Any)
    } else {
        let origins: Result<Vec<HeaderValue>, _> =
            config.cors_origins().iter().map(|o| o.parse()).collect();

        CorsLayer::new()
            .allow_origin(origins.unwrap_or_default())
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers(Any)
    };
    Ok(cors)
}

/// Setup authentication middleware state
fn setup_auth_middleware(
    _config: &Config,
    state: &Arc<AppState>,
) -> Result<AuthState, anyhow::Error> {
    // Get master API key from environment
    let master_api_key = std::env::var("MASTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("MASTER_API_KEY environment variable not set"))?;

    if master_api_key.len() < 32 {
        return Err(anyhow::anyhow!(
            "MASTER_API_KEY must be at least 32 characters long"
        ));
    }

    Ok(AuthState {
        master_api_key,
        api_key_repository: state.api_key_repository.clone(),
        tenant_repository: state.tenant_repository.clone(),
    })
}

/// Setup rate limiter with periodic cleanup task
fn setup_rate_limiter(config: &Config) -> Arc<HttpRateLimiter> {
    // Allow shard count to be configured via environment variable (default: 16)
    let shard_count = std::env::var("RATE_LIMITER_SHARD_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(16)
        .max(1); // Ensure at least 1 shard

    let rate_limiter = if let Some(tenant_limit) = config.http_tenant_rate_limit_per_minute() {
        Arc::new(HttpRateLimiter::with_tenant_limit_and_shards(
            config.http_rate_limit_per_minute(),
            tenant_limit,
            shard_count,
        ))
    } else {
        Arc::new(HttpRateLimiter::with_shards(
            config.http_rate_limit_per_minute(),
            shard_count,
        ))
    };

    // Start periodic cleanup task to prevent memory leak from expired buckets
    let rate_limiter_for_cleanup = rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // Every 5 minutes
        loop {
            interval.tick().await;
            rate_limiter_for_cleanup.cleanup_expired_buckets().await;
        }
    });

    tracing::info!(
        rate_limit_per_minute = config.http_rate_limit_per_minute(),
        tenant_rate_limit_per_minute = ?config.http_tenant_rate_limit_per_minute(),
        shard_count = shard_count,
        "HTTP rate limiting enabled with sharded buckets and automatic cleanup (every 5 minutes)"
    );
    rate_limiter
}

/// Public routes (no authentication required)
fn public_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/health",
            get({
                let state = state.clone();
                move || {
                    let state = state.clone();
                    async { health_check(state).await }
                }
            }),
        )
        .route(
            "/live",
            get({
                let state = state.clone();
                move || async { liveness_check(state).await }
            }),
        )
        .route(
            "/ready",
            get({
                let state = state.clone();
                move || async { readiness_check(state).await }
            }),
        )
        .with_state(state)
        .route(
            "/api/openapi.json",
            get(|| async { Json(crate::api_doc::get_openapi_spec()) }),
        )
        .route(
            "/llms.txt",
            get(|| async {
                // Serve llms.txt from static directory (async I/O to avoid blocking runtime)
                match tokio::fs::read_to_string("static/llms.txt").await {
                    Ok(content) => (
                        axum::http::StatusCode::OK,
                        [("Content-Type", "text/plain; charset=utf-8")],
                        content,
                    )
                        .into_response(),
                    Err(_) => {
                        (axum::http::StatusCode::NOT_FOUND, "llms.txt not found").into_response()
                    }
                }
            }),
        )
        .route(
            "/.well-known/llms.txt",
            get(|| async {
                // Also serve from .well-known path (standard location, async I/O)
                match tokio::fs::read_to_string("static/llms.txt").await {
                    Ok(content) => (
                        axum::http::StatusCode::OK,
                        [("Content-Type", "text/plain; charset=utf-8")],
                        content,
                    )
                        .into_response(),
                    Err(_) => {
                        (axum::http::StatusCode::NOT_FOUND, "llms.txt not found").into_response()
                    }
                }
            }),
        )
}

/// Protected routes (require authentication).
fn protected_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/batch", API_PREFIX),
            post(handlers::batch::batch_operations),
        )
        .merge(media_routes(state.clone()))
        .merge(image_routes(state.clone()))
        .merge(video_routes(state.clone()))
        .merge(document_routes(state.clone()))
        .merge(audio_routes(state.clone()))
        .merge(folder_routes(state.clone()))
        .merge(preset_routes(state.clone()))
        .merge(analytics_routes(state.clone()))
        .merge(search_routes(state.clone()))
        .merge(metadata_routes(state.clone()))
        .merge(task_routes(state.clone()))
        .merge(file_group_routes(state.clone()))
        .merge(plugin_routes(state.clone()))
        .merge(upload_routes(state.clone()))
        .merge(webhook_routes(state.clone()))
        .merge(api_key_routes(state.clone()))
        .with_state(state)
}

/// Media routes (unified operations)
fn media_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/media/:id", API_PREFIX),
            get(handlers::media_get::get_media),
        )
        .route(
            &format!("{}/media/:id", API_PREFIX),
            delete(handlers::media_delete::delete_media),
        )
        .route(
            &format!("{}/media/batch/delete", API_PREFIX),
            post(handlers::batch_media::batch_delete_media),
        )
        .route(
            &format!("{}/media/batch/copy", API_PREFIX),
            post(handlers::batch_media::batch_copy_media),
        )
        .with_state(state)
}

/// Image routes
fn image_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/images", API_PREFIX),
            post(handlers::image_upload::upload_image),
        )
        .route(
            &format!("{}/images/from-url", API_PREFIX),
            post(handlers::image_upload_url::upload_image_from_url),
        )
        .route(
            &format!("{}/images", API_PREFIX),
            get(handlers::image_get::list_images),
        )
        .route(
            &format!("{}/images/:id", API_PREFIX),
            get(handlers::image_get::get_image),
        )
        .route(
            &format!("{}/images/:id/file", API_PREFIX),
            get(handlers::image_download::download_image),
        )
        .route(
            &format!("{}/images/:id/*operations", API_PREFIX),
            get(handlers::transform::transform_image),
        )
        .route(
            &format!("{}/images/:id/metadata", API_PREFIX),
            put(handlers::metadata::update_image_metadata),
        )
        .with_state(state)
}

/// Video routes
fn video_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    #[cfg(feature = "video")]
    {
        Router::new()
            .route(
                &format!("{}/videos", API_PREFIX),
                post(handlers::video_upload::upload_video),
            )
            .route(
                &format!("{}/videos", API_PREFIX),
                get(handlers::video_get::list_videos),
            )
            .route(
                &format!("{}/videos/:id", API_PREFIX),
                get(handlers::video_get::get_video),
            )
            .route(
                &format!("{}/videos/:id/metadata", API_PREFIX),
                put(handlers::metadata::update_video_metadata),
            )
            .route(
                &format!("{}/videos/:id/stream/master.m3u8", API_PREFIX),
                get(handlers::video_stream::stream_master_playlist),
            )
            .route(
                &format!("{}/videos/:id/stream/:variant/index.m3u8", API_PREFIX),
                get(handlers::video_stream::stream_variant_playlist),
            )
            .route(
                &format!("{}/videos/:id/stream/:variant/:segment", API_PREFIX),
                get(handlers::video_stream::stream_segment),
            )
            .with_state(state)
    }
    #[cfg(not(feature = "video"))]
    {
        Router::new().with_state(state)
    }
}

/// Document routes
fn document_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    #[cfg(feature = "document")]
    {
        Router::new()
            .route(
                &format!("{}/documents", API_PREFIX),
                post(handlers::document_upload::upload_document),
            )
            .route(
                &format!("{}/documents", API_PREFIX),
                get(handlers::document_get::list_documents),
            )
            .route(
                &format!("{}/documents/:id", API_PREFIX),
                get(handlers::document_get::get_document),
            )
            .route(
                &format!("{}/documents/:id/file", API_PREFIX),
                get(handlers::document_download::download_document),
            )
            // Document delete is now handled by unified media delete endpoint
            .route(
                &format!("{}/documents/:id/metadata", API_PREFIX),
                put(handlers::metadata::update_document_metadata),
            )
            .with_state(state)
    }
    #[cfg(not(feature = "document"))]
    {
        Router::new().with_state(state)
    }
}

/// Audio routes
fn audio_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    #[cfg(feature = "audio")]
    {
        Router::new()
            .route(
                &format!("{}/audios", API_PREFIX),
                post(handlers::audio_upload::upload_audio),
            )
            .route(
                &format!("{}/audios", API_PREFIX),
                get(handlers::audio_get::list_audios),
            )
            .route(
                &format!("{}/audios/:id", API_PREFIX),
                get(handlers::audio_get::get_audio),
            )
            .route(
                &format!("{}/audios/:id/file", API_PREFIX),
                get(handlers::audio_download::download_audio),
            )
            .route(
                &format!("{}/audios/:id/metadata", API_PREFIX),
                put(handlers::metadata::update_audio_metadata),
            )
            .with_state(state)
    }
    #[cfg(not(feature = "audio"))]
    {
        Router::new().with_state(state)
    }
}

/// Folder routes
fn folder_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/folders", API_PREFIX),
            post(handlers::folders::create_folder),
        )
        .route(
            &format!("{}/folders", API_PREFIX),
            get(handlers::folders::list_folders),
        )
        .route(
            &format!("{}/folders/tree", API_PREFIX),
            get(handlers::folders::get_folder_tree),
        )
        .route(
            &format!("{}/folders/:id", API_PREFIX),
            get(handlers::folders::get_folder),
        )
        .route(
            &format!("{}/folders/:id", API_PREFIX),
            put(handlers::folders::update_folder),
        )
        .route(
            &format!("{}/folders/:id", API_PREFIX),
            delete(handlers::folders::delete_folder),
        )
        .route(
            &format!("{}/media/:id/folder", API_PREFIX),
            put(handlers::folders::move_media),
        )
        .with_state(state)
}

/// Preset (named transformation) routes
fn preset_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/presets", API_PREFIX),
            post(handlers::named_transformations::create_preset)
                .get(handlers::named_transformations::list_presets),
        )
        .route(
            &format!("{}/presets/:name", API_PREFIX),
            get(handlers::named_transformations::get_preset)
                .put(handlers::named_transformations::update_preset)
                .delete(handlers::named_transformations::delete_preset),
        )
        .with_state(state)
}

/// Analytics routes
fn analytics_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/analytics/traffic", API_PREFIX),
            get(handlers::analytics::get_traffic_summary),
        )
        .route(
            &format!("{}/analytics/urls", API_PREFIX),
            get(handlers::analytics::get_url_statistics),
        )
        .route(
            &format!("{}/analytics/storage", API_PREFIX),
            get(handlers::analytics::get_storage_summary),
        )
        .route(
            &format!("{}/analytics/storage/refresh", API_PREFIX),
            post(handlers::analytics::refresh_storage_metrics),
        )
        .route(
            &format!("{}/audit-logs", API_PREFIX),
            get(handlers::analytics::list_audit_logs),
        )
        .route(
            &format!("{}/audit-logs/:id", API_PREFIX),
            get(handlers::analytics::get_audit_log),
        )
        .with_state(state)
}

/// Search routes
fn search_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/search", API_PREFIX),
            get(handlers::search::search_files),
        )
        .with_state(state)
}

/// Metadata routes
fn metadata_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/config", API_PREFIX),
            get(handlers::config::get_config),
        )
        .route(
            &format!("{}/media/:id/metadata", API_PREFIX),
            get(handlers::metadata::get_metadata),
        )
        .route(
            &format!("{}/media/:id/metadata/:key", API_PREFIX),
            put(handlers::metadata::update_metadata_key)
                .get(handlers::metadata::get_metadata_key)
                .delete(handlers::metadata::delete_metadata_key),
        )
        .with_state(state)
}

/// Task management routes
fn task_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/tasks", API_PREFIX),
            get(handlers::tasks::list_tasks),
        )
        .route(
            &format!("{}/tasks/:id", API_PREFIX),
            get(handlers::tasks::get_task),
        )
        .route(
            &format!("{}/tasks/:id/cancel", API_PREFIX),
            post(handlers::tasks::cancel_task),
        )
        .route(
            &format!("{}/tasks/:id/retry", API_PREFIX),
            post(handlers::tasks::retry_task),
        )
        .route(
            &format!("{}/tasks/stats", API_PREFIX),
            get(handlers::tasks::get_task_stats),
        )
        .with_state(state)
}

/// File group routes
fn file_group_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/groups", API_PREFIX),
            post(handlers::file_group::create_file_group),
        )
        .route(
            &format!("{}/groups/:id", API_PREFIX),
            get(handlers::file_group::get_file_group),
        )
        .route(
            &format!("{}/groups/:id/info", API_PREFIX),
            get(handlers::file_group::get_file_group_info),
        )
        .route(
            &format!("{}/groups/:id/nth/:index", API_PREFIX),
            get(handlers::file_group::get_file_by_index),
        )
        .route(
            &format!("{}/groups/:id/archive/:format", API_PREFIX),
            get(handlers::file_group::get_group_archive),
        )
        .route(
            &format!("{}/groups/:id", API_PREFIX),
            delete(handlers::file_group::delete_file_group),
        )
        .with_state(state)
}

/// Webhook management routes
fn webhook_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/webhooks", API_PREFIX),
            post(handlers::webhooks::create_webhook).get(handlers::webhooks::list_webhooks),
        )
        .route(
            &format!("{}/webhooks/:id", API_PREFIX),
            get(handlers::webhooks::get_webhook)
                .put(handlers::webhooks::update_webhook)
                .delete(handlers::webhooks::delete_webhook),
        )
        .route(
            &format!("{}/webhooks/:id/events", API_PREFIX),
            get(handlers::webhooks::list_webhook_events),
        )
        .with_state(state)
}

/// API key management routes
fn api_key_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/api-keys", API_PREFIX),
            post(handlers::api_keys::create_api_key).get(handlers::api_keys::list_api_keys),
        )
        .route(
            &format!("{}/api-keys/:id", API_PREFIX),
            get(handlers::api_keys::get_api_key).delete(handlers::api_keys::revoke_api_key),
        )
        .with_state(state)
}

/// Plugin routes
fn plugin_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    #[cfg(feature = "plugin")]
    {
        Router::new()
            .route(
                &format!("{}/plugins", API_PREFIX),
                get(handlers::plugins::list_plugins),
            )
            .route(
                &format!("{}/plugins/costs", API_PREFIX),
                get(handlers::plugins::get_plugin_costs),
            )
            .route(
                &format!("{}/plugins/costs/summary", API_PREFIX),
                get(handlers::plugins::get_plugin_costs_summary),
            )
            .route(
                &format!("{}/plugins/:plugin_name/execute", API_PREFIX),
                post(handlers::plugins::execute_plugin),
            )
            .route(
                &format!("{}/plugins/:plugin_name/config", API_PREFIX),
                get(handlers::plugins::get_plugin_config),
            )
            .route(
                &format!("{}/plugins/:plugin_name/config", API_PREFIX),
                put(handlers::plugins::update_plugin_config),
            )
            .route(
                &format!("{}/plugins/:plugin_name/costs", API_PREFIX),
                get(handlers::plugins::get_plugin_costs_by_name),
            )
            .with_state(state)
    }
    #[cfg(not(feature = "plugin"))]
    {
        Router::new().with_state(state)
    }
}

/// Upload routes (presigned and chunked)
fn upload_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/uploads/presigned", API_PREFIX),
            post(handlers::presigned_upload::generate_presigned_url),
        )
        .route(
            &format!("{}/uploads/complete", API_PREFIX),
            post(handlers::presigned_upload::complete_upload),
        )
        .route(
            &format!("{}/uploads/chunked/start", API_PREFIX),
            post(handlers::chunked_upload::start_chunked_upload),
        )
        .route(
            &format!(
                "{}/uploads/chunked/:session_id/chunk/:chunk_index",
                API_PREFIX
            ),
            put(handlers::chunked_upload::record_chunk_upload),
        )
        .route(
            &format!("{}/uploads/chunked/:session_id/complete", API_PREFIX),
            post(handlers::chunked_upload::complete_chunked_upload),
        )
        .route(
            &format!("{}/uploads/chunked/:session_id/progress", API_PREFIX),
            get(handlers::chunked_upload::get_chunked_upload_progress),
        )
        .with_state(state)
}

#[derive(serde::Serialize)]
struct HealthCheckResponse {
    status: String,
    database: String,
    storage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    clamav: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_queue: Option<String>,
}

/// Liveness probe - simple check that process is running
/// Always returns 200 if process can respond
async fn liveness_check(_state: Arc<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "alive"
        })),
    )
}

/// Readiness probe - checks if service can accept traffic
/// Checks critical dependencies (database)
async fn readiness_check(state: Arc<AppState>) -> impl IntoResponse {
    const TIMEOUT: Duration = Duration::from_secs(5);

    let mut response = serde_json::json!({
        "status": "ready",
        "database": "unknown"
    });

    let mut overall_ready = true;

    // Check database with timeout
    match tokio::time::timeout(TIMEOUT, sqlx::query("SELECT 1").execute(&state.db_pool)).await {
        Ok(Ok(_)) => {
            response["database"] = serde_json::json!("ready");
        }
        Ok(Err(e)) => {
            tracing::error!(error = %e, "Database readiness check failed");
            response["database"] = serde_json::json!(format!("not_ready: {}", e));
            overall_ready = false;
        }
        Err(_) => {
            tracing::error!("Database readiness check timed out");
            response["database"] = serde_json::json!("timeout");
            overall_ready = false;
        }
    }

    let status_code = if overall_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}

async fn health_check(state: Arc<AppState>) -> impl IntoResponse {
    const TIMEOUT: Duration = Duration::from_secs(5);

    let mut response = HealthCheckResponse {
        status: "healthy".to_string(),
        database: "unknown".to_string(),
        storage: "unknown".to_string(),
        clamav: None,
        semantic_search: None,
        task_queue: None,
    };

    let mut overall_healthy = true;

    // Check database using the pool directly with timeout
    match tokio::time::timeout(TIMEOUT, sqlx::query("SELECT 1").execute(&state.db_pool)).await {
        Ok(Ok(_)) => {
            response.database = "healthy".to_string();
        }
        Ok(Err(e)) => {
            tracing::error!(error = %e, "Database health check failed");
            response.database = format!("unhealthy: {}", e);
            overall_healthy = false;
        }
        Err(_) => {
            tracing::error!("Database health check timed out");
            response.database = "timeout".to_string();
            overall_healthy = false;
        }
    }

    // Check storage - try a lightweight exists check with a non-existent key
    // This verifies connectivity without creating files
    match tokio::time::timeout(
        TIMEOUT,
        state.media.storage.exists("health-check-non-existent-key"),
    )
    .await
    {
        Ok(Ok(_)) => {
            response.storage = "healthy".to_string();
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "Storage health check warning");
            response.storage = format!("degraded: {}", e);
            // Storage issues don't fail overall health (graceful degradation)
        }
        Err(_) => {
            tracing::warn!("Storage health check timed out");
            response.storage = "timeout".to_string();
            // Storage timeouts don't fail overall health (graceful degradation)
        }
    }

    // Check ClamAV if enabled
    if state.security.clamav_enabled {
        // The ClamAV client futures are !Send, which breaks axum handler bounds.
        // For now, report configured status without performing a live scan.
        response.clamav = Some("not_checked".to_string());
    }

    // Check semantic search (Anthropic) if enabled
    if state.config.semantic_search_enabled() {
        #[cfg(feature = "semantic-search")]
        {
            if let Some(provider) = &state.semantic_search {
                match tokio::time::timeout(TIMEOUT, provider.health_check()).await {
                    Ok(Ok(true)) => {
                        response.semantic_search = Some("healthy".to_string());
                    }
                    Ok(Ok(false)) | Ok(Err(_)) => {
                        tracing::warn!("Semantic search health check failed");
                        response.semantic_search = Some("unhealthy".to_string());
                        // Semantic search is optional, don't fail overall health
                    }
                    Err(_) => {
                        tracing::warn!("Semantic search health check timed out");
                        response.semantic_search = Some("timeout".to_string());
                    }
                }
            } else {
                response.semantic_search = Some("not_configured".to_string());
            }
        }
        #[cfg(not(feature = "semantic-search"))]
        {
            response.semantic_search = Some("not_configured".to_string());
        }
    }

    // Check task queue - verify we can access the repository
    match tokio::time::timeout(TIMEOUT, async {
        // Check if repository can be accessed (simple query to verify connectivity)
        sqlx::query("SELECT COUNT(*) FROM tasks WHERE 1=0")
            .execute(&state.db_pool)
            .await
    })
    .await
    {
        Ok(Ok(_)) => {
            response.task_queue = Some("healthy".to_string());
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "Task queue health check failed");
            response.task_queue = Some(format!("unhealthy: {}", e));
            // Task queue issues don't fail overall health
        }
        Err(_) => {
            tracing::warn!("Task queue health check timed out");
            response.task_queue = Some("timeout".to_string());
        }
    }

    let status_code = if overall_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}
