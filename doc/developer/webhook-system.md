# Webhook System

Mindia's webhook system provides real-time event notifications to external services. This document explains the architecture, implementation, and retry logic.

## Overview

The webhook system enables:
- **Event Notifications**: Notify external services when events occur
- **Reliable Delivery**: Automatic retries with exponential backoff
- **Signature Verification**: HMAC-SHA256 signatures for security
- **Event Filtering**: Subscribe to specific event types
- **Delivery Tracking**: Track delivery status and history

## Architecture

```
Event Occurs → WebhookService → Find Active Webhooks → Deliver to Endpoints
                                      ↓
                              WebhookRetryService (for failures)
                                      ↓
                              Exponential Backoff
                                      ↓
                              Retry Delivery
```

## Components

### 1. Webhook Service (`src/services/webhook.rs`)

Main service for triggering and delivering webhooks.

**Key Features:**
- Event triggering
- HTTP delivery with timeout
- HMAC signature generation
- Delivery status tracking

**Configuration:**
```rust
pub struct WebhookServiceConfig {
    pub timeout_seconds: u64,    // Default: 30
    pub max_retries: i32,        // Default: 72 (~72 hours)
}
```

### 2. Webhook Retry Service (`src/services/webhook_retry.rs`)

Background service that processes failed webhook deliveries.

**Key Features:**
- Polls for due retries
- Batch processing
- Concurrent retry execution
- Exponential backoff

**Configuration:**
```rust
pub struct WebhookRetryServiceConfig {
    pub poll_interval_seconds: u64,      // Default: 30
    pub batch_size: i64,                 // Default: 100
    pub max_concurrent_retries: usize,   // Default: 10
}
```

### 3. Webhook Repository (`src/db/webhook.rs`)

Database operations for webhook management.

**Key Operations:**
- Create/update/delete webhooks
- Find active webhooks by event type
- List webhooks for tenant

### 4. Webhook Event Repository (`src/db/webhook.rs`)

Tracks webhook delivery events.

**Key Operations:**
- Create event record
- Update delivery status
- List events for webhook

### 5. Webhook Retry Repository (`src/db/webhook.rs`)

Manages retry queue for failed deliveries.

**Key Operations:**
- Create retry record
- Get due retries
- Update retry status
- Mark retry as processed

## Event Types

### Media Events

- `media.uploaded`: Media file uploaded
- `media.updated`: Media metadata updated
- `media.deleted`: Media file deleted

### Video Events

- `video.transcoding.started`: Video transcoding started
- `video.transcoding.completed`: Video transcoding completed
- `video.transcoding.failed`: Video transcoding failed

### Task Events

- `task.completed`: Background task completed
- `task.failed`: Background task failed

### Plugin Events

- `plugin.execution.started`: Plugin execution started
- `plugin.execution.completed`: Plugin execution completed
- `plugin.execution.failed`: Plugin execution failed

## Webhook Delivery

### HTTP Request Format

**Method:** POST
**Content-Type:** `application/json`
**Headers:**
- `X-Mindia-Event`: Event type (e.g., `media.uploaded`)
- `X-Mindia-Signature`: HMAC-SHA256 signature
- `X-Mindia-Delivery-Id`: Unique delivery ID
- `X-Mindia-Timestamp`: Unix timestamp

**Body:**
```json
{
  "event": "media.uploaded",
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "type": "image",
    "filename": "photo.jpg",
    "url": "https://bucket.s3.amazonaws.com/uploads/...",
    "size": 1048576
  },
  "initiator": {
    "type": "user",
    "id": "660e8400-e29b-41d4-a716-446655440001"
  },
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### Signature Generation

HMAC-SHA256 signature for webhook security:

```rust
let payload_string = serde_json::to_string(&payload)?;
let mut mac = HmacSha256::new_from_slice(webhook.secret.as_bytes())?;
mac.update(payload_string.as_bytes());
let signature = hex::encode(mac.finalize().into_bytes());
```

**Verification (on receiver side):**
```javascript
const crypto = require('crypto');

function verifySignature(payload, signature, secret) {
  const hmac = crypto.createHmac('sha256', secret);
  hmac.update(JSON.stringify(payload));
  const expected = hmac.digest('hex');
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expected)
  );
}
```

## Retry Logic

### Exponential Backoff

Failed webhook deliveries are retried with exponential backoff:

- **Initial Delay**: 1 minute
- **Backoff Multiplier**: 2x
- **Max Delay**: 1 hour
- **Max Retries**: 72 (approximately 72 hours of retries)

### Retry Schedule

```
Attempt 1:  Immediate
Attempt 2:  +1 minute
Attempt 3:  +2 minutes
Attempt 4:  +4 minutes
Attempt 5:  +8 minutes
...
Attempt N:  +1 hour (max)
```

### Retry Conditions

Webhooks are retried if:
- HTTP status code is 5xx (server error)
- Network timeout
- Connection error
- HTTP status code is 429 (rate limit)

Webhooks are NOT retried if:
- HTTP status code is 2xx (success)
- HTTP status code is 4xx (client error, except 429)
- Max retries exceeded

## Delivery Status

### Status Values

- **`pending`**: Delivery not yet attempted
- **`delivered`**: Successfully delivered (2xx response)
- **`failed`**: Delivery failed (will be retried)
- **`permanently_failed`**: Max retries exceeded

### Status Tracking

Each delivery attempt is recorded:
- HTTP status code
- Response body (truncated)
- Delivery timestamp
- Error message (if any)

## Implementation

### Triggering a Webhook

```rust
// In your handler or service
webhook_service
    .trigger_event(
        tenant_id,
        WebhookEventType::MediaUploaded,
        WebhookDataInfo::Image {
            id: image_id,
            filename: filename.clone(),
            url: s3_url.clone(),
            size: file_size,
            // ... other fields
        },
        WebhookInitiatorInfo::User { user_id },
    )
    .await?;
