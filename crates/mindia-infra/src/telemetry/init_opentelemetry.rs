#[cfg(feature = "observability-opentelemetry")]
use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
#[cfg(feature = "observability-opentelemetry")]
use opentelemetry_sdk::{
    logs as sdklogs,
    metrics::{self as sdkmetrics, PeriodicReader},
    trace::{self as sdktrace, BatchConfig, BatchSpanProcessor, RandomIdGenerator, Sampler},
    Resource,
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use std::env;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize OpenTelemetry telemetry (traces, metrics)
/// Note: OTLP log export is not configured; global logger provider was removed in opentelemetry 0.27.
#[allow(clippy::too_many_arguments)]
pub fn init_telemetry(
    enabled: bool,
    endpoint: Option<String>,
    service_name: String,
    service_version: String,
    protocol: String,
    environment: String,
    sampler: String,
    sample_ratio: f64,
    metrics_interval_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    if !enabled || endpoint.is_none() {
        // Initialize standard tracing without OpenTelemetry
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "mindia=debug,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        tracing::info!("OpenTelemetry disabled, using standard tracing");
        return Ok(());
    }

    let endpoint = endpoint.ok_or_else(|| {
        anyhow::anyhow!("OTEL_ENDPOINT not configured. Set OTEL_ENDPOINT environment variable or disable OpenTelemetry with OTEL_ENABLED=false")
    })?;

    // Get hostname for resource attributes
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    // Generate service instance ID (optional, for multi-instance deployments)
    let instance_id =
        env::var("OTEL_SERVICE_INSTANCE_ID").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

    // Create resource with service information and semantic conventions
    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, service_name.clone()),
        KeyValue::new(SERVICE_VERSION, service_version.clone()),
        KeyValue::new("deployment.environment", environment.clone()),
        KeyValue::new("host.name", hostname.clone()),
        KeyValue::new("service.instance.id", instance_id.clone()),
    ]);

    // Configure sampler based on configuration
    let sampler_config = match sampler.as_str() {
        "always_off" => Sampler::AlwaysOff,
        "trace_id_ratio" => {
            let ratio = sample_ratio.clamp(0.0, 1.0);
            if ratio <= 0.0 {
                tracing::warn!("OTEL_SAMPLE_RATIO is 0.0 or negative, using AlwaysOff sampler");
                Sampler::AlwaysOff
            } else if ratio >= 1.0 {
                tracing::info!("OTEL_SAMPLE_RATIO is 1.0, using AlwaysOn sampler");
                Sampler::AlwaysOn
            } else {
                tracing::info!(ratio = ratio, "Using TraceIdRatioBased sampler");
                Sampler::TraceIdRatioBased(ratio)
            }
        }
        _ => {
            // Default to AlwaysOn
            if sampler != "always_on" {
                tracing::warn!(
                    sampler = %sampler,
                    "Unknown sampler type, defaulting to AlwaysOn"
                );
            }
            Sampler::AlwaysOn
        }
    };

    // Build OTLP span exporter (HTTP or gRPC)
    let span_exporter = if protocol == "http" {
        opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .build()
            .map_err(|e| format!("Failed to build HTTP span exporter: {}", e))?
    } else {
        opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build()
            .map_err(|e| format!("Failed to build gRPC span exporter: {}", e))?
    };

    // Tracer provider with batch processor
    let batch_processor =
        BatchSpanProcessor::builder(span_exporter, opentelemetry_sdk::runtime::Tokio)
            .with_batch_config(BatchConfig::default())
            .build();

    let tracer_provider = sdktrace::TracerProvider::builder()
        .with_span_processor(batch_processor)
        .with_sampler(sampler_config)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource.clone())
        .build();

    let tracer = tracer_provider.tracer(service_name.clone());
    opentelemetry::global::set_tracer_provider(tracer_provider);

    // Build OTLP metric exporter (HTTP or gRPC)
    let metric_exporter = if protocol == "http" {
        opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .with_temporality(sdkmetrics::Temporality::Cumulative)
            .build()
            .map_err(|e| format!("Failed to build HTTP metric exporter: {}", e))?
    } else {
        opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .with_temporality(sdkmetrics::Temporality::Cumulative)
            .build()
            .map_err(|e| format!("Failed to build gRPC metric exporter: {}", e))?
    };

    let otlp_reader = PeriodicReader::builder(metric_exporter, opentelemetry_sdk::runtime::Tokio)
        .with_interval(Duration::from_secs(metrics_interval_secs))
        .build();

    let meter_provider = sdkmetrics::SdkMeterProvider::builder()
        .with_reader(otlp_reader)
        .with_resource(resource.clone())
        .build();

    opentelemetry::global::set_meter_provider(meter_provider);

    // Build OTLP log exporter and logger provider (for use by log bridges; global logger API was removed in 0.27)
    #[allow(dead_code)]
    let _logger_provider = if protocol == "http" {
        let log_exporter = opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .build()
            .map_err(|e| format!("Failed to build HTTP log exporter: {}", e))?;
        Some(
            sdklogs::LoggerProvider::builder()
                .with_resource(resource.clone())
                .with_batch_exporter(log_exporter, opentelemetry_sdk::runtime::Tokio)
                .build(),
        )
    } else {
        let log_exporter = opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build()
            .map_err(|e| format!("Failed to build gRPC log exporter: {}", e))?;
        Some(
            sdklogs::LoggerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(log_exporter, opentelemetry_sdk::runtime::Tokio)
                .build(),
        )
    };
    // Logger provider is not set globally (removed in opentelemetry 0.27). Keep _logger_provider if a log bridge is added later.

    // Create a tracing layer with OpenTelemetry
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Initialize tracing subscriber with OpenTelemetry layer
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mindia=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)
        .init();

    tracing::info!(
        endpoint = %endpoint,
        protocol = %protocol,
        environment = %environment,
        sampler = %sampler,
        sample_ratio = sample_ratio,
        metrics_interval_secs = metrics_interval_secs,
        hostname = %hostname,
        instance_id = %instance_id,
        "OpenTelemetry initialized successfully"
    );

    Ok(())
}

pub async fn shutdown_telemetry() {
    tracing::info!("Shutting down OpenTelemetry...");

    opentelemetry::global::shutdown_tracer_provider();

    // MeterProvider trait does not define force_flush; shutdown is best-effort on drop.
    // Global logger provider was removed in opentelemetry 0.27.

    tracing::info!("OpenTelemetry shutdown complete");
}
