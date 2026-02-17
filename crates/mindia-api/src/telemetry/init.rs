use tracing_subscriber::{
    fmt::format::Format, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

/// Initialize telemetry (tracing, and OpenTelemetry when feature is enabled).
#[allow(clippy::too_many_arguments)]
pub fn init_telemetry(
    enabled: bool,
    endpoint: Option<String>,
    _service_name: String,
    _service_version: String,
    _protocol: String,
    _environment: String,
    _sampler: String,
    _sample_ratio: f64,
    _metrics_interval_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    if !enabled || endpoint.is_none() {
        // Console: compact format (message string for convenience). Structured fields go to OTLP when enabled.
        let console_fmt = tracing_subscriber::fmt::layer().event_format(
            Format::default()
                .compact()
                .with_target(false)
                .without_time(),
        );
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "mindia=debug,tower_http=debug".into()),
            )
            .with(console_fmt)
            .init();

        tracing::info!("OpenTelemetry disabled, using standard tracing");
        return Ok(());
    }

    #[cfg(not(feature = "observability-opentelemetry"))]
    {
        tracing::warn!("OpenTelemetry requested but feature not enabled, using standard tracing");
        let console_fmt = tracing_subscriber::fmt::layer().event_format(
            Format::default()
                .compact()
                .with_target(false)
                .without_time(),
        );
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "mindia=debug,tower_http=debug".into()),
            )
            .with(console_fmt)
            .init();
        Ok(())
    }

    #[cfg(feature = "observability-opentelemetry")]
    {
        mindia_infra::init_telemetry(
            enabled,
            endpoint,
            service_name,
            service_version,
            protocol,
            environment,
            sampler,
            sample_ratio,
            metrics_interval_secs,
        )
    }
}

pub async fn shutdown_telemetry() {
    #[cfg(not(feature = "observability-opentelemetry"))]
    {
        tracing::debug!("Telemetry shutdown (OpenTelemetry feature not enabled)");
    }

    #[cfg(feature = "observability-opentelemetry")]
    {
        mindia_infra::shutdown_telemetry().await;
    }
}
