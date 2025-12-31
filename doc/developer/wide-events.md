# Wide Event Logging

This document describes the wide event logging implementation following the principles from [loggingsucks.com](https://loggingsucks.com/).

## Overview

Wide events (also called "canonical log lines") are comprehensive, structured log events that capture all context for a single request in a single log line. Instead of logging what the code is doing, we log what happened to the request with full context.

## Benefits

1. **High Cardinality**: Each event includes tenant_id, user_id, and other high-cardinality fields for powerful querying
2. **High Dimensionality**: 50+ fields capture everything you need to debug without grep-ing through logs
3. **Tail Sampling**: Cost-effective sampling that always keeps errors, slow requests, and VIP users
4. **Single Source of Truth**: One event per request contains all context

## Architecture

### Middleware

The `wide_event_middleware` runs early in the middleware stack and:

1. **Initializes** the event with request metadata (method, path, IP, user agent, etc.)
2. **Captures** tenant context automatically (if available from auth middleware)
3. **Stores** the event in request extensions for handlers to enrich
4. **Finalizes** the event with response metadata (status code, duration)
5. **Applies** tail sampling based on outcome and configuration
6. **Emits** the event as a single structured JSON log line

### Event Structure

Each wide event includes:

```json
{
  "request_id": "req_8bf7ec2d",
  "trace_id": "abc123",
  "timestamp": "2025-01-15T10:23:45.612Z",
  "service": "mindia-api",
  "version": "1.0.0",
  "environment": "production",
  "method": "POST",
  "path": "/api/images",
  "status_code": 200,
  "duration_ms": 1247,
  "outcome": "success",
  "tenant": {
    "id": "uuid",
    "name": "Acme Corp",
    "status": "Active"
  },
  "user": {
    "id": "uuid",
    "role": "admin"
  },
  "business": {
    "media_type": "image",
    "media_id": "uuid",
    "file_size": 245891,
    "operation": "upload"
  },
  "error": null,
  "performance": {
    "db_queries": 3,
    "cache_hit": false
  },
  "storage": {
    "operation": "upload",
    "bucket": "my-bucket",
    "size_bytes": 245891,
    "duration_ms": 892
  }
}
```

## Tail Sampling

Tail sampling makes sampling decisions **after** the request completes, based on the outcome. This ensures we never lose important events.

### Sampling Rules

By default, the following events are **always kept** (100% sampling):

1. **All errors** (status_code >= 500)
2. **Slow requests** (duration > 2000ms, configurable via `WIDE_EVENT_SLOW_THRESHOLD_MS`)
3. **VIP tenants** (if `WIDE_EVENT_VIP_TENANT_IDS` is set)
4. **Debug paths** (if `WIDE_EVENT_KEEP_PATHS` is set)

All other events are randomly sampled at a configurable rate (default: 5% via `WIDE_EVENT_SAMPLE_RATE`).

### Configuration

Environment variables for tail sampling:

```bash
# Slow request threshold (ms) - requests slower than this are always kept
WIDE_EVENT_SLOW_THRESHOLD_MS=2000

# Random sampling rate for successful requests (0.0 to 1.0)
WIDE_EVENT_SAMPLE_RATE=0.05  # 5%

# Whether to keep all 4xx client errors (default: false)
WIDE_EVENT_KEEP_CLIENT_ERRORS=false

# Comma-separated list of VIP tenant IDs (always kept)
WIDE_EVENT_VIP_TENANT_IDS=uuid1,uuid2,uuid3

# Comma-separated list of paths to always keep (for debugging rollouts)
WIDE_EVENT_KEEP_PATHS=/api/checkout,/api/payment

# Always keep all events (for debugging, default: false)
WIDE_EVENT_ALWAYS_KEEP=false
```

## Enriching Events in Handlers

Handlers can enrich wide events with business context using helper functions:

```rust
use crate::middleware::wide_event::{enrich_wide_event_with_business};

pub async fn upload_image(
    State(state): State<Arc<AppState>>,
    tenant_ctx: TenantContext,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    // ... handler logic ...
    
    let media_id = create_image(...);
    
    // Enrich the wide event with business context
    // Note: Tenant context is automatically captured by the middleware
    if let Some(mut request) = /* get request somehow */ {
        enrich_wide_event_with_business(&mut request, |business| {
            business.media_type = Some("image".to_string());
            business.media_id = Some(media_id);
            business.operation = Some("upload".to_string());
            business.file_size = Some(file_size);
        });
    }
    
    Ok(Json(response))
}
```

**Note**: Currently, handlers need access to the `Request` object to enrich events. In most cases, tenant context is already captured automatically by the middleware, so enrichment is optional.

## Integration

The wide event middleware is automatically integrated into the route setup and runs after:

1. Request ID middleware
2. Security headers middleware  
3. Rate limiting middleware
4. Auth middleware (for protected routes)

And before:

- Analytics middleware (for database storage)

## Querying Wide Events

With wide events in structured JSON format, you can query them using log aggregation tools like:

- **Loki**: `{service="mindia-api"} | json | status_code >= 500`
- **Elasticsearch**: `service:mindia-api AND status_code:>=500 AND tenant.id:uuid`
- **ClickHouse**: `SELECT * FROM logs WHERE service = 'mindia-api' AND status_code >= 500`

Example queries:

```sql
-- All errors for a specific tenant
SELECT * FROM logs 
WHERE tenant.id = 'uuid' 
  AND status_code >= 500
  AND timestamp > now() - INTERVAL 1 HOUR;

-- Slow requests with specific business context
SELECT * FROM logs
WHERE duration_ms > 2000
  AND business.operation = 'upload'
  AND business.media_type = 'video';

-- Errors grouped by error type
SELECT error.type, COUNT(*) 
FROM logs
WHERE status_code >= 500
GROUP BY error.type;
```

## Migration from Traditional Logging

The wide event middleware runs alongside the existing analytics middleware. Both can coexist:

- **Wide events**: For observability, debugging, and log aggregation
- **Analytics middleware**: For business analytics stored in the database

Over time, you may choose to consolidate to wide events only and query them for both observability and analytics.

## Best Practices

1. **Don't log sensitive data**: Wide events automatically mask sensitive query parameters, but ensure handlers don't add sensitive business context
2. **Keep business context focused**: Only add business context that's useful for debugging (media IDs, operation types, etc.)
3. **Use tail sampling**: Don't disable tail sampling in production - adjust thresholds instead
4. **Monitor sampling rates**: Track the percentage of events being sampled to balance cost vs. observability
5. **Query before you grep**: With structured events, query them instead of grep-ing through text logs

## Troubleshooting

### Events not appearing in logs

- Check `WIDE_EVENT_SAMPLE_RATE` - successful requests may be sampled out
- Verify the path is not excluded in `should_log_request()`
- Check if `WIDE_EVENT_ALWAYS_KEEP=false` is accidentally set

### Missing tenant context

- Tenant context is automatically captured for protected routes (after auth middleware)
- Public routes won't have tenant context, which is expected
- Ensure auth middleware runs before wide_event_middleware (it does by default)

### High log volume

- Reduce `WIDE_EVENT_SAMPLE_RATE` (e.g., from 0.05 to 0.01 for 1% sampling)
- Increase `WIDE_EVENT_SLOW_THRESHOLD_MS` to sample out more slow requests
- Ensure tail sampling is working correctly (errors should always be kept)

## References

- [Logging Sucks - Your Logs Are Lying To You](https://loggingsucks.com/)
- [Stripe Engineering: Canonical Log Lines](https://stripe.com/blog/canonical-log-lines)