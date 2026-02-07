# Batch Operations

Batch endpoints let you delete or copy (duplicate) multiple media items in a single request, with per-item results so you can see which succeeded or failed.

## Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v0/media/batch/delete` | Delete up to 50 media items by ID |
| `POST` | `/api/v0/media/batch/copy` | Duplicate up to 50 media items (new ID, copied file, same metadata) |

Both require authentication and return a list of results, one per requested ID. Maximum batch size is **50**; larger requests return `400 BATCH_SIZE_EXCEEDED`.

## Batch Delete

Request body:

```json
{
  "ids": [
    "550e8400-e29b-41d4-a716-446655440000",
    "660e8400-e29b-41d4-a716-446655440001"
  ]
}
```

Response (one entry per ID):

```json
{
  "results": [
    { "id": "550e8400-e29b-41d4-a716-446655440000", "status": 204 },
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "status": 404,
      "error": "Media not found"
    }
  ]
}
```

- **204**: Item was deleted (storage and DB; type-specific cleanup such as HLS or embeddings is performed).
- **404**: No media with that ID for the tenant.
- **500**: Server error; `error` contains the message.

Audit logs and webhooks (`file.deleted`) are emitted for each successful delete.

## Batch Copy

Request body:

```json
{
  "ids": [
    "550e8400-e29b-41d4-a716-446655440000",
    "660e8400-e29b-41d4-a716-446655440001"
  ]
}
```

Response (one entry per source ID):

```json
{
  "results": [
    {
      "source_id": "550e8400-e29b-41d4-a716-446655440000",
      "new_id": "770e8400-e29b-41d4-a716-446655440002",
      "status": 201
    },
    {
      "source_id": "660e8400-e29b-41d4-a716-446655440001",
      "new_id": null,
      "status": 404,
      "error": "Media not found"
    }
  ]
}
```

- **201**: Copy created; `new_id` is the new media UUID. The copy has the same metadata and type; the file is duplicated in storage. The new record has `store_permanently: true` and `expires_at: null`.
- **404**: No media with that ID for the tenant.
- **500**: Server error; `error` contains the message.

Webhooks are sent as `file.uploaded` with `initiator.type` set to `"copy"` for each successful copy.

## Example: cURL

**Batch delete:**

```bash
curl -X POST "https://your-api/api/v0/media/batch/delete" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"ids": ["550e8400-e29b-41d4-a716-446655440000"]}'
```

**Batch copy:**

```bash
curl -X POST "https://your-api/api/v0/media/batch/copy" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"ids": ["550e8400-e29b-41d4-a716-446655440000"]}'
```

## Behavior

- Operations run **sequentially**; each item is fully processed before the next.
- Failures are isolated: one failed ID does not stop the rest; check each `results` entry.
- Batch delete uses the same logic as single `DELETE /api/v0/media/:id` (HLS cleanup for videos, embedding cleanup for audio, audit, webhooks).
- Batch copy creates a new storage object and a new media row; user metadata, type metadata, and folder are copied. For videos, HLS assets are not copied; the new video has the same type metadata and may need processing to be re-run if you rely on HLS.
