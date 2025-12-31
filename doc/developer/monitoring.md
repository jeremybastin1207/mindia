# Monitoring & Alerting Setup

Guide for setting up monitoring, alerting, and observability for Mindia in production.

## Overview

Mindia uses OpenTelemetry for comprehensive observability:
- **Traces**: Distributed request tracing
- **Metrics**: Performance and business metrics
- **Logs**: Structured application logs

## Metrics

### Key Metrics

#### Application Metrics

HTTP metrics are automatically collected for all requests when the `observability-opentelemetry` feature is enabled:

- `http.server.request.count` - Total HTTP requests (labeled by method, route, status_code)
- `http.server.request.duration` - Request latency histogram in seconds (labeled by method, route, status_code)
- `http.server.active_requests` - Current active requests (up/down counter, labeled by method, route)
- `http.server.errors.count` - Total HTTP errors (4xx and 5xx responses, labeled by method, route, status_code)

**Metric Labels:**
- `http.method`: HTTP method (GET, POST, PUT, DELETE, etc.)
- `http.route`: Matched route pattern (e.g., `/api/images/:id`)
- `http.status_code`: HTTP status code (200, 404, 500, etc.)

**Implementation Details:**
- Metrics are recorded automatically via Axum middleware
- Request start/end tracking with automatic duration calculation
- Error detection for status codes >= 400
- No performance impact when OpenTelemetry is disabled (feature flag)

#### Database Metrics

Database metrics infrastructure is defined but requires instrumentation at the query level:

- `db.client.queries.count` - Total database queries (labeled by operation, table)
- `db.client.queries.duration` - Query latency histogram in seconds (labeled by operation, table)
- `db.client.active_queries` - Current active queries (up/down counter, labeled by operation, table)

**Connection Pool Metrics** (automatically collected):
- `db.pool.size` - Maximum number of connections in the pool (gauge)
- `db.pool.idle` - Number of idle connections in the pool (gauge)
- `db.pool.active` - Number of active connections in the pool (gauge)

**Metric Labels:**
- `db.operation`: Query operation type (SELECT, INSERT, UPDATE, DELETE)
- `db.sql.table`: Database table name

**Note:** Connection pool metrics are automatically collected every 30 seconds when OpenTelemetry is enabled. Query-level metrics require manual instrumentation of database operations.

#### Storage Metrics

S3 operation metrics infrastructure is defined but requires instrumentation at the storage operation level:

- `aws.s3.operations.count` - Total S3 operations (labeled by operation, bucket)
- `aws.s3.operations.duration` - S3 operation latency histogram in seconds (labeled by operation, bucket)
- `aws.s3.bytes.transferred` - Total bytes transferred to/from S3 (labeled by operation, bucket)

**Metric Labels:**
- `aws.s3.operation`: Operation type (PutObject, GetObject, DeleteObject, etc.)
- `aws.s3.bucket`: S3 bucket name

**Note:** Storage metrics require wrapping storage operations or adding instrumentation to the storage implementation. Currently, storage operations are logged with tracing but not instrumented with metrics.

#### Business Metrics

- `mindia.uploads.count` - File uploads by type
- `mindia.transformations.count` - Image transformations
- `mindia.streams.count` - Video stream requests
- `mindia.storage.bytes` - Total storage used
- `mindia.files.count` - Total files stored

### Metric Collection

#### Prometheus

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'mindia'
    static_configs:
      - targets: ['localhost:8888']
    scrape_interval: 15s
```

#### Grafana Cloud

```env
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp-gateway-prod.grafana.net/otlp
OTEL_EXPORTER_OTLP_PROTOCOL=http
```

## Alerting Rules

### Critical Alerts

#### High Error Rate

```yaml
# Prometheus alert rule
- alert: HighErrorRate
  expr: rate(http_server_errors_count[5m]) > 10
  for: 5m
  annotations:
    summary: "High error rate detected"
    description: "Error rate is {{ $value }} errors/sec"
