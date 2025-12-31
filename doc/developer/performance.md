# Performance

Performance optimization techniques and considerations for Mindia.

## Key Metrics

**Target Performance**:
- Image upload: < 2s for 5MB
- Image transformation: < 200ms (cold), < 50ms (warm)
- Video upload: < 5s for 50MB
- API response: < 100ms (p95)
- Search query: < 500ms

## Database Optimization

### Connection Pooling

```rust
// config.rs
let pool = PgPoolOptions::new()
    .max_connections(20)
    .acquire_timeout(Duration::from_secs(5))
    .connect(&database_url)
    .await?;
```

### Indexes

Critical indexes:

```sql
-- Vector search (IVFFlat or HNSW)
CREATE INDEX idx_embeddings_vector 
ON embeddings USING ivfflat (embedding vector_cosine_ops);

-- Tenant-scoped queries
CREATE INDEX idx_images_tenant ON images(tenant_id, uploaded_at DESC);

-- Analytics queries
CREATE INDEX idx_request_logs_created ON request_logs(created_at DESC);
```

### Query Optimization

```rust
// ❌ Bad: N+1 queries
for image in images {
    let embedding = get_embedding(image.id).await;
}

// ✅ Good: Single query with JOIN
let images_with_embeddings = sqlx::query!(
    "SELECT i.*, e.embedding 
     FROM images i 
     LEFT JOIN embeddings e ON e.entity_id = i.id
     WHERE i.tenant_id = $1",
    tenant_id
).fetch_all(&pool).await?;
```

## Image Processing

### Async Processing

```rust
// Spawn blocking for CPU-intensive work
let processed = tokio::task::spawn_blocking(move || {
    image::load_from_memory(&bytes)?
        .resize(width, height, FilterType::Lanczos3)
        .into_bytes()
}).await??;
```

### Format Selection

```rust
// Use WebP for smaller file sizes
match format {
    "webp" => encode_webp(&img),  // Smallest
    "jpeg" => encode_jpeg(&img),  // Good compression
    "png" => encode_png(&img),    // Lossless
}
```

## Video Transcoding

### Hardware Acceleration

```rust
// FFmpeg with hardware encoding
let codec = if nvidia_available() {
    "h264_nvenc"  // NVIDIA
} else if qsv_available() {
    "h264_qsv"    // Intel Quick Sync
} else {
    "libx264"     // Software fallback
};
```

### Concurrency

```rust
// Process multiple qualities concurrently
let variants = vec!["1080p", "720p", "480p"];
let futures: Vec<_> = variants.iter()
    .map(|variant| transcode_variant(input, *variant))
    .collect();

let results = futures::future::join_all(futures).await;
```

## Caching

### HTTP Cache Headers

```rust
// Immutable transformations (1 year cache)
let headers = HeaderMap::new();
headers.insert(
    CACHE_CONTROL,
    "public, max-age=31536000, immutable".parse()?
);
```

### Application-Level Cache

```rust
use moka::future::Cache;

let cache: Cache<String, Vec<u8>> = Cache::builder()
    .max_capacity(1000)
    .time_to_live(Duration::from_secs(3600))
    .build();

// Cache expensive operations
if let Some(cached) = cache.get(&key).await {
    return Ok(cached);
}

let result = expensive_operation().await?;
cache.insert(key, result.clone()).await;
```

## S3 Optimization

### Multipart Uploads

For large files (> 5MB):

```rust
// Automatic multipart for large uploads
if file_size > 5_000_000 {
    let multipart = s3_client
        .create_multipart_upload()
        .bucket(&bucket)
        .key(&key)
        .send()
        .await?;

    // Upload parts concurrently
    // ...
}
```

### Presigned URLs

For direct client uploads:

```rust
// Generate presigned URL (no server proxy)
let presigned = s3_client
    .put_object()
    .bucket(&bucket)
    .key(&key)
    .presigned(Duration::from_secs(3600))
    .await?;
```

## Async Patterns

### Concurrent Operations

```rust
// Process multiple items concurrently
let results = futures::stream::iter(items)
    .map(|item| process_item(item))
    .buffer_unordered(10)  // Limit concurrency
    .collect::<Vec<_>>()
    .await;
```

### Timeouts

```rust
use tokio::time::timeout;

let result = timeout(
    Duration::from_secs(30),
    long_running_operation()
).await??;
```

## Monitoring

### Telemetry

```rust
use tracing::{instrument, info};

#[instrument(skip(data))]
async fn process_image(data: Vec<u8>) -> Result<Image> {
    let start = Instant::now();
    
    let result = do_processing(data).await?;
    
    info!(
        duration_ms = start.elapsed().as_millis(),
        "Image processed"
    );
    
    Ok(result)
}
```

### Metrics

Key metrics to track:
- Request latency (p50, p95, p99)
- Error rate
- Database query time
- S3 operation time
- Transcoding time
- Queue depth

## Profiling

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin mindia

# View flamegraph.svg
```

### Memory Profiling

```bash
# Use valgrind
valgrind --tool=massif ./target/release/mindia

# Or heaptrack
heaptrack ./target/release/mindia
```

## Scalability

### Horizontal Scaling

Mindia is stateless and can scale horizontally:

1. Run multiple instances behind load balancer
2. Share PostgreSQL and S3
3. Use distributed task queue
4. CDN for static assets

### Database Scaling

- Read replicas for analytics
- Connection pooling
- Partitioning large tables
- Archive old data

### Storage Scaling

S3 scales automatically. Consider:
- Lifecycle policies for old media
- CloudFront CDN
- Cross-region replication

## Next Steps

- [Architecture](architecture.md) - System design
- [Deployment](deployment.md) - Production deployment
- [Monitoring](monitoring.md) - Observability setup

