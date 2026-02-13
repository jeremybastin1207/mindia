# Grafana Observability Stack

This directory contains configuration files for running Grafana with OpenTelemetry collection for Mindia using **Grafana Alloy**.

## Structure

- `docker-compose.yml` - Docker Compose configuration for the entire stack
- `alloy-config.river` - Grafana Alloy configuration (River language)
- `tempo-config.yaml` - Tempo (trace storage) configuration
- `prometheus-config.yaml` - Prometheus (metrics) configuration
- `provisioning/datasources/datasources.yaml` - Grafana datasource provisioning

## Usage

Start the observability stack:

```bash
cd .grafana
docker-compose up -d
```

Or from the project root:

```bash
docker-compose -f .grafana/docker-compose.yml up -d
```

Access Grafana at: http://localhost:3001

## Configuration

**Grafana Alloy** receives OpenTelemetry data from Mindia on:
- Port `4317` (gRPC) - default OTLP endpoint
- Port `4318` (HTTP) - alternative protocol
- Port `8888` - Prometheus metrics endpoint (scraped by Prometheus)
- Port `12345` - Alloy UI/metrics endpoint

Make sure your Mindia `.env` file has:

```bash
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_EXPORTER_OTLP_PROTOCOL=grpc
OTEL_SERVICE_NAME=mindia-api
```

## Services

- **Grafana**: http://localhost:3001 (visualization)
- **Prometheus**: http://localhost:9090 (metrics)
- **Tempo**: http://localhost:3200 (traces)
- **Grafana Alloy**: Ports 4317 (gRPC), 4318 (HTTP), 8888 (Prometheus metrics), 12345 (UI)

## Why Grafana Alloy?

Grafana Alloy is the modern observability agent from Grafana Labs that:
- Provides better integration with Grafana ecosystem
- Supports OTLP, Prometheus, Loki, and more
- Uses River configuration language for declarative config
- Offers better performance and resource efficiency
- Is actively maintained and developed by Grafana Labs
