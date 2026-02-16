//! Health check handlers and response types.

use crate::state::AppState;
use axum::{http::StatusCode, response::IntoResponse, Json};
use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

/// Run an async check with timeout; returns status string "healthy", "timeout", or "{prefix}: {error}".
async fn run_check<F, E>(timeout: Duration, f: F, error_prefix: &str) -> String
where
    F: Future<Output = Result<(), E>>,
    E: Display,
{
    match tokio::time::timeout(timeout, f).await {
        Ok(Ok(())) => "healthy".to_string(),
        Ok(Err(e)) => format!("{}: {}", error_prefix, e),
        Err(_) => "timeout".to_string(),
    }
}

#[derive(serde::Serialize)]
pub(super) struct HealthCheckResponse {
    pub status: String,
    pub database: String,
    pub storage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clamav: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_queue: Option<String>,
}

#[derive(serde::Serialize)]
pub(super) struct DeepHealthCheckResponse {
    pub status: String,
    pub database: String,
    pub storage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clamav: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_queue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhooks: Option<String>,
}

/// Liveness probe - process is running.
pub async fn liveness_check(_state: Arc<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "alive" })),
    )
}

/// Readiness probe - critical dependencies (database).
pub async fn readiness_check(state: Arc<AppState>) -> impl IntoResponse {
    const TIMEOUT: Duration = Duration::from_secs(5);

    let mut response = serde_json::json!({
        "status": "ready",
        "database": "unknown"
    });

    let mut overall_ready = true;
    match tokio::time::timeout(TIMEOUT, sqlx::query("SELECT 1").execute(&state.db.pool)).await {
        Ok(Ok(_)) => response["database"] = serde_json::json!("ready"),
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

/// Full health check (database, storage, optional ClamAV/semantic-search, task queue).
pub async fn health_check(state: Arc<AppState>) -> impl IntoResponse {
    const TIMEOUT: Duration = Duration::from_secs(5);

    let mut response = HealthCheckResponse {
        status: "healthy".to_string(),
        database: "unknown".to_string(),
        storage: "unknown".to_string(),
        clamav: None,
        semantic_search: None,
        task_queue: None,
    };

    let pool = state.db.pool.clone();
    response.database = run_check(
        TIMEOUT,
        async move { sqlx::query("SELECT 1").execute(&pool).await.map(drop) },
        "unhealthy",
    )
    .await;
    let overall_healthy = response.database == "healthy";

    let storage = state.media.storage.clone();
    response.storage = run_check(
        TIMEOUT,
        async move {
            storage
                .exists("health-check-non-existent-key")
                .await
                .map(drop)
        },
        "degraded",
    )
    .await;

    if state.security.clamav_enabled {
        response.clamav = Some("not_checked".to_string());
    }

    if state.config.semantic_search_enabled() {
        #[cfg(feature = "semantic-search")]
        {
            if let Some(provider) = &state.semantic_search {
                response.semantic_search = Some(
                    match tokio::time::timeout(TIMEOUT, provider.health_check()).await {
                        Ok(Ok(true)) => "healthy".to_string(),
                        Ok(Ok(false)) | Ok(Err(_)) => "unhealthy".to_string(),
                        Err(_) => "timeout".to_string(),
                    },
                );
            } else {
                response.semantic_search = Some("not_configured".to_string());
            }
        }
        #[cfg(not(feature = "semantic-search"))]
        {
            response.semantic_search = Some("not_configured".to_string());
        }
    }

    let pool = state.db.pool.clone();
    response.task_queue = Some(
        run_check(
            TIMEOUT,
            async move {
                sqlx::query("SELECT COUNT(*) FROM tasks WHERE 1=0")
                    .execute(&pool)
                    .await
                    .map(drop)
            },
            "unhealthy",
        )
        .await,
    );

    let status_code = if overall_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}

/// Deep health check: same as /health plus webhooks table.
pub async fn deep_health_check(state: Arc<AppState>) -> impl IntoResponse {
    const TIMEOUT: Duration = Duration::from_secs(5);

    let mut response = DeepHealthCheckResponse {
        status: "healthy".to_string(),
        database: "unknown".to_string(),
        storage: "unknown".to_string(),
        clamav: None,
        semantic_search: None,
        task_queue: None,
        webhooks: None,
    };

    let pool = state.db.pool.clone();
    response.database = run_check(
        TIMEOUT,
        async move { sqlx::query("SELECT 1").execute(&pool).await.map(drop) },
        "unhealthy",
    )
    .await;
    let overall_healthy = response.database == "healthy";

    let storage = state.media.storage.clone();
    response.storage = run_check(
        TIMEOUT,
        async move {
            storage
                .exists("health-check-non-existent-key")
                .await
                .map(drop)
        },
        "degraded",
    )
    .await;

    if state.security.clamav_enabled {
        response.clamav = Some("not_checked".to_string());
    }

    if state.config.semantic_search_enabled() {
        #[cfg(feature = "semantic-search")]
        {
            if let Some(provider) = &state.semantic_search {
                response.semantic_search = Some(
                    match tokio::time::timeout(TIMEOUT, provider.health_check()).await {
                        Ok(Ok(true)) => "healthy".to_string(),
                        _ => "unhealthy".to_string(),
                    },
                );
            } else {
                response.semantic_search = Some("not_configured".to_string());
            }
        }
        #[cfg(not(feature = "semantic-search"))]
        {
            response.semantic_search = Some("not_configured".to_string());
        }
    }

    let pool = state.db.pool.clone();
    response.task_queue = Some(
        run_check(
            TIMEOUT,
            async move {
                sqlx::query("SELECT COUNT(*) FROM tasks WHERE 1=0")
                    .execute(&pool)
                    .await
                    .map(drop)
            },
            "unhealthy",
        )
        .await,
    );

    let pool = state.db.pool.clone();
    response.webhooks = Some(
        run_check(
            TIMEOUT,
            async move {
                sqlx::query("SELECT 1 FROM webhooks LIMIT 0")
                    .execute(&pool)
                    .await
                    .map(drop)
            },
            "unhealthy",
        )
        .await,
    );

    let status_code = if overall_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}
