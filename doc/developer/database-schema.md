# Database Schema

Complete PostgreSQL database schema for Mindia.

## Overview

Mindia uses PostgreSQL with the pgvector extension for vector similarity search.

## Core Tables

### tenants

Organizations using Mindia.

```sql
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    s3_bucket VARCHAR(255) NOT NULL UNIQUE,
    s3_region VARCHAR(50) NOT NULL,
    status tenant_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TYPE tenant_status AS ENUM ('active', 'suspended', 'deleted');
```

### users

User accounts.

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    email VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role user_role NOT NULL DEFAULT 'member',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

CREATE TYPE user_role AS ENUM ('admin', 'member', 'viewer');
CREATE INDEX idx_users_tenant ON users(tenant_id);
CREATE INDEX idx_users_email ON users(email);
```

## Media Tables

### images

```sql
CREATE TABLE images (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    filename VARCHAR(255) NOT NULL,
    original_filename VARCHAR(255) NOT NULL,
    s3_key VARCHAR(512) NOT NULL,
    s3_url TEXT NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    file_size BIGINT NOT NULL,
    width INTEGER,
    height INTEGER,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_images_tenant ON images(tenant_id, uploaded_at DESC);
```

### videos

```sql
CREATE TABLE videos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    filename VARCHAR(255) NOT NULL,
    s3_key VARCHAR(512) NOT NULL,
    processing_status processing_status NOT NULL DEFAULT 'pending',
    hls_master_key VARCHAR(512),
    duration FLOAT,
    width INTEGER,
    height INTEGER,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TYPE processing_status AS ENUM ('pending', 'processing', 'completed', 'failed');
```

### audios

```sql
CREATE TABLE audios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    filename VARCHAR(255) NOT NULL,
    s3_key VARCHAR(512) NOT NULL,
    duration FLOAT,
    bitrate INTEGER,
    sample_rate INTEGER,
    channels INTEGER,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### documents

```sql
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    filename VARCHAR(255) NOT NULL,
    s3_key VARCHAR(512) NOT NULL,
    file_size BIGINT NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Search & Analytics

### embeddings

Vector embeddings for semantic search.

```sql
CREATE TABLE embeddings (
    id SERIAL PRIMARY KEY,
    entity_id UUID NOT NULL,
    entity_type entity_type NOT NULL,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    description TEXT NOT NULL,
    embedding vector(768) NOT NULL,
    model_name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(entity_id, entity_type)
);

CREATE TYPE entity_type AS ENUM ('image', 'video', 'audio', 'document');
CREATE INDEX idx_embeddings_vector ON embeddings USING ivfflat (embedding vector_cosine_ops);
```

### request_logs

HTTP request analytics.

```sql
CREATE TABLE request_logs (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID REFERENCES tenants(id),
    method VARCHAR(10) NOT NULL,
    path VARCHAR(512) NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER NOT NULL,
    bytes_sent BIGINT,
    bytes_received BIGINT,
    ip_address VARCHAR(45),
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_request_logs_tenant ON request_logs(tenant_id, created_at DESC);
```

## Webhooks

### webhooks

```sql
CREATE TABLE webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    url VARCHAR(512) NOT NULL,
    events TEXT[] NOT NULL,
    signing_secret VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### webhook_events

```sql
CREATE TABLE webhook_events (
    id BIGSERIAL PRIMARY KEY,
    webhook_id UUID NOT NULL REFERENCES webhooks(id),
    event_type VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL,
    delivery_status delivery_status NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TYPE delivery_status AS ENUM ('pending', 'delivered', 'failed');
```

## Background Jobs

### tasks

```sql
CREATE TABLE tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id),
    task_type task_type NOT NULL,
    payload JSONB NOT NULL,
    status task_status NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    scheduled_for TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT
);

CREATE TYPE task_type AS ENUM ('video_transcode', 'generate_embedding');
CREATE TYPE task_status AS ENUM ('pending', 'running', 'completed', 'failed');
```

## Indexes

Key indexes for performance:

```sql
-- Tenant-scoped queries
CREATE INDEX idx_images_tenant_created ON images(tenant_id, created_at DESC);
CREATE INDEX idx_videos_tenant_created ON videos(tenant_id, created_at DESC);

-- Search
CREATE INDEX idx_embeddings_tenant ON embeddings(tenant_id);
CREATE INDEX idx_embeddings_entity ON embeddings(entity_id, entity_type);

-- Analytics
CREATE INDEX idx_request_logs_path ON request_logs(path, created_at DESC);
CREATE INDEX idx_request_logs_status ON request_logs(status_code, created_at DESC);

-- Tasks
CREATE INDEX idx_tasks_status ON tasks(status, scheduled_for);
```

## Next Steps

- [Architecture](architecture.md) - System design
- [Development Setup](development-setup.md) - Get started
- [Migrations](migrations.md) - Managing schema changes

