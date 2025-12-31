use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize basic tracing (without OpenTelemetry)
#[allow(clippy::too_many_arguments)]
pub fn init_telemetry(
    _enabled: bool,
    _endpoint: Option<String>,
    _service_name: String,
    _service_version: String,
    _protocol: String,
    _environment: String,
    _sampler: String,
    _sample_ratio: f64,
    _metrics_interval_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mindia=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("OpenTelemetry feature not enabled, using standard tracing");
    Ok(())
}

pub async fn shutdown_telemetry() {
    tracing::debug!("Telemetry shutdown (OpenTelemetry feature not enabled)");
}
