# API Reference

Complete reference for all Mindia API endpoints.

## Base URL

```
https://api.example.com
```

All endpoints use the versioned prefix **`/api/v0`** (e.g. `POST /api/v0/images`, `GET /api/v0/images`).

## Authentication

All endpoints (except `/health`) require authentication using the **master API key** or a **tenant API key**.

**Header**:
```
Authorization: Bearer <your-master-api-key-or-api-key>
```

- **Master API key**: Set via `MASTER_API_KEY`; used for all requests when no per-tenant keys are created.
- **API keys**: Create and manage via `/api/v0/api-keys`; keys start with `mk_live_`. See [API Keys](api-keys.md) for details.

## Images

### Upload Image

`POST /api/images`

**Query**: `?store=0|1|auto`

**Body**: `multipart/form-data` with `file` field

**Response**: `200 OK`
```json
{
  "id": "uuid",
  "filename": "uuid.jpg",
  "url": "https://...",
  "width": 1920,
  "height": 1080,
  "file_size": 1048576
}
```

### List Images

`GET /api/images`

**Query**: `?limit=50&offset=0`

**Response**: `200 OK` - Array of images

### Get Image

`GET /api/images/:id`

**Response**: `200 OK` - Single image object

### Download Image

`GET /api/images/:id/file`

**Response**: `200 OK` - Image bytes

### Transform Image

`GET /api/images/:id/{operations}`

**Operations**: `w_500/h_300/f_webp/q_high`

**Response**: `200 OK` - Transformed image bytes

### Delete Image

`DELETE /api/images/:id`

**Response**: `204 No Content`

## Videos

### Upload Video

`POST /api/videos`

**Body**: `multipart/form-data` with `file` field

**Response**: `200 OK`
```json
{
  "id": "uuid",
  "filename": "video.mp4",
  "processing_status": "pending"
}
```

### List Videos

`GET /api/videos`

**Query**: `?limit=50&offset=0`

### Get Video

`GET /api/videos/:id`

**Response**: `200 OK`
```json
{
  "id": "uuid",
  "processing_status": "completed",
  "hls_url": "https://.../master.m3u8",
  "variants": [...]
}
```

### Stream Video

- `GET /api/videos/:id/stream/master.m3u8`
- `GET /api/videos/:id/stream/:variant/index.m3u8`
- `GET /api/videos/:id/stream/:variant/:segment`

### Delete Video

`DELETE /api/videos/:id`

## Audio

### Upload Audio

`POST /api/audios`

**Response**: `200 OK`
```json
{
  "id": "uuid",
  "duration": 300.5,
  "bitrate": 128000,
  "channels": 2
}
```

### List/Get/Download/Delete

Same patterns as images:
- `GET /api/audios`
- `GET /api/audios/:id`
- `GET /api/audios/:id/file`
- `DELETE /api/audios/:id`

## Documents

### Upload Document

`POST /api/documents`

### List/Get/Download/Delete

Same patterns as images:
- `GET /api/documents`
- `GET /api/documents/:id`
- `GET /api/documents/:id/file`
- `DELETE /api/documents/:id`

## Search

### Semantic Search

`GET /api/search`

**Query**: `?q=sunset+beach&type=image&limit=20`

**Response**: `200 OK`
```json
{
  "query": "sunset beach",
  "results": [
    {
      "id": "uuid",
      "entity_type": "image",
      "description": "...",
      "similarity_score": 0.89
    }
  ],
  "count": 1
}
```

## API Keys

### Create API Key

`POST /api/api-keys`

**Request**:
```json
{
  "name": "Production API Key",
  "description": "Used for automated uploads",
  "expires_in_days": 365
}
```

**Response**: `201 Created`
```json
{
  "id": "uuid",
  "api_key": "mk_live_...",
  "name": "Production API Key",
  "expires_at": "2027-01-01T12:00:00Z"
}
```

### List API Keys

`GET /api/api-keys`

**Query**: `?limit=50&offset=0`

### Get API Key

`GET /api/api-keys/:id`

### Revoke API Key

`DELETE /api/api-keys/:id`

**Response**: `200 OK`
```json
{
  "message": "API key revoked successfully",
  "id": "uuid"
}
```

See [API Keys](api-keys.md) for complete guide.

## Webhooks (Admin Only)

### Create Webhook

`POST /api/webhooks`

**Request**:
```json
{
  "url": "https://example.com/hook",
  "events": ["image.uploaded"],
  "signing_secret": "secret"
}
```

### List/Update/Delete

- `GET /api/webhooks`
- `PATCH /api/webhooks/:id`
- `DELETE /api/webhooks/:id`

## Analytics

### Traffic Summary

`GET /api/analytics/traffic`

**Query**: `?start_date=2024-01-01&end_date=2024-01-31&limit=20`

**Response**: `200 OK`
```json
{
  "total_requests": 15234,
  "avg_response_time_ms": 45.2,
  "requests_per_method": {...},
  "popular_urls": [...]
}
```

### URL Statistics

`GET /api/analytics/urls`

### Storage Metrics

`GET /api/analytics/storage`

**Response**: `200 OK`
```json
{
  "total_files": 1543,
  "total_bytes": 524288000,
  "by_content_type": [...]
}
```

### Refresh Storage Metrics

`POST /api/analytics/storage/refresh`

## Error Responses

All errors return JSON:

```json
{
  "error": "Description of error"
}
```

**Status Codes**:
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `413` - Payload Too Large
- `422` - Unprocessable Entity
- `429` - Too Many Requests
- `500` - Internal Server Error

## Rate Limiting

**Headers** (when enabled):
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640000000
```

## Next Steps

- [Authentication](authentication.md)
- [API Keys](api-keys.md)
- [Images](images.md)
- [Videos](videos.md)
- [Best Practices](best-practices.md)

