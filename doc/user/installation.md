# Installation Guide

This guide provides detailed installation instructions for Mindia across different platforms and deployment scenarios.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Local Development Setup](#local-development-setup)
- [Docker Setup](#docker-setup)
- [Production Deployment](#production-deployment)
- [Platform-Specific Instructions](#platform-specific-instructions)

## Prerequisites

### Required

1. **PostgreSQL 13+** with pgvector extension
   - Recommended: [Neon](https://neon.tech) (serverless PostgreSQL)
   - Alternative: Self-hosted PostgreSQL, Amazon RDS, Supabase

2. **S3-Compatible Storage**
   - AWS S3 (recommended)
   - MinIO (self-hosted)
   - DigitalOcean Spaces
   - Backblaze B2

3. **System Requirements**
   - CPU: 2+ cores (4+ recommended for video transcoding)
   - RAM: 2GB minimum (4GB+ recommended)
   - Disk: 10GB+ for temporary video processing files
   - Network: Good bandwidth for S3 transfers

### Optional

4. **FFmpeg** (for video transcoding)
   - Version 4.0 or higher
   - Hardware acceleration support (NVENC, QSV, VideoToolbox) recommended

5. **Anthropic API key** (for semantic search)
   - Get from https://console.anthropic.com

6. **ClamAV** (for virus scanning)
   - For enhanced security on file uploads

## Local Development Setup

### Step 1: Install Rust

#### macOS

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart your terminal, then verify
rustc --version
cargo --version
```

#### Linux (Ubuntu/Debian)

```bash
# Install build essentials
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH
source $HOME/.cargo/env

# Verify
rustc --version
cargo --version
```

#### Windows

1. Download [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
2. Install "Desktop development with C++"
3. Install Rust from https://rustup.rs (run the installer)
4. Restart your terminal
5. Verify: `rustc --version`

### Step 2: Clone Repository

```bash
git clone <your-repository-url>
cd mindia
```

### Step 3: Set Up PostgreSQL Database

#### Option A: Neon (Recommended)

1. Sign up at https://console.neon.tech
2. Create a new project
3. Note the connection string:
   ```
   postgresql://user:password@ep-xxx.neon.tech/neondb?sslmode=require
   ```

#### Option B: Local PostgreSQL

```bash
# Install PostgreSQL
# macOS
brew install postgresql@15
brew services start postgresql@15

# Ubuntu/Debian
sudo apt install postgresql-15 postgresql-contrib-15
sudo systemctl start postgresql

# Create database and user
createdb mindia
createuser -P mindia_user

# Grant permissions
psql -d mindia -c "GRANT ALL PRIVILEGES ON DATABASE mindia TO mindia_user;"
```

Install pgvector extension:

```bash
# Connect to database
psql -d mindia

# Install extension
CREATE EXTENSION IF NOT EXISTS vector;

# Verify
\dx
```

### Step 4: Set Up S3 Storage

#### Option A: AWS S3

1. **Create S3 Bucket**:
   - Go to AWS Console → S3
   - Click "Create bucket"
   - Choose a unique name
   - Select region (e.g., us-east-1)
   - Keep default settings
   - Click "Create bucket"

2. **Create IAM User**:
   - Go to IAM → Users → Add user
   - Name: `mindia-api`
   - Access type: Programmatic access
   - Attach policy (inline):

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:DeleteObject",
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::your-bucket-name/*",
        "arn:aws:s3:::your-bucket-name"
      ]
    }
  ]
}
```

3. **Save Credentials**:
   - Access Key ID
   - Secret Access Key

#### Option B: MinIO (Self-Hosted)

```bash
# Run MinIO with Docker
docker run -d \
  -p 9000:9000 \
  -p 9001:9001 \
  --name minio \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  -v ~/minio/data:/data \
  minio/minio server /data --console-address ":9001"

# Access console at http://localhost:9001
# Create bucket named "mindia"
# Create access key under Access Keys
```

### Step 5: Configure Environment

```bash
# Copy example environment file
cp .env.example .env

# Edit with your credentials
nano .env
```

**Required Configuration**:

```env
# Database
DATABASE_URL=postgresql://user:password@host/dbname?sslmode=require

# S3 Storage
S3_BUCKET=your-bucket-name
S3_REGION=us-east-1
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...

# Authentication
JWT_SECRET=generate-with-openssl-rand-hex-32

# Server
PORT=3000
```

Generate JWT secret:
```bash
openssl rand -hex 32
```

### Step 6: Build and Run

```bash
# Install dependencies and build
cargo build --release

# Run the application
./target/release/mindia

# Or run in development mode (faster compilation, slower runtime)
cargo run
```

The server will start on `http://localhost:3000`.

### Step 7: Verify Installation

```bash
# Health check
curl http://localhost:3000/health

# Should return:
# {"status":"healthy","database":"connected"}
```

### Step 8: Verify API Access

Set your master API key and test an endpoint:

```bash
# Use the same value as MASTER_API_KEY in your .env
export MASTER_KEY="your-master-api-key-at-least-32-characters"

# List images (should return [] when empty)
curl -s -H "Authorization: Bearer $MASTER_KEY" http://localhost:3000/api/v0/images
```

## Docker Setup

### Basic Docker

```bash
# Build image
docker build -t mindia .

# Run container
docker run -d \
  --name mindia \
  -p 3000:3000 \
  --env-file .env \
  mindia
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  mindia:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - S3_BUCKET=${S3_BUCKET}
      - S3_REGION=${S3_REGION}
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - JWT_SECRET=${JWT_SECRET}
    env_file:
      - .env
    depends_on:
      - postgres
    restart: unless-stopped

  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: mindia
      POSTGRES_USER: mindia
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

volumes:
  postgres_data:
```

Run:

```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f mindia

# Stop services
docker-compose down
```

## Production Deployment

### Fly.io

See [Quick Start Guide](quick-start.md#option-1-deploy-to-flyio-recommended) for Fly.io deployment.

### AWS EC2

1. **Launch EC2 Instance**:
   - AMI: Ubuntu 22.04 LTS
   - Instance type: t3.medium (minimum)
   - Security group: Allow ports 80, 443, 22

2. **Install Dependencies**:

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install system dependencies
sudo apt install -y build-essential pkg-config libssl-dev
```

3. **Deploy Application**:

```bash
# Clone and build
git clone <repo>
cd mindia
cargo build --release

# Create systemd service
sudo nano /etc/systemd/system/mindia.service
```

```ini
[Unit]
Description=Mindia Media API
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/mindia
EnvironmentFile=/home/ubuntu/mindia/.env
ExecStart=/home/ubuntu/mindia/target/release/mindia
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
# Start service
sudo systemctl daemon-reload
sudo systemctl enable mindia
sudo systemctl start mindia

# Check status
sudo systemctl status mindia
```

4. **Set Up Nginx**:

```bash
sudo apt install -y nginx

sudo nano /etc/nginx/sites-available/mindia
```

```nginx
server {
    listen 80;
    server_name api.yourdomain.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

```bash
sudo ln -s /etc/nginx/sites-available/mindia /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

5. **SSL with Let's Encrypt**:

```bash
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d api.yourdomain.com
```

### DigitalOcean App Platform

1. **Create App**:
   - Go to DigitalOcean → Apps → Create App
   - Connect your GitHub repo
   - Select branch

2. **Configure Build**:
   - Buildpack: Docker
   - Dockerfile path: `./Dockerfile`

3. **Set Environment Variables**:
   - Add all required variables from `.env`

4. **Deploy**:
   - Click "Create Resources"

### Kubernetes

Example `deployment.yaml`:

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
        - name: S3_BUCKET
          valueFrom:
            secretKeyRef:
              name: mindia-secrets
              key: s3-bucket
        # Add other env vars
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
---
apiVersion: v1
kind: Service
metadata:
  name: mindia-service
spec:
  selector:
    app: mindia
  ports:
  - port: 80
    targetPort: 3000
  type: LoadBalancer
```

## Platform-Specific Instructions

### macOS

**Install FFmpeg** (for video support):
```bash
brew install ffmpeg
```

**Semantic search** uses Anthropic (cloud). Set `ANTHROPIC_API_KEY` when enabling; see [Semantic Search](semantic-search.md).

### Linux (Ubuntu/Debian)

**Install FFmpeg**:
```bash
sudo apt install -y ffmpeg
```

**Semantic search** uses Anthropic (cloud). Set `ANTHROPIC_API_KEY` when enabling; see [Semantic Search](semantic-search.md).

### Windows

**Install FFmpeg**:
1. Download from https://www.gyan.dev/ffmpeg/builds/
2. Extract to `C:\ffmpeg`
3. Add `C:\ffmpeg\bin` to PATH

**Semantic search** uses Anthropic (cloud). Set `ANTHROPIC_API_KEY` when enabling; see [Semantic Search](semantic-search.md).

## Post-Installation

### Optional Features

#### Enable Semantic Search

```env
SEMANTIC_SEARCH_ENABLED=true
ANTHROPIC_API_KEY=your-api-key
```

See [Semantic Search](semantic-search.md) for details.

#### Enable Virus Scanning

```bash
# Install ClamAV
# macOS
brew install clamav
brew services start clamav

# Linux
sudo apt install clamav clamav-daemon
sudo systemctl start clamav-daemon
```

```env
CLAMAV_ENABLED=true
CLAMAV_HOST=localhost
CLAMAV_PORT=3310
```

#### Enable OpenTelemetry

```env
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=mindia
```

### Verification

Run these tests to verify everything works:

```bash
# Health check
curl http://localhost:3000/health

# Upload test image (use MASTER_API_KEY from .env)
curl -X POST http://localhost:3000/api/v0/images \
  -H "Authorization: Bearer $MASTER_API_KEY" \
  -F "file=@test.jpg"
```

## Troubleshooting

See the [Quick Start Guide](quick-start.md#common-issues) for common issues and solutions.

## Next Steps

- [Configuration](configuration.md) - Complete environment variable reference
- [Quick Start](quick-start.md) - Quick deployment guide
- [Best Practices](best-practices.md) - Production deployment tips