```

#### Database Connection Issues

```yaml
- alert: DatabaseConnectionFailure
  expr: db_connections_active == 0
  for: 1m
  annotations:
    summary: "Database connection pool exhausted"
```

#### High Latency

```yaml
- alert: HighLatency
  expr: histogram_quantile(0.95, http_server_request_duration_bucket) > 1
  for: 5m
  annotations:
    summary: "95th percentile latency exceeds 1s"
```

### Warning Alerts

#### Storage Quota Warning

```yaml
- alert: StorageQuotaWarning
  expr: mindia_storage_bytes > 1000000000000  # 1TB
  for: 1h
  annotations:
    summary: "Storage usage approaching limit"
```

#### High Memory Usage

```yaml
- alert: HighMemoryUsage
  expr: process_resident_memory_bytes / process_memory_limit_bytes > 0.8
  for: 5m
  annotations:
    summary: "Memory usage above 80%"
```

## Dashboards

### Application Dashboard

**Key Panels**:
- Request rate (requests/sec)
- Error rate (%)
- Latency (p50, p95, p99)
- Active requests
- Uploads per minute
- Storage usage

### Database Dashboard

**Key Panels**:
- Query rate
- Query latency
- Connection pool usage
- Slow queries
- Database size

### Storage Dashboard

**Key Panels**:
- Upload/download rate
- Storage operations latency
- Total storage used
- Files count
- Storage by type (images/videos/documents)

## Logging

### Log Levels

- **ERROR**: Application errors, failures
- **WARN**: Warnings, degraded functionality
- **INFO**: General information, important events
- **DEBUG**: Detailed debugging (development only)

### Structured Logging

All logs include:
- `timestamp`: Event timestamp
- `level`: Log level
- `message`: Log message
- `request_id`: Request correlation ID
- `tenant_id`: Tenant identifier (if applicable)
- `user_id`: User identifier (if applicable)

### Log Aggregation

#### JSON Format

```env
RUST_LOG=json
```

#### Log Shipping

- **Loki**: Log aggregation
- **Elasticsearch**: Search and analysis
- **CloudWatch Logs**: AWS log management
- **Datadog**: Log management and analysis

## Tracing

### Distributed Tracing

OpenTelemetry provides distributed tracing with automatic instrumentation:
- **HTTP Request Spans**: All HTTP requests are automatically traced with:
  - Method, route, status code
  - Client IP and user agent
  - Request/response content lengths
  - Duration and latency
- **Database Query Spans**: SQL operations are traced with query details
- **S3 Operation Spans**: Storage operations with bucket and operation type
- **External API Call Spans**: Plugin API calls (AssemblyAI, AWS Transcribe, etc.)
- **FFmpeg Processing Spans**: Video transcoding operations with variant details

**Span Attributes:**
- `http.method`, `http.route`, `http.status_code`
- `db.operation`, `db.sql.table`
- `aws.s3.operation`, `aws.s3.bucket`
- `task.type`, `task.status`

### Trace Sampling

For high-volume production:

```rust
// Sample 10% of traces
let sampler = TraceIdRatioBased::new(0.1);
```

## SLO/SLA Definitions

### Service Level Objectives

- **Availability**: 99.9% uptime
- **Latency**: p95 < 500ms for API requests
- **Error Rate**: < 0.1% error rate
- **Throughput**: Handle 1000 req/sec

### Service Level Indicators

- **Uptime**: `(total_time - downtime) / total_time`
- **Latency**: `histogram_quantile(0.95, request_duration)`
- **Error Rate**: `errors / total_requests`
- **Throughput**: `requests / time_window`

## Incident Response

### On-Call Procedures

1. **Alert Received**: Acknowledge alert
2. **Assess**: Check dashboards and logs
3. **Investigate**: Identify root cause
4. **Mitigate**: Apply fix or workaround
5. **Verify**: Confirm resolution
6. **Document**: Update runbook

### Escalation

- **Level 1**: On-call engineer (0-15 min)
- **Level 2**: Senior engineer (15-30 min)
- **Level 3**: Engineering lead (30+ min)

## Tools

### Monitoring

- **Prometheus**: Metrics collection
- **Grafana**: Dashboards and visualization
- **Jaeger**: Distributed tracing
- **Loki**: Log aggregation

### Cloud Services

- **Grafana Cloud**: Managed observability
- **Datadog**: Full-stack observability
- **New Relic**: Application performance monitoring
- **Honeycomb**: Observability platform

## Setup Instructions

### 1. Configure OpenTelemetry

```env
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=https://your-otel-endpoint
OTEL_SERVICE_NAME=mindia
OTEL_SERVICE_VERSION=1.0.0
```

### 2. Set Up Prometheus

```bash
# Install Prometheus
wget https://github.com/prometheus/prometheus/releases/download/v2.45.0/prometheus-2.45.0.linux-amd64.tar.gz

