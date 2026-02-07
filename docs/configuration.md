# Configuration Reference

Complete reference for all Mindia environment variables and configuration options.

**Runtime vs compile-time:** Behavior toggles (e.g. `CLAMAV_ENABLED`, `SEMANTIC_SEARCH_ENABLED`) are read at startup—you can enable or disable them without recompiling. Optional heavy dependencies (ClamAV client, AWS SDKs, semantic-search providers) are only included when the matching Cargo feature is enabled.

## Table of Contents

- [Required Configuration](#required-configuration)
- [Database Configuration](#database-configuration)
- [Storage Configuration](#storage-configuration)
- [Server Configuration](#server-configuration)
- [Media Upload Limits](#media-upload-limits)
- [Security Configuration](#security-configuration)
- [Video Transcoding](#video-transcoding)
- [Semantic Search](#semantic-search)
- [Task Queue Configuration](#task-queue-configuration)
- [Webhook Configuration](#webhook-configuration)
- [Email / Alert Notifications](#email--alert-notifications)
- [OpenTelemetry / Observability](#opentelemetry--observability)
- [Environment-Specific Settings](#environment-specific-settings)

## Required Configuration

These environment variables **must** be set for Mindia to run:

### DATABASE_URL

PostgreSQL connection string.

```env
DATABASE_URL=postgresql://user:password@host:5432/dbname?sslmode=require
```

**Format**: `postgresql://[user]:[password]@[host]:[port]/[database]?[params]`

**Examples**:
```env
# Neon (recommended)
DATABASE_URL=postgresql://user:pass@ep-xxx.neon.tech/neondb?sslmode=require

# Local PostgreSQL
DATABASE_URL=postgresql://mindia:password@localhost:5432/mindia

# AWS RDS
DATABASE_URL=postgresql://admin:pass@db.xxx.rds.amazonaws.com:5432/mindia?sslmode=require
```

**Notes**:
- Always use `?sslmode=require` for production
- For Neon, copy the connection string from the dashboard
- For pgvector support, ensure the extension is installed

### JWT_SECRET

Secret key for signing JWT tokens. **Must be at least 32 characters.**

```env
JWT_SECRET=your-random-secret-key-min-32-chars
```

**Generate**:
```bash
# Linux/macOS
openssl rand -hex 32

# Or use any random string generator (min 32 chars)
```

**Security**:
- Never commit this to version control
- Use different secrets for dev/staging/production
- Rotate periodically in production

### S3 Configuration (if using S3 storage)

When `STORAGE_BACKEND=s3` or not specified (S3 is default):

```env
S3_BUCKET=your-bucket-name
S3_REGION=us-east-1
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...
```

**Required Permissions**:
- `s3:PutObject` - Upload files
- `s3:GetObject` - Download files
- `s3:DeleteObject` - Delete files
- `s3:ListBucket` - List objects (optional)

---

## Database Configuration

### DB_MAX_CONNECTIONS

Maximum number of database connections in the pool.

```env
DB_MAX_CONNECTIONS=20
```

**Default**: `20`  
**Range**: `1-100`

**Guidelines**:
- Increase for high-traffic applications
- Consider database limits (Neon free tier: 5 connections)
- **Critical for multi-instance deployments**: Total connections across all instances must not exceed database max connections
- Formula: `DB_MAX_CONNECTIONS = Database max connections / Number of instances`

**Multi-Instance Deployment Example**:
- Database limit: 100 connections
- 5 instances → Set `DB_MAX_CONNECTIONS=20` per instance
- Total: 5 × 20 = 100 connections (at capacity)

**Warning**: If you exceed database connection limits, your application will fail to connect to the database, causing service outages.

### DB_TIMEOUT_SECONDS

Timeout for acquiring a database connection from the pool.

```env
DB_TIMEOUT_SECONDS=30
```

**Default**: `30`  
**Range**: `5-300`

**Notes**:
- Increase if you see timeout errors under load
- Lower for faster failure detection

### ANALYTICS_DB_TYPE

Database type to use for analytics and audit logs.

```env
ANALYTICS_DB_TYPE=postgres
```

**Options**:
- `postgres` - Use PostgreSQL for analytics (default)

**Default**: `postgres`

**Notes**:
- Analytics use the main PostgreSQL database by default
- Storage metrics always use PostgreSQL (they query the main application tables)
- No additional configuration needed for analytics when using PostgreSQL

### ANALYTICS_DB_URL

**Note**: This setting is no longer used. Analytics use the main `DATABASE_URL` when `ANALYTICS_DB_TYPE=postgres` (default).

---

## Storage Configuration

### STORAGE_BACKEND

Storage backend to use for media files.

```env
STORAGE_BACKEND=s3
```

**Options**:
- `s3` - AWS S3 or S3-compatible storage (default)
- `local` - Local filesystem storage
- `nfs` - Network filesystem (NFS)

**Notes**:
- S3 is recommended for production
- Local storage is good for development
- If not specified, defaults to S3

### S3 Storage Options

```env
S3_BUCKET=your-bucket-name
S3_REGION=us-east-1
AWS_REGION=us-east-1  # Alias for S3_REGION
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...
```

**S3-Compatible Providers**:
- AWS S3
- MinIO: Use endpoint override (code level)
- DigitalOcean Spaces: Set region to `nyc3`, `sfo3`, etc.
- Backblaze B2: Requires endpoint configuration

### Local Storage Options

```env
STORAGE_BACKEND=local
LOCAL_STORAGE_PATH=/var/mindia/uploads
LOCAL_STORAGE_BASE_URL=http://localhost:3000/uploads
```

**LOCAL_STORAGE_PATH**:
- Directory to store uploaded files
- Must have write permissions
- Auto-created if doesn't exist

**LOCAL_STORAGE_BASE_URL**:
- Base URL for serving files
- Used to construct public URLs

---

## Server Configuration

### PORT

Port for the HTTP server.

```env
PORT=3000
```

**Default**: `3000`

**Notes**:
- Use `8080` or `8000` if 3000 is taken
- Fly.io automatically binds to PORT environment variable
- Behind reverse proxy: any port is fine

### ENVIRONMENT

Application environment.

```env
ENVIRONMENT=production
```

**Options**:
- `development` (default)
- `staging`
- `production` (or `prod`)

**Effects**:
- `production`: More strict validations, optimized settings
- `development`: More verbose logging, relaxed CORS

### REQUEST_TIMEOUT_SECS

Global HTTP request timeout in seconds. Requests that take longer are aborted.

```env
REQUEST_TIMEOUT_SECS=60
```

**Default**: `60`. Minimum: `1`.

### Health endpoints

- **`GET /health`** – Full health check (database, storage, optional ClamAV/semantic-search, task queue). Returns 200 when critical deps are OK; use for load balancers.
- **`GET /health/deep`** – Same as `/health` plus webhooks table connectivity. Use for ops dashboards or debugging.
- **`GET /live`** – Liveness (process is up). Always 200 if the process responds.
- **`GET /ready`** – Readiness (database reachable). Use for Kubernetes or similar readiness probes.

---

## Media Upload Limits

### Images

```env
MAX_FILE_SIZE_MB=10
ALLOWED_EXTENSIONS=jpg,jpeg,png,gif,webp
ALLOWED_CONTENT_TYPES=image/jpeg,image/png,image/gif,image/webp
```

**MAX_FILE_SIZE_MB**:
- Default: `10` MB
- Range: `1-100` MB (larger may cause memory issues)

**ALLOWED_EXTENSIONS**:
- Comma-separated list (no spaces)
- Lowercase only
- Supports: jpg, jpeg, png, gif, webp

**ALLOWED_CONTENT_TYPES**:
- MIME types to accept
- Must match ALLOWED_EXTENSIONS

### Videos

```env
MAX_VIDEO_SIZE_MB=500
VIDEO_ALLOWED_EXTENSIONS=mp4,mov,avi,webm,mkv
VIDEO_ALLOWED_CONTENT_TYPES=video/mp4,video/quicktime,video/x-msvideo,video/webm,video/x-matroska
```

**MAX_VIDEO_SIZE_MB**:
- Default: `500` MB
- Range: `10-5000` MB

**Supported Formats**:
- MP4 (H.264/H.265)
- MOV (QuickTime)
- AVI
- WebM
- MKV (Matroska)

### Documents

```env
MAX_DOCUMENT_SIZE_MB=50
DOCUMENT_ALLOWED_EXTENSIONS=pdf
DOCUMENT_ALLOWED_CONTENT_TYPES=application/pdf
```

**MAX_DOCUMENT_SIZE_MB**:
- Default: `50` MB
- Currently supports PDF only

### Audio

```env
MAX_AUDIO_SIZE_MB=100
AUDIO_ALLOWED_EXTENSIONS=mp3,m4a,wav,flac,ogg
AUDIO_ALLOWED_CONTENT_TYPES=audio/mpeg,audio/mp4,audio/x-m4a,audio/wav,audio/flac,audio/ogg
```

**MAX_AUDIO_SIZE_MB**:
- Default: `100` MB

**Supported Formats**:
- MP3
- M4A (AAC)
- WAV
- FLAC
- OGG

---

## Security Configuration

### CORS_ORIGINS

Allowed CORS origins (comma-separated).

```env
CORS_ORIGINS=https://example.com,https://app.example.com
```

**Default**: `*` (allow all - not recommended for production)

**Examples**:
```env
# Development (allow all)
CORS_ORIGINS=*

# Single domain
CORS_ORIGINS=https://example.com

# Multiple domains
CORS_ORIGINS=https://example.com,https://app.example.com,https://admin.example.com

# Localhost for development
CORS_ORIGINS=http://localhost:3000,http://localhost:5173
```

**Production**:
- Never use `*` in production
- List specific domains only
- Include all subdomains you need

### REMOVE_EXIF

Remove EXIF metadata from images for privacy.

```env
REMOVE_EXIF=true
```

**Default**: `true`

**Options**:
- `true` - Strip EXIF data on upload (recommended)
- `false` - Preserve EXIF data

**Notes**:
- EXIF can contain GPS coordinates, camera info, timestamps
- Stripping improves privacy and slightly reduces file size
- Supports JPEG and PNG formats

### CLAMAV_ENABLED

Enable virus scanning with ClamAV.

```env
CLAMAV_ENABLED=false
CLAMAV_HOST=localhost
CLAMAV_PORT=3310
```

**CLAMAV_ENABLED**:
- Default: `false`
- Set to `true` to enable scanning

**Setup**:
```bash
# Install ClamAV
# macOS
brew install clamav
brew services start clamav

# Linux
sudo apt install clamav clamav-daemon
sudo systemctl start clamav-daemon
```

**Notes**:
- Adds ~100-500ms latency per upload
- Fail-open vs fail-closed controlled by `CLAMAV_FAIL_CLOSED`
- Recommended for public-facing applications
- See [ClamAV](clamav.md) for full documentation

### CONTENT_MODERATION_ENABLED

Enable content moderation for uploaded media using AWS Rekognition.

```env
CONTENT_MODERATION_ENABLED=false
```

**Default**: `false`

**Options**:
- `true` - Queue content moderation tasks for uploaded images and videos
- `false` - Skip content moderation

**Requirements**:
- AWS credentials configured (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
- `aws_rekognition_moderation` plugin configured
- For video moderation: S3 storage backend required

**Notes**:
- When enabled, moderation tasks are queued automatically on upload
- Failed moderation triggers webhooks for `file_processing_failed` events
- Adds minimal latency as processing is done asynchronously
- See [Plugins](plugins.md) for configuration details

### AUTO_STORE_ENABLED

Default behavior for files uploaded with `?store=auto`.

```env
AUTO_STORE_ENABLED=true
```

**Default**: `true`

**Options**:
- `true` - Files with `?store=auto` are stored permanently
- `false` - Files with `?store=auto` are deleted after 24 hours

**Usage**:
- Clients can override with `?store=1` (permanent) or `?store=0` (24h)

---

## Video Transcoding

### FFMPEG_PATH

Path to FFmpeg executable.

```env
FFMPEG_PATH=/usr/bin/ffmpeg
```

**Default**: `ffmpeg` (searches PATH)

**Notes**:
- FFmpeg 4.0+ required
- Hardware acceleration auto-detected (NVENC, QSV, VideoToolbox)

### MAX_CONCURRENT_TRANSCODES

Maximum concurrent video transcoding jobs.

```env
MAX_CONCURRENT_TRANSCODES=2
```

**Default**: `2`  
**Range**: `1-10`

**Guidelines**:
- 1 transcode ≈ 1-2 CPU cores
- Set to number of CPU cores / 2
- Higher = faster processing but more resource usage

### HLS_SEGMENT_DURATION

Duration of each HLS segment in seconds.

```env
HLS_SEGMENT_DURATION=6
```

**Default**: `6` seconds  
**Range**: `2-10`

**Notes**:
- Shorter = better seeking, more files, more overhead
- Longer = fewer files, slower seeking
- 6 seconds is optimal for most use cases

### HLS_VARIANTS

Quality variants to generate for HLS streaming.

```env
HLS_VARIANTS=360p,480p,720p,1080p
```

**Default**: `360p,480p,720p,1080p`

**Available**:
- `360p` - 640x360, 800 Kbps
- `480p` - 854x480, 1400 Kbps
- `720p` - 1280x720, 2800 Kbps
- `1080p` - 1920x1080, 5000 Kbps

**Notes**:
- Only variants ≤ source resolution are generated
- Fewer variants = faster transcoding
- More variants = better adaptive streaming

---

## Semantic Search

### SEMANTIC_SEARCH_ENABLED

Enable semantic search using **Anthropic/Claude** (cloud, paid API). Requires `ANTHROPIC_API_KEY`.

```env
SEMANTIC_SEARCH_ENABLED=true
ANTHROPIC_API_KEY=your-api-key
ANTHROPIC_VISION_MODEL=claude-sonnet-4-20250514
ANTHROPIC_EMBEDDING_MODEL=embed-v3
```
- **ANTHROPIC_EMBEDDING_MODEL**: default `embed-v3` (modern). Use `embed-1` if your account only has the older model.

See [Semantic Search](semantic-search.md) for details.

---

## Task Queue Configuration

### TASK_QUEUE_MAX_WORKERS

Maximum concurrent background workers.

```env
TASK_QUEUE_MAX_WORKERS=4
```

**Default**: `4`  
**Range**: `1-20`

**Notes**:
- Workers process embedding generation and video transcoding
- Higher = more parallelism but more resource usage

### TASK_QUEUE_POLL_INTERVAL_MS

How often to poll for new tasks (milliseconds).

```env
TASK_QUEUE_POLL_INTERVAL_MS=1000
```

**Default**: `1000` (1 second)  
**Range**: `100-10000`

### Rate Limits

```env
TASK_QUEUE_VIDEO_RATE_LIMIT=2
TASK_QUEUE_EMBEDDING_RATE_LIMIT=5
```

**TASK_QUEUE_VIDEO_RATE_LIMIT**:
- Max concurrent video transcoding tasks
- Default: `2`

**TASK_QUEUE_EMBEDDING_RATE_LIMIT**:
- Max concurrent embedding generation tasks
- Default: `5`

---

## Webhook Configuration

### Webhook Retry Settings

```env
WEBHOOK_TIMEOUT_SECONDS=30
WEBHOOK_MAX_RETRIES=72
WEBHOOK_RETRY_POLL_INTERVAL_SECONDS=300
WEBHOOK_RETRY_BATCH_SIZE=50
WEBHOOK_MAX_CONCURRENT_RETRIES=10
WEBHOOK_MAX_CONCURRENT_DELIVERIES=50
```

**WEBHOOK_TIMEOUT_SECONDS**:
- HTTP timeout for webhook requests
- Default: `30` seconds

**WEBHOOK_MAX_RETRIES**:
- Maximum retry attempts
- Default: `72` (~72 hours with exponential backoff)

**WEBHOOK_RETRY_POLL_INTERVAL_SECONDS**:
- How often to check for failed webhooks to retry
- Default: `300` (5 minutes)

**WEBHOOK_RETRY_BATCH_SIZE**:
- Number of failed webhooks to retry per batch
- Default: `50`

**WEBHOOK_MAX_CONCURRENT_RETRIES**:
- Maximum concurrent retry attempts
- Default: `10`

**WEBHOOK_MAX_CONCURRENT_DELIVERIES**:
- Maximum concurrent webhook deliveries per instance
- Default: `50`
- **Important**: Limits the number of simultaneous webhook HTTP requests to prevent resource exhaustion
- Increase for high-traffic scenarios, but monitor memory and network connection usage

---

## Email / Alert Notifications

Usage alerts (storage, API requests approaching or exceeding limits) can be sent by email to organization owners and admins. Email is **disabled by default**; set `EMAIL_ALERTS_ENABLED=true` and configure SMTP to enable.

### EMAIL_ALERTS_ENABLED

Enable or disable email notifications for usage alerts.

```env
EMAIL_ALERTS_ENABLED=false
```

- Default: `false`
- When `true`, requires `SMTP_HOST` and `SMTP_FROM`

### SMTP Configuration

```env
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USER=alerts@example.com
SMTP_PASSWORD=secret
SMTP_FROM=Mindia Alerts <alerts@example.com>
SMTP_TLS=true
```

**SMTP_HOST**: SMTP relay hostname (required when email alerts enabled).

**SMTP_PORT**: SMTP port. Default: `587` (STARTTLS). Use `465` for implicit TLS with `relay()`-style configs.

**SMTP_USER** / **SMTP_PASSWORD**: Optional; set when the relay requires authentication.

**SMTP_FROM**: Sender address used for alert emails (required when email alerts enabled). Use `"Name <email@example.com>"` format.

**SMTP_TLS**: Use TLS for SMTP. Default: `true`. When `true`, uses STARTTLS (port 587 typical).

### FRONTEND_URL

Optional base URL of the dashboard. When set, alert emails include a "View usage" link.

```env
FRONTEND_URL=https://app.example.com
```

### Security

- Do not log or expose `SMTP_PASSWORD` or other SMTP secrets.
- Prefer environment variables over config files for credentials.

---

## OpenTelemetry / Observability

### OTEL_ENABLED

Enable OpenTelemetry tracing and metrics.

```env
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=mindia
OTEL_SERVICE_VERSION=0.1.0
OTEL_EXPORTER_OTLP_PROTOCOL=grpc
OTEL_SAMPLER=always_on
OTEL_SAMPLE_RATIO=1.0
OTEL_METRICS_INTERVAL_SECS=30
OTEL_SERVICE_INSTANCE_ID=optional-instance-id
```

**OTEL_ENABLED**:
- Default: `true`
- Set to `false` to disable telemetry

**OTEL_EXPORTER_OTLP_ENDPOINT**:
- OTLP collector endpoint
- Default: `http://localhost:4317`
- Examples:
  - Jaeger: `http://localhost:4317`
  - Grafana Cloud: `https://otlp-gateway-prod.grafana.net/otlp`
  - Honeycomb: `https://api.honeycomb.io:443`

**OTEL_EXPORTER_OTLP_PROTOCOL**:
- `grpc` (default) or `http`
- Use `http` for HTTP/protobuf protocol, `grpc` for gRPC protocol

**OTEL_SAMPLER**:
- Trace sampling strategy
- Options: `always_on` (default), `always_off`, `trace_id_ratio`
- `always_on`: Sample all traces (useful for development)
- `always_off`: Don't sample any traces (useful for testing)
- `trace_id_ratio`: Sample based on trace ID ratio (requires `OTEL_SAMPLE_RATIO`)

**OTEL_SAMPLE_RATIO**:
- Sample ratio for `trace_id_ratio` sampler
- Default: `1.0` (100% sampling)
- Range: `0.0` to `1.0`
- Example: `0.1` = 10% of traces sampled
- Only used when `OTEL_SAMPLER=trace_id_ratio`

**OTEL_METRICS_INTERVAL_SECS**:
- Interval in seconds for metrics export
- Default: `30`
- Lower values = more frequent exports (higher overhead)
- Higher values = less frequent exports (lower overhead)

**OTEL_SERVICE_INSTANCE_ID**:
- Optional service instance identifier
- Default: Auto-generated UUID
- Useful for multi-instance deployments to distinguish instances
- If not set, a UUID is automatically generated

**Collected Data**:
- HTTP request traces with semantic conventions (`http.method`, `http.route`, `http.status_code`)
- Database query spans with semantic conventions (`db.system`, `db.name`, `db.operation`)
- S3 operation spans with AWS semantic conventions (`aws.service.name`, `aws.s3.bucket`, `aws.s3.operation`)
- FFmpeg processing spans with process semantic conventions (`process.executable.name`, `process.command`)
- Business metrics (uploads, transformations, streams)
- Resource attributes: `service.name`, `service.version`, `deployment.environment`, `host.name`, `service.instance.id`

**Semantic Conventions Used**:
- HTTP: `http.method`, `http.route`, `http.status_code`, `http.target`, `http.scheme`
- Database: `db.system`, `db.name`, `db.operation`, `db.sql.table`
- AWS S3: `aws.service.name`, `aws.s3.bucket`, `aws.s3.key`, `aws.s3.operation`
- Process: `process.command`, `process.executable.name`, `process.executable.path`
- Service: `service.name`, `service.version`, `deployment.environment`

---

## Environment-Specific Settings

### Development

```env
ENVIRONMENT=development
RUST_LOG=mindia=debug,tower_http=debug
CORS_ORIGINS=http://localhost:3000,http://localhost:5173
OTEL_ENABLED=false
```

### Staging

```env
ENVIRONMENT=staging
RUST_LOG=mindia=info,tower_http=info
CORS_ORIGINS=https://staging.example.com
OTEL_ENABLED=true
CLAMAV_ENABLED=true
```

### Production

```env
ENVIRONMENT=production
RUST_LOG=mindia=info,tower_http=warn
CORS_ORIGINS=https://example.com,https://app.example.com
OTEL_ENABLED=true
CLAMAV_ENABLED=true
REMOVE_EXIF=true
AUTO_STORE_ENABLED=false
```

---

## Example Configurations

### Minimal (Development)

```env
DATABASE_URL=postgresql://localhost/mindia
JWT_SECRET=dev-secret-key-min-32-characters-long
STORAGE_BACKEND=local
LOCAL_STORAGE_PATH=./uploads
LOCAL_STORAGE_BASE_URL=http://localhost:3000/uploads
PORT=3000
```

### Standard (Production)

```env
# Database
DATABASE_URL=postgresql://user:pass@host.neon.tech/db?sslmode=require
DB_MAX_CONNECTIONS=20

# S3 Storage
S3_BUCKET=myapp-media
S3_REGION=us-east-1
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...

# Authentication
JWT_SECRET=<generate-with-openssl-rand-hex-32>

# Server
PORT=3000
ENVIRONMENT=production

# Security
CORS_ORIGINS=https://myapp.com,https://api.myapp.com
REMOVE_EXIF=true
CLAMAV_ENABLED=true

# Media Limits
MAX_FILE_SIZE_MB=10
MAX_VIDEO_SIZE_MB=500

# Observability
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=https://otel.myapp.com
RUST_LOG=mindia=info
```

### Full-Featured

All features enabled:

```env
# Database
DATABASE_URL=postgresql://user:pass@host.neon.tech/db?sslmode=require
DB_MAX_CONNECTIONS=20
DB_TIMEOUT_SECONDS=30

# S3 Storage
S3_BUCKET=myapp-media
S3_REGION=us-east-1
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...

# Authentication
JWT_SECRET=<secure-random-secret>

# Server
PORT=3000
ENVIRONMENT=production

# Security
CORS_ORIGINS=https://myapp.com
REMOVE_EXIF=true
CLAMAV_ENABLED=true
CLAMAV_HOST=localhost
CLAMAV_PORT=3310

# Media Limits
MAX_FILE_SIZE_MB=10
MAX_VIDEO_SIZE_MB=500
MAX_DOCUMENT_SIZE_MB=50
MAX_AUDIO_SIZE_MB=100

# Video Transcoding
FFMPEG_PATH=/usr/bin/ffmpeg
MAX_CONCURRENT_TRANSCODES=2
HLS_SEGMENT_DURATION=6
HLS_VARIANTS=360p,480p,720p,1080p

# Semantic Search
SEMANTIC_SEARCH_ENABLED=true
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_EMBEDDING_MODEL=nomic-embed-text
OLLAMA_VISION_MODEL=llama3.2-vision:11b

# Task Queue
TASK_QUEUE_MAX_WORKERS=4
TASK_QUEUE_VIDEO_RATE_LIMIT=2
TASK_QUEUE_EMBEDDING_RATE_LIMIT=5

# Webhooks
WEBHOOK_TIMEOUT_SECONDS=30
WEBHOOK_MAX_RETRIES=72

# Observability
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
OTEL_SERVICE_NAME=mindia
RUST_LOG=mindia=info,tower_http=info
```

---

## Next Steps

- [Quick Start](quick-start.md) - Get started quickly
- [Installation](installation.md) - Detailed setup guide
- [Best Practices](best-practices.md) - Production recommendations

