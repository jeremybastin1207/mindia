# Tech Stack

Technologies used in Mindia and why they were chosen.

## Core Technologies

### Rust
**Why**: Memory safety, performance, strong type system, excellent async support

**Alternatives Considered**: Go (less type safety), Node.js (slower), Python (slower)

**Trade-offs**: Longer compile times, steeper learning curve

### Axum (Web Framework)
**Why**: Modern, ergonomic, built on Tower, excellent async support

**Alternatives**: Actix-web, Rocket, Warp

### Tokio (Async Runtime)
**Why**: Industry standard, mature, excellent ecosystem

**Alternatives**: async-std (smaller ecosystem)

## Database

### PostgreSQL
**Why**: Robust, ACID compliance, excellent extensions

**Key Features**:
- pgvector extension for semantic search
- JSON support
- Full-text search
- Battle-tested reliability

**Alternatives Considered**: MySQL (no pgvector), MongoDB (no strong consistency)

### sqlx
**Why**: Compile-time SQL verification, async support, connection pooling

**Alternatives**: Diesel (compile-time but not async-native), SeaORM

## Storage

### AWS S3
**Why**: Industry standard, reliable, CDN-friendly, scalable

**Benefits**:
- Unlimited scaling
- High durability (99.999999999%)
- Versioning support
- Lifecycle policies
- CloudFront integration

**Alternatives**: Local storage (not scalable), MinIO (self-hosted)

## Media Processing

### image-rs
**Why**: Pure Rust, safe, good performance, wide format support

**Formats**: JPEG, PNG, GIF, WebP

### FFmpeg
**Why**: Industry standard for video, hardware acceleration, HLS support

**Features**: H.264/H.265 encoding, adaptive bitrate, multiple qualities

### mozjpeg / ravif
**Why**: Better compression than standard encoders

## AI/ML

### Anthropic (Claude)
**Why**: Cloud API for embeddings and vision, no local GPU required

**Usage**:
- Embeddings API: Vector generation for semantic search
- Messages API: Vision and document summarization (Claude models)

**Configuration**: `ANTHROPIC_API_KEY`, `ANTHROPIC_VISION_MODEL`, `ANTHROPIC_EMBEDDING_MODEL`

### pgvector
**Why**: Native PostgreSQL extension, fast, scales well

**Features**: Cosine similarity, IVFFlat/HNSW indexes

## Observability

### OpenTelemetry
**Why**: Vendor-neutral, traces+metrics+logs, industry standard

**Benefits**: Works with Jaeger, Prometheus, Grafana Cloud, Honeycomb, etc.

### tracing
**Why**: Rust-native, async-aware, structured logging

## Background Job Queue

### PostgreSQL-backed Queue
**Why**: Leverages existing PostgreSQL, persistence, ACID transactions

**Features**:
- LISTEN/NOTIFY for instant task notifications (low latency)
- FOR UPDATE SKIP LOCKED for multi-instance coordination
- Automatic retry with exponential backoff
- Task dependencies and scheduling
- Worker pool with concurrency control

**Alternatives**: Redis/Sidekiq (extra infrastructure), dedicated message broker (overkill)

**Implementation**: Custom queue in `mindia-worker` crate using tokio and sqlx

## Dependencies

See `Cargo.toml` for complete list. Key dependencies:

```toml
axum = "0.7"              # Web framework
tokio = "1"               # Async runtime (worker pool, async I/O)
sqlx = "0.8"              # Database with LISTEN/NOTIFY
aws-sdk-s3 = "1.13"       # S3 client
image = "0.24"            # Image processing
jsonwebtoken = "9"        # JWT
bcrypt = "0.15"           # Password hashing
opentelemetry = "0.27"    # Observability
```

## Design Principles

1. **Type Safety**: Leverage Rust's type system
2. **Async First**: Everything async for performance
3. **Zero-Copy**: Minimize data copying where possible
4. **Fail Fast**: Compile-time checks over runtime errors
5. **Observable**: Comprehensive instrumentation
6. **Scalable**: Stateless, horizontally scalable design

## Next Steps

- [Architecture](architecture.md) - System design
- [Development Setup](development-setup.md) - Get started
- [Performance](performance.md) - Optimization techniques