# Configure prometheus.yml
# Start Prometheus
./prometheus --config.file=prometheus.yml
```

### 3. Set Up Grafana

```bash
# Install Grafana
docker run -d -p 3000:3000 grafana/grafana

# Import dashboards
# Configure data sources
# Set up alerts
```

### 4. Configure Alerting

```yaml
# alertmanager.yml
route:
  receiver: 'slack'
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
receivers:
  - name: 'slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/...'
  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: '...'
```

## Best Practices

1. **Monitor Everything**: Track all critical metrics
2. **Set Appropriate Thresholds**: Avoid alert fatigue
3. **Use SLOs**: Define clear objectives
4. **Test Alerts**: Verify alerting works
5. **Document Runbooks**: Create response procedures
6. **Review Regularly**: Adjust thresholds based on data

## Health Check Endpoints

Mindia provides three health check endpoints for Kubernetes and orchestration compatibility:

### `/live` - Liveness Probe

Simple endpoint that returns 200 OK if the process is running. Always succeeds unless the application has crashed.

**Response:**
```json
{
  "status": "alive"
}
```

**Use Case:** Kubernetes liveness probe - restarts the container if the process dies.

### `/ready` - Readiness Probe

Checks if the service can accept traffic. Verifies critical dependencies:

**Response:**
```json
{
  "status": "ready",
  "database": "ready"
}
```

**Status Codes:**
- `200 OK`: Service is ready to accept traffic
- `503 Service Unavailable`: Critical dependencies unavailable

**Use Case:** Kubernetes readiness probe - removes pod from service load balancer if not ready.

**Dependencies Checked:**
- Database connectivity

**Timeout:** All dependency checks have a 5-second timeout to prevent hanging requests.

### `/health` - Comprehensive Health Check

Full health check including all components and optional services.

**Response:**
```json
{
  "status": "healthy",
  "database": "healthy",
  "storage": "healthy",
  "clamav": "healthy",
  "semantic_search": "healthy",
  "task_queue": "healthy"
}
```

**Status Codes:**
- `200 OK`: All critical components healthy
- `503 Service Unavailable`: One or more critical components unhealthy

**Dependencies Checked:**
- Database connectivity (critical)
- Storage connectivity (graceful degradation - doesn't fail overall health)
- ClamAV (optional - if enabled)
- Semantic Search/Anthropic (optional - if enabled)
- Task Queue connectivity

**Timeout:** All dependency checks have a 5-second timeout per dependency.

**Note:** Optional services (ClamAV, semantic search) are checked but don't cause overall health failure if they're unavailable.

## Next Steps

- [x] Add timeout handling to health checks
- [x] Separate liveness/readiness endpoints
- [x] Add error metrics (`http.server.errors.count`)
- [x] Add connection pool metrics
- [ ] Instrument database queries with DatabaseMetrics
- [ ] Instrument S3 operations with S3Metrics
- [ ] Add business metrics calls to handlers (uploads, transformations, streams)
- [ ] Set up monitoring infrastructure
- [ ] Configure metrics collection
- [ ] Create dashboards
- [ ] Set up alerting rules
- [ ] Test alerting
- [ ] Document runbooks