```

### Webhook Retry Worker

The retry service runs as a background task:

```rust
async fn worker_loop(
    retry_repo: WebhookRetryRepository,
    webhook_service: Arc<WebhookService>,
    config: WebhookRetryServiceConfig,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    let mut poll_interval = interval(Duration::from_secs(config.poll_interval_seconds));
    
    loop {
        tokio::select! {
            _ = poll_interval.tick() => {
                // Process batch of due retries
                process_retry_batch(&retry_repo, &webhook_service, &config).await;
            }
            _ = shutdown_rx.recv() => {
                break; // Shutdown
            }
        }
    }
}
```

## Database Schema

**Webhooks Table:**
```sql
CREATE TABLE webhooks (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    url TEXT NOT NULL,
    secret TEXT NOT NULL,
    event_types TEXT[] NOT NULL,
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Webhook Events Table:**
```sql
CREATE TABLE webhook_events (
    id UUID PRIMARY KEY,
    webhook_id UUID NOT NULL REFERENCES webhooks(id),
    event_type VARCHAR(50) NOT NULL,
    delivery_id UUID NOT NULL,
    status VARCHAR(20) NOT NULL,
    http_status INTEGER,
    response_body TEXT,
    error_message TEXT,
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Webhook Retries Table:**
```sql
CREATE TABLE webhook_retries (
    id UUID PRIMARY KEY,
    webhook_id UUID NOT NULL REFERENCES webhooks(id),
    event_id UUID NOT NULL REFERENCES webhook_events(id),
    retry_count INTEGER DEFAULT 0,
    next_retry_at TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

## Configuration

### Environment Variables

```env
# Webhook service configuration
WEBHOOK_TIMEOUT_SECONDS=30
WEBHOOK_MAX_RETRIES=72

# Webhook retry service configuration
WEBHOOK_RETRY_POLL_INTERVAL_SECONDS=30
WEBHOOK_RETRY_BATCH_SIZE=100
WEBHOOK_RETRY_MAX_CONCURRENT=10
```

## Security Best Practices

1. **HTTPS Only**: Always use HTTPS endpoints for webhooks
2. **Secret Management**: Use strong, unique secrets for each webhook
3. **Signature Verification**: Always verify HMAC signatures
4. **Idempotency**: Make webhook handlers idempotent
5. **Rate Limiting**: Implement rate limiting on receiver side
6. **Timeout Handling**: Set appropriate timeouts for webhook delivery

## Error Handling

### Delivery Failures

When a webhook delivery fails:
1. Event status set to `failed`
2. Retry record created with `next_retry_at`
3. Retry service picks up due retries
4. Retry attempted with exponential backoff
5. If max retries exceeded, status set to `permanently_failed`

### Common Errors

- **Timeout**: Endpoint didn't respond within timeout
- **Connection Error**: Network or DNS issues
- **5xx Errors**: Server errors (retried)
- **4xx Errors**: Client errors (not retried, except 429)

## Monitoring

### Webhook Statistics

Track webhook performance:
- Delivery success rate
- Average delivery time
- Retry counts
- Failure reasons

### Logging

All webhook operations are logged:
- Event triggers
- Delivery attempts
- Success/failure
- Retry scheduling

## Testing Webhooks

### Local Testing

Use tools like [ngrok](https://ngrok.com/) to expose local endpoints:

```bash
ngrok http 3000
# Use ngrok URL as webhook endpoint
```

### Webhook Testing Endpoint

Mindia provides a test endpoint:

```bash
POST /api/webhooks/{id}/test
```

This triggers a test event to verify webhook configuration.

## Best Practices

1. **Idempotent Handlers**: Webhook receivers should be idempotent
2. **Quick Response**: Respond quickly (within timeout) to avoid retries
3. **Error Handling**: Return appropriate HTTP status codes
4. **Logging**: Log all webhook deliveries for debugging
5. **Monitoring**: Monitor delivery success rates
6. **Secret Rotation**: Periodically rotate webhook secrets

## Related Documentation

- [Webhooks](../user/webhooks.md) - User guide for webhooks
- [Job Queue](job-queue.md) - Background task processing
- [Authentication System](authentication-system.md) - Security and authentication

