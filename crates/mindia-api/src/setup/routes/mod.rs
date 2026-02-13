//! Route configuration and setup.
//!
//! Domain route groups live in [domains](domains); health checks in [health](health).

mod domains;
mod health;

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
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use mindia_core::Config;
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::auth::middleware::{AuthFailureLimiter, AuthState};

#[cfg(feature = "observability-opentelemetry")]
use crate::http_metrics::{
    CustomMakeSpan, CustomOnFailure, CustomOnRequest, CustomOnResponse, HttpMetrics,
};

/// Setup all application routes
pub async fn setup_routes(
    config: &Config,
    state: Arc<AppState>,
) -> Result<Router<()>, anyhow::Error> {
    let cors = setup_cors(config)?;
    let auth_state = setup_auth_middleware(config, &state)?;
    let rate_limiter = setup_rate_limiter(config);
    let idempotency_state = setup_idempotency_state(86400);

    let public_routes = public_routes(state.clone());
    let protected_routes =
        protected_routes(state.clone()).layer(axum::middleware::from_fn_with_state(
            Arc::new(auth_state.clone()),
            crate::auth::middleware::auth_middleware,
        ));

    let app_state_routes = public_routes.merge(protected_routes);

    let trace_layer = {
        #[cfg(feature = "observability-opentelemetry")]
        {
            if config.otel_enabled() {
                let meter = opentelemetry::global::meter("mindia");
                let http_metrics = HttpMetrics::new(meter);
                TraceLayer::new_for_http()
                    .make_span_with(CustomMakeSpan::new(http_metrics.clone()))
                    .on_request(CustomOnRequest)
                    .on_response(CustomOnResponse::new(http_metrics.clone()))
                    .on_failure(CustomOnFailure::new(http_metrics))
            } else {
                TraceLayer::new_for_http()
            }
        }
        #[cfg(not(feature = "observability-opentelemetry"))]
        {
            TraceLayer::new_for_http()
        }
    };

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

    let http_concurrency_limit = std::env::var("HTTP_CONCURRENCY_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10_000)
        .max(1);
    tracing::info!(
        http_concurrency_limit = http_concurrency_limit,
        "HTTP concurrency limit layer enabled"
    );

    let request_timeout_secs = std::env::var("REQUEST_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60)
        .max(1);
    tracing::info!(request_timeout_secs, "Request timeout layer enabled");

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
        .layer(DefaultBodyLimit::disable())
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

fn setup_auth_middleware(
    _config: &Config,
    state: &Arc<AppState>,
) -> Result<AuthState, anyhow::Error> {
    let master_api_key = std::env::var("MASTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("MASTER_API_KEY environment variable not set"))?;

    if master_api_key.len() < 32 {
        return Err(anyhow::anyhow!(
            "MASTER_API_KEY must be at least 32 characters long"
        ));
    }

    let auth_failure_limiter = Some(Arc::new(AuthFailureLimiter::new(10, 900)));

    Ok(AuthState {
        master_api_key,
        api_key_repository: state.db.api_key_repository.clone(),
        tenant_repository: state.db.tenant_repository.clone(),
        auth_failure_limiter,
    })
}

fn setup_rate_limiter(config: &Config) -> Arc<HttpRateLimiter> {
    let shard_count = std::env::var("RATE_LIMITER_SHARD_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(16)
        .max(1);

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

    let rate_limiter_for_cleanup = rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
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

fn public_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/health",
            get({
                let state = state.clone();
                move || {
                    let state = state.clone();
                    async { health::health_check(state).await }
                }
            }),
        )
        .route(
            "/health/deep",
            get({
                let state = state.clone();
                move || {
                    let state = state.clone();
                    async { health::deep_health_check(state).await }
                }
            }),
        )
        .route(
            "/live",
            get({
                let state = state.clone();
                move || async { health::liveness_check(state).await }
            }),
        )
        .route(
            "/ready",
            get({
                let state = state.clone();
                move || async { health::readiness_check(state).await }
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

fn protected_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .merge(domains::media_routes(state.clone()))
        .merge(domains::image_routes(state.clone()))
        .merge(domains::video_routes(state.clone()))
        .merge(domains::document_routes(state.clone()))
        .merge(domains::audio_routes(state.clone()))
        .merge(domains::folder_routes(state.clone()))
        .merge(domains::preset_routes(state.clone()))
        .merge(domains::analytics_routes(state.clone()))
        .merge(domains::search_routes(state.clone()))
        .merge(domains::metadata_routes(state.clone()))
        .merge(domains::task_routes(state.clone()))
        .merge(domains::file_group_routes(state.clone()))
        .merge(domains::plugin_routes(state.clone()))
        .merge(domains::workflow_routes(state.clone()))
        .merge(domains::upload_routes(state.clone()))
        .merge(domains::webhook_routes(state.clone()))
        .merge(domains::api_key_routes(state.clone()))
        .with_state(state)
}
