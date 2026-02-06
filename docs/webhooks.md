# Webhooks

Set up real-time event notifications for your Mindia instance.

## Table of Contents

- [Overview](#overview)
- [Webhook Events](#webhook-events)
- [Create Webhook](#create-webhook)
- [List Webhooks](#list-webhooks)
- [Update Webhook](#update-webhook)
- [Delete Webhook](#delete-webhook)
- [Webhook Payload](#webhook-payload)
- [Signature Verification](#signature-verification)
- [Retry Logic](#retry-logic)
- [Best Practices](#best-practices)

## Overview

Webhooks allow you to receive HTTP callbacks when specific events occur in your Mindia instance.

**Use Cases**:
- Trigger processing pipelines when media is uploaded
- Update external databases when files are deleted
- Send notifications when video transcoding completes
- Sync media metadata to other systems

**Features**:
- ✅ Multiple event types
- ✅ HMAC-SHA256 signature verification
- ✅ Automatic retry with exponential backoff
- ✅ Event history and logs
- ✅ Per-webhook configuration

**Permissions**: Webhooks require **admin** role

## Webhook Events

| Event | Description | Triggered When |
|-------|-------------|----------------|
| `image.uploaded` | Image uploaded | New image is successfully uploaded |
| `image.deleted` | Image deleted | Image is removed from system |
| `video.uploaded` | Video uploaded | New video is uploaded (before transcoding) |
| `video.completed` | Video transcoding done | Video HLS transcoding completes successfully |
| `video.failed` | Video transcoding failed | Video transcoding encounters an error |
| `video.deleted` | Video deleted | Video and its variants are removed |
| `audio.uploaded` | Audio uploaded | New audio file is uploaded |
| `audio.deleted` | Audio deleted | Audio file is removed |
| `document.uploaded` | Document uploaded | New document (PDF) is uploaded |
| `document.deleted` | Document deleted | Document is removed |

## Create Webhook

Register a new webhook endpoint.

### Endpoint

```
POST /api/webhooks
```

### Headers

```
Authorization: Bearer <token>
Content-Type: application/json
```

### Request Body

```json
{
  "url": "https://example.com/webhook",
  "events": ["image.uploaded", "video.completed"],
  "signing_secret": "your-secret-key",
  "is_active": true
}
```

**Fields**:
- `url` (required): HTTPS endpoint to receive webhooks
- `events` (required): Array of event types to subscribe to
- `signing_secret` (optional): Secret key for HMAC signature (auto-generated if not provided)
- `is_active` (optional): Enable/disable webhook (default: `true`)

### Response

**Status**: `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "url": "https://example.com/webhook",
  "events": ["image.uploaded", "video.completed"],
  "signing_secret": "your-secret-key",
  "is_active": true,
  "created_at": "2024-01-01T00:00:00Z"
}
```

### Examples

```bash
TOKEN="your-admin-token"

curl -X POST https://api.example.com/api/webhooks \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/webhook",
    "events": ["image.uploaded", "video.completed"],
    "signing_secret": "my-secret-key"
  }'
```

```javascript
async function createWebhook(url, events, secret) {
  const token = localStorage.getItem('token');

  const response = await fetch('https://api.example.com/api/webhooks', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      url,
      events,
      signing_secret: secret,
      is_active: true,
    }),
  });

  return await response.json();
}

// Usage
const webhook = await createWebhook(
  'https://myapp.com/mindia-webhook',
  ['image.uploaded', 'image.deleted'],
  'super-secret-key-12345'
);
```

## List Webhooks

Get all webhooks for your organization.

### Endpoint

```
GET /api/webhooks
```

### Headers

```
Authorization: Bearer <token>
```

### Response

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "url": "https://example.com/webhook",
    "events": ["image.uploaded", "video.completed"],
    "is_active": true,
    "created_at": "2024-01-01T00:00:00Z"
  }
]
```

**Note**: `signing_secret` is not returned for security reasons.

## Update Webhook

Modify an existing webhook.

### Endpoint

```
PATCH /api/webhooks/:id
```

### Headers

```
Authorization: Bearer <token>
Content-Type: application/json
```

### Request Body

```json
{
  "url": "https://newurl.com/webhook",
  "events": ["image.uploaded"],
  "is_active": false
}
```

All fields are optional. Only provided fields will be updated.

### Response

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "url": "https://newurl.com/webhook",
  "events": ["image.uploaded"],
  "is_active": false,
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-02T00:00:00Z"
}
```

## Delete Webhook

Remove a webhook.

### Endpoint

```
DELETE /api/webhooks/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `204 No Content`

## Webhook Payload

When an event occurs, Mindia sends an HTTP POST request to your webhook URL.

### Payload Structure

```json
{
  "hook": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "event": "image.uploaded",
    "target": "https://example.com/webhook",
    "project": "tenant-uuid",
    "created_at": "2024-01-01T00:00:00Z"
  },
  "data": {
    "type": "image",
    "id": "file-uuid",
    "filename": "photo.jpg",
    "url": "https://bucket.s3.amazonaws.com/...",
    "content_type": "image/jpeg",
    "file_size": 1048576,
    "width": 1920,
    "height": 1080
  },
  "initiator": {
    "user_id": "user-uuid",
    "ip": "192.168.1.1",
    "user_agent": "Mozilla/5.0..."
  }
}
```

### Headers

Mindia includes these headers in webhook requests:

| Header | Value | Description |
|--------|-------|-------------|
| `Content-Type` | `application/json` | Payload format |
| `User-Agent` | `Mindia-Webhook/1.0` | Identifies Mindia |
| `X-Uc-Signature` | `v1=<signature>` | HMAC-SHA256 signature |

### Event-Specific Payloads

**Image Uploaded**:
```json
{
  "hook": { ... },
  "data": {
    "type": "image",
    "id": "uuid",
    "filename": "photo.jpg",
    "url": "https://...",
    "content_type": "image/jpeg",
    "file_size": 1048576,
    "width": 1920,
    "height": 1080
  },
  "initiator": { ... }
}
```

**Video Completed**:
```json
{
  "hook": { ... },
  "data": {
    "type": "video",
    "id": "uuid",
    "filename": "video.mp4",
    "hls_url": "https://.../master.m3u8",
    "duration": 120.5,
    "variants": [...]
  },
  "initiator": { ... }
}
```

## Signature Verification

Mindia signs webhook payloads using HMAC-SHA256. Always verify signatures to ensure requests are authentic.

### Verification Process

1. Extract signature from `X-Uc-Signature` header
2. Compute HMAC-SHA256 of raw request body using your secret
3. Compare computed signature with provided signature

### Implementation Examples

**Node.js/Express**:

```javascript
const crypto = require('crypto');

function verifyWebhookSignature(req, secret) {
  const signature = req.headers['x-uc-signature'];
  
  if (!signature || !signature.startsWith('v1=')) {
    throw new Error('Missing or invalid signature');
  }

  const providedSignature = signature.substring(3); // Remove 'v1='
  const body = JSON.stringify(req.body);
  
  const hmac = crypto.createHmac('sha256', secret);
  hmac.update(body);
  const computedSignature = hmac.digest('hex');

  if (computedSignature !== providedSignature) {
    throw new Error('Invalid signature');
  }

  return true;
}

// Express route
app.post('/webhook', express.raw({ type: 'application/json' }), (req, res) => {
  try {
    verifyWebhookSignature(req, 'your-secret-key');
    
    const payload = JSON.parse(req.body);
    console.log('Received event:', payload.hook.event);
    
    // Process event
    handleWebhookEvent(payload);
    
    res.status(200).send('OK');
  } catch (error) {
    console.error('Webhook verification failed:', error);
    res.status(401).send('Unauthorized');
  }
});
```

**Python/Flask**:

```python
import hmac
import hashlib
from flask import Flask, request

app = Flask(__name__)

def verify_webhook_signature(request, secret):
    signature = request.headers.get('X-Uc-Signature', '')
    
    if not signature.startswith('v1='):
        raise ValueError('Missing or invalid signature')
    
    provided_signature = signature[3:]  # Remove 'v1='
    body = request.get_data()
    
    computed_hmac = hmac.new(
        secret.encode(),
        body,
        hashlib.sha256
    )
    computed_signature = computed_hmac.hexdigest()
    
    if not hmac.compare_digest(computed_signature, provided_signature):
        raise ValueError('Invalid signature')
    
    return True

@app.route('/webhook', methods=['POST'])
def webhook():
    try:
        verify_webhook_signature(request, 'your-secret-key')
        
        payload = request.get_json()
        print(f"Received event: {payload['hook']['event']}")
        
        # Process event
        handle_webhook_event(payload)
        
        return 'OK', 200
    except Exception as e:
        print(f'Webhook verification failed: {e}')
        return 'Unauthorized', 401
```

## Retry Logic

If your webhook endpoint fails (non-2xx response or timeout), Mindia automatically retries with exponential backoff.

**Retry Schedule**:
- Attempt 1: Immediately
- Attempt 2: After 1 minute
- Attempt 3: After 5 minutes
- Attempt 4: After 15 minutes
- Attempt 5: After 1 hour
- Continues up to 72 attempts (~72 hours)

**Timeout**: 30 seconds per request

**Success Criteria**: HTTP 2xx status code

**Best Practices**:
- Return 200 OK quickly (within 30s)
- Process webhook asynchronously in your system
- Don't perform long-running tasks in the webhook handler

## Best Practices

### 1. Verify Signatures

```javascript
// ✅ Good: Always verify signatures
app.post('/webhook', async (req, res) => {
  try {
    verifyWebhookSignature(req, process.env.WEBHOOK_SECRET);
    await processWebhook(req.body);
    res.status(200).send('OK');
  } catch (error) {
    res.status(401).send('Unauthorized');
  }
});

// ❌ Bad: Trust all requests
app.post('/webhook', async (req, res) => {
  await processWebhook(req.body); // Not safe!
  res.status(200).send('OK');
});
```

### 2. Process Asynchronously

```javascript
// ✅ Good: Queue and respond quickly
app.post('/webhook', async (req, res) => {
  verifyWebhookSignature(req, secret);
  
  // Add to queue
  await queue.add('webhook', req.body);
  
  // Respond immediately
  res.status(200).send('OK');
});

// Process in background worker
queue.process('webhook', async (job) => {
  const payload = job.data;
  await performLongRunningTask(payload);
});
```

### 3. Handle Idempotency

Webhooks may be delivered multiple times. Make your handler idempotent:

```javascript
const processedEvents = new Set();

async function handleWebhook(payload) {
  const eventId = payload.hook.id;
  
  // Skip if already processed
  if (processedEvents.has(eventId)) {
    console.log('Event already processed:', eventId);
    return;
  }

  // Process event
  await doSomething(payload);
  
  // Mark as processed
  processedEvents.add(eventId);
}
```

### 4. Log Webhook Events

```javascript
app.post('/webhook', async (req, res) => {
  const payload = req.body;
  
  console.log('Webhook received:', {
    event: payload.hook.event,
    id: payload.hook.id,
    data: payload.data.id,
    timestamp: new Date().toISOString(),
  });

  try {
    await processWebhook(payload);
    console.log('Webhook processed successfully');
    res.status(200).send('OK');
  } catch (error) {
    console.error('Webhook processing failed:', error);
    res.status(500).send('Error');
  }
});
```

### 5. Use HTTPS

```javascript
// ✅ Good: HTTPS endpoint
const webhookUrl = 'https://secure.example.com/webhook';

// ❌ Bad: HTTP endpoint (insecure)
const webhookUrl = 'http://example.com/webhook';
```

### 6. Handle Specific Events

```javascript
async function handleWebhookEvent(payload) {
  switch (payload.hook.event) {
    case 'image.uploaded':
      await handleImageUpload(payload.data);
      break;
    
    case 'video.completed':
      await handleVideoComplete(payload.data);
      break;
    
    case 'video.failed':
      await handleVideoFailure(payload.data);
      break;
    
    default:
      console.log('Unhandled event:', payload.hook.event);
  }
}
```

## Next Steps

- [Analytics](analytics.md) - Track usage and metrics
- [API Reference](api-reference.md) - Complete API documentation
- [Best Practices](best-practices.md) - Production deployment tips

