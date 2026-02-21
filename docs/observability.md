# Observability (OpenTelemetry)

Mindia supports OpenTelemetry for traces and metrics when the `observability-opentelemetry` feature is enabled (included in the default and `full` feature sets).

## Enabling OpenTelemetry

Set these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `OTEL_ENABLED` | Enable OpenTelemetry | `false` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP endpoint (e.g. `http://localhost:4317` for gRPC, `http://localhost:4318/v1/traces` for HTTP) | — |
| `OTEL_SERVICE_NAME` | Service name in traces/metrics | `mindia` |
| `OTEL_SERVICE_VERSION` | Service version | `0.1.0` |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | Export protocol: `grpc` or `http` | `grpc` |
| `OTEL_SAMPLER` | Sampler: `always_on`, `always_off`, `trace_id_ratio` | `always_on` |
| `OTEL_SAMPLE_RATIO` | Sample ratio (0.0–1.0) when using `trace_id_ratio` | `1.0` |
| `OTEL_METRICS_INTERVAL_SECS` | Metrics export interval (seconds) | `30` |
| `OTEL_SERVICE_INSTANCE_ID` | Optional instance ID for multi-instance deployments | auto-generated UUID |

When `OTEL_ENABLED=false` or `OTEL_EXPORTER_OTLP_ENDPOINT` is unset, standard tracing (stdout) is used and no OTLP export occurs.

## Feature flag

Enable the feature at build time:

```bash
cargo build -p mindia-api --features observability-opentelemetry
```

It is included in the default and `full` feature profiles.

## OTLP backends

Configure `OTEL_EXPORTER_OTLP_ENDPOINT` for your collector or backend:

- **Jaeger**: `http://localhost:4317` (gRPC)
- **Grafana Tempo**: `http://localhost:4317` (gRPC) or `http://localhost:4318` (HTTP)
- **Honeycomb**: `https://api.honeycomb.io` (HTTP)
- **Datadog**: use their OTLP ingestion endpoint

## Production

1. Set `OTEL_ENABLED=true` and `OTEL_EXPORTER_OTLP_ENDPOINT` to your collector.
2. Use `trace_id_ratio` with `OTEL_SAMPLE_RATIO` &lt; 1.0 to reduce volume in high-traffic deployments.
3. Ensure the collector is reachable from the Mindia API (network/firewall).
