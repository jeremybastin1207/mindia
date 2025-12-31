# Deployment

Guide to deploying Mindia in production.

## Deployment Options

### 1. Docker (Recommended)

**Dockerfile**:
```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    ffmpeg \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mindia /usr/local/bin/
CMD ["mindia"]
```

**docker-compose.yml**:
```yaml
version: '3.8'

services:
  mindia:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgresql://user:pass@db/mindia
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - S3_REGION=us-east-1
      - RUST_LOG=info
    depends_on:
      - db

  db:
    image: pgvector/pgvector:pg15
    environment:
      - POSTGRES_DB=mindia
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

### 2. Kubernetes

**deployment.yaml**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mindia
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mindia
  template:
    metadata:
      labels:
        app: mindia
    spec:
      containers:
      - name: mindia
        image: your-registry/mindia:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: mindia-secrets
              key: database-url
        - name: AWS_ACCESS_KEY_ID
          valueFrom:
            secretKeyRef:
              name: aws-credentials
              key: access-key-id
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
```

### 3. Cloud Platforms

#### AWS ECS

1. Build Docker image
2. Push to ECR
3. Create task definition
4. Create service
5. Configure ALB

#### Render

```yaml
# render.yaml
services:
  - type: web
    name: mindia
    env: docker
    dockerfilePath: ./Dockerfile
    envVars:
      - key: DATABASE_URL
        sync: false
      - key: AWS_ACCESS_KEY_ID
        sync: false
```

## Database Setup

### Neon (Recommended)

1. Create database at [neon.tech](https://neon.tech)
2. Enable pgvector extension
3. Copy connection string to `DATABASE_URL`

### Self-Hosted PostgreSQL

```bash
# Install PostgreSQL + pgvector
docker run -d \
  --name postgres \
  -e POSTGRES_DB=mindia \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  pgvector/pgvector:pg15

# Enable extension
psql -d mindia -c "CREATE EXTENSION vector;"
```

## Storage Setup

### AWS S3

```bash
# Create bucket
aws s3 mb s3://your-mindia-bucket --region us-east-1

# Enable versioning (optional)
aws s3api put-bucket-versioning \
  --bucket your-mindia-bucket \
  --versioning-configuration Status=Enabled

# Set CORS
aws s3api put-bucket-cors \
  --bucket your-mindia-bucket \
  --cors-configuration file://cors.json
```

**cors.json**:
```json
{
  "CORSRules": [
    {
      "AllowedOrigins": ["https://your-frontend.com"],
      "AllowedMethods": ["GET", "HEAD"],
      "AllowedHeaders": ["*"],
      "MaxAgeSeconds": 3600
    }
  ]
}
```

## CDN Setup

### CloudFront

1. Create distribution
2. Set origin to Mindia API
3. Configure cache behaviors:
   - `/api/images/*/w_*`: Cache everything, 1 year TTL
   - `/api/videos/*/stream/*`: Cache everything, 1 day TTL
   - `/api/*`: No caching

### Cloudflare

1. Add domain
2. Create Page Rules:
   - `api.example.com/api/images/*/w_*`: Cache Everything
   - `api.example.com/api/videos/*/stream/*`: Cache Everything

## Environment Variables

**Production .env**:
```env
# Environment
ENVIRONMENT=production
PORT=3000

# Database
DATABASE_URL=postgresql://user:pass@host/mindia
DATABASE_MAX_CONNECTIONS=20

# Storage (S3)
AWS_ACCESS_KEY_ID=your-aws-access-key-id
AWS_SECRET_ACCESS_KEY=your-aws-secret-access-key
S3_REGION=us-east-1

# JWT
JWT_SECRET=your-secure-random-secret-min-32-chars

# CORS
CORS_ORIGINS=https://your-frontend.com

# Features
SEMANTIC_SEARCH_ENABLED=true
CLAMAV_ENABLED=true
REMOVE_EXIF=true

# Semantic Search (Anthropic)
ANTHROPIC_API_KEY=
ANTHROPIC_VISION_MODEL=claude-sonnet-4-20250514
ANTHROPIC_EMBEDDING_MODEL=embed-v3

# Logging
RUST_LOG=mindia=info,tower_http=debug

# Telemetry (optional)
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=https://api.honeycomb.io
```

## Monitoring

### OpenTelemetry

```env
# Enable telemetry
OTEL_ENABLED=true
OTEL_SERVICE_NAME=mindia
OTEL_EXPORTER_OTLP_ENDPOINT=https://api.honeycomb.io

# Or Jaeger
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
```

### Health Checks

```bash
# Kubernetes liveness/readiness
livenessProbe:
  httpGet:
    path: /health
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 30
```

## SSL/TLS

### Let's Encrypt

```bash
# Using Caddy (automatic HTTPS)
caddy reverse-proxy --from api.example.com --to localhost:3000
```

### nginx

```nginx
server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Scaling

### Database Connection Pool Sizing

When running multiple instances of Mindia (horizontal scaling), it's critical to configure database connection pools correctly to avoid connection exhaustion.

**Key Principle**: 
- Total connections across all instances must not exceed your database's maximum connection limit
- Formula: `DB_MAX_CONNECTIONS = Database max connections / Number of instances`

**Examples**:

1. **Small Deployment** (Neon free tier: 5 connections max)
   - 2 instances → `DB_MAX_CONNECTIONS=2` per instance
   - Total: 2 × 2 = 4 connections (safe margin)

2. **Medium Deployment** (RDS db.t3.medium: ~40 connections max)
   - 3 instances → `DB_MAX_CONNECTIONS=10` per instance
   - Total: 3 × 10 = 30 connections (within limit)

3. **Large Deployment** (RDS db.r6g.2xlarge: ~200 connections max)
   - 5 instances → `DB_MAX_CONNECTIONS=35` per instance
   - Total: 5 × 35 = 175 connections (within limit)

**Configuration**:
```env
# Set per instance
DB_MAX_CONNECTIONS=20
```

**Monitoring**:
- Monitor database connection count in your database metrics
- Set alerts if connection usage exceeds 80% of database limit
- Use connection pooling tools (e.g., PgBouncer, RDS Proxy) for better efficiency

**Using RDS Proxy** (AWS):
When using AWS RDS Proxy, you can set higher `DB_MAX_CONNECTIONS` values since the proxy manages actual database connections efficiently.

### Load Balancer

```yaml
# AWS ALB or similar
# Route requests to multiple Mindia instances
```

### Auto-Scaling

```yaml
# Kubernetes HPA
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mindia-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: mindia
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

## Backup Strategy

1. **Database**: Neon auto-backups or `pg_dump` cron
2. **S3**: Versioning + cross-region replication
3. **Config**: Version control (Git)

## Security Checklist

- [ ] HTTPS everywhere
- [ ] Strong JWT_SECRET (32+ random chars)
- [ ] CORS set to specific origins
- [ ] Database not publicly accessible
- [ ] S3 bucket not public
- [ ] Secrets in environment variables (not code)
- [ ] Regular security updates
- [ ] Rate limiting configured
- [ ] ClamAV enabled (if possible)

## Next Steps

- [Monitoring](monitoring.md) - Set up observability
- [Performance](performance.md) - Optimization tips
- [Troubleshooting](troubleshooting.md) - Common issues

