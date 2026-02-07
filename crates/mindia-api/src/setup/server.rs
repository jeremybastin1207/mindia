//! Server startup and graceful shutdown

use anyhow::Result;
use axum::Router;
use mindia_core::Config;

/// Start the server with graceful shutdown
pub async fn start_server(config: &Config, app: Router) -> Result<()> {
    let addr = format!("0.0.0.0:{}", config.server_port());
    tracing::info!(addr = %addr, "Starting server");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    let max_image_mb = config.max_file_size_bytes() / 1024 / 1024;
    let max_video_mb = config.max_video_size_bytes() / 1024 / 1024;
    let max_document_mb = config.max_document_size_bytes() / 1024 / 1024;
    tracing::info!(
        max_image_mb,
        max_video_mb,
        max_document_mb,
        image_extensions = %config.allowed_extensions().join(","),
        video_extensions = %config.video_allowed_extensions().join(","),
        document_extensions = %config.document_allowed_extensions().join(","),
        audio_extensions = %config.audio_allowed_extensions().join(","),
        ffmpeg_path = %config.ffmpeg_path(),
        max_concurrent_transcodes = config.max_concurrent_transcodes(),
        "Server ready and accepting connections"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Signal handler for graceful shutdown
///
/// Listens for Ctrl+C (SIGINT) and SIGTERM signals to initiate graceful shutdown.
///
/// # Panics
/// - Panics if Ctrl+C signal handler cannot be installed (unrecoverable system error)
/// - On Unix systems, panics if SIGTERM signal handler cannot be installed (unrecoverable system error)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal");
        },
    }

    tracing::info!("Shutting down gracefully...");

    // Flush and shutdown OpenTelemetry
    crate::telemetry::shutdown_telemetry().await;
}
