# Analytics API Endpoints

This folder contains Bruno requests for querying analytics data from the mindia API.

## Available Endpoints

### Traffic Analytics
- **Get Traffic Summary** - Comprehensive traffic statistics with popular URLs
- **Get URL Statistics** - Detailed statistics per URL endpoint

### Storage Analytics
- **Get Storage Summary** - Current storage usage by file type
- **Refresh Storage Metrics** - Manually recalculate storage metrics

## Query Parameters

All traffic endpoints support optional filtering:

- `start_date` - Filter from date (ISO 8601 format, e.g., `2024-01-01T00:00:00Z`)
- `end_date` - Filter to date (ISO 8601 format)
- `limit` - Number of results to return (default: 20)

## Examples

### Get last 7 days of traffic
```
GET /api/analytics/traffic?start_date=2024-01-24T00:00:00Z&limit=10
```

### Get top 5 URLs
```
GET /api/analytics/urls?limit=5
```

### Get traffic for January 2024
```
GET /api/analytics/traffic?start_date=2024-01-01T00:00:00Z&end_date=2024-01-31T23:59:59Z
```

## Metrics Tracked

### Traffic Metrics
- Total requests
- Bytes sent (downloads) and received (uploads)
- Average response time
- Requests per HTTP method (GET, POST, DELETE)
- Requests per status code (200, 404, 500, etc.)
- Popular URLs with detailed statistics

### URL-Specific Metrics
- Request count
- Total bytes transferred
- Response time (average, min, max)
- Status code distribution (2xx, 3xx, 4xx, 5xx)

### Storage Metrics
- Total file count
- Total storage bytes
- Image count and bytes
- Video count and bytes
- Breakdown by content type (MIME type)

## Path Normalization

URLs are normalized for meaningful grouping:
- `/api/images/123e4567-e89b-12d3-a456-426614174000/file` → `/api/images/:id/file`
- This allows aggregation of requests to the same endpoint with different IDs

## Caching

Storage metrics are cached and refreshed every 6 hours automatically.
Use the "Refresh Storage Metrics" endpoint to force an immediate update.

## Performance

- Request logging is asynchronous and doesn't impact API performance
- Analytics queries are optimized with database indexes
- Storage calculations are cached to avoid expensive recomputation

