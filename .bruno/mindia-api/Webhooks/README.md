# Webhooks

Webhooks allow you to receive HTTP callbacks when events occur in the system. Instead of polling for changes, your application receives real-time notifications.

## Overview

Webhooks send POST requests to your configured endpoint with event data. Each webhook subscribes to a specific event type (e.g., `file.uploaded`, `file.deleted`).

## Features

- **Event Types**: Subscribe to file lifecycle events
- **Security**: HMAC signature validation with signing secrets
- **Reliability**: Automatic retries with exponential backoff
- **Monitoring**: Event logs for debugging delivery issues
- **Multi-tenant**: Isolated by tenant ID

## Event Types

### Core Events
- `file.uploaded` - Any file was uploaded (images, videos, audio, documents)
- `file.deleted` - Any file was deleted
- `file.processing_completed` - File processing finished (success or failure)

### Media-Specific Aliases
These map to core events:
- `image.uploaded`, `video.uploaded`, `audio.uploaded`, `document.uploaded` → `file.uploaded`
- `image.deleted`, `video.deleted`, `audio.deleted`, `document.deleted` → `file.deleted`
- `video.completed`, `video.failed` → `file.processing_completed`

## Payload Format

```json
{
  "event_id": "uuid-of-event-log",
  "event_type": "file.uploaded",
  "timestamp": "2026-01-30T12:00:00Z",
  "tenant_id": "uuid-of-tenant",
  "data": {
    "media_id": "uuid-of-file",
    "media_type": "image",
    "filename": "example.jpg",
    "size_bytes": 1048576,
    "url": "https://cdn.example.com/files/...",
    ...
  }
}
```

## Security

### SSRF Protection
- Private/internal URLs are rejected (localhost, 127.0.0.1, 10.x.x.x, 192.168.x.x, etc.)
- Only HTTPS endpoints allowed in production

### HMAC Signatures
When a `signing_secret` is configured, webhooks include an HMAC signature:

```
X-Webhook-Signature: sha256=<hmac-hex-digest>
```

To verify (pseudocode):
```javascript
const signature = request.headers['x-webhook-signature'];
const expectedSignature = 'sha256=' + hmac_sha256(signing_secret, request.body);
if (signature !== expectedSignature) {
  throw new Error('Invalid signature');
}
```

## Retry Logic

Failed webhook deliveries are automatically retried:
- **Max retries**: Configurable (default: 3)
- **Backoff**: Exponential (e.g., 1min, 5min, 15min)
- **Timeout**: Configurable (default: 30 seconds)

After max retries, the webhook is marked as failed and no further attempts are made.

## Testing

Use [webhook.site](https://webhook.site) to quickly test webhooks:

1. Visit webhook.site and copy your unique URL
2. Create a webhook pointing to that URL
3. Trigger an event (e.g., upload a file)
4. Check webhook.site to see the delivered payload

## Best Practices

1. **Idempotency**: Events may be delivered more than once. Use `event_id` to deduplicate.
2. **Quick Response**: Respond with 2xx status quickly. Process payload asynchronously.
3. **Verify Signatures**: Always validate HMAC signatures in production.
4. **Monitor Events**: Use "List Webhook Events" to debug delivery issues.
5. **Error Handling**: Return 2xx for successful receipt, even if processing fails later.

## Configuration

Environment variables (server-side):
- `WEBHOOK_TIMEOUT_SECONDS` - Request timeout (default: 30)
- `WEBHOOK_MAX_RETRIES` - Max retry attempts (default: 3)
- `WEBHOOK_MAX_CONCURRENT_DELIVERIES` - Concurrent deliveries (default: 10)
- `WEBHOOK_RETRY_POLL_INTERVAL_SECONDS` - Retry poll interval (default: 60)
