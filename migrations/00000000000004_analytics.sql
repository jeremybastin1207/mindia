-- Analytics: request logs and storage metrics

CREATE TABLE IF NOT EXISTS request_logs (
    id BIGSERIAL PRIMARY KEY,
    request_id UUID NOT NULL DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    method VARCHAR(10) NOT NULL,
    path TEXT NOT NULL,
    normalized_path TEXT NOT NULL,
    query_string TEXT,
    status_code INTEGER NOT NULL,
    request_size_bytes BIGINT NOT NULL DEFAULT 0,
    response_size_bytes BIGINT NOT NULL DEFAULT 0,
    duration_ms BIGINT NOT NULL,
    user_agent TEXT,
    ip_address INET,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_request_logs_tenant_id ON request_logs(tenant_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_tenant_created ON request_logs(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_request_logs_path ON request_logs(normalized_path);
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_request_logs_status_code ON request_logs(status_code);
CREATE INDEX IF NOT EXISTS idx_request_logs_method ON request_logs(method);
CREATE INDEX IF NOT EXISTS idx_request_logs_path_created ON request_logs(normalized_path, created_at DESC);

CREATE TABLE IF NOT EXISTS storage_metrics (
    id SERIAL PRIMARY KEY,
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    total_files BIGINT NOT NULL,
    total_storage_bytes BIGINT NOT NULL,
    image_count BIGINT NOT NULL DEFAULT 0,
    image_bytes BIGINT NOT NULL DEFAULT 0,
    video_count BIGINT NOT NULL DEFAULT 0,
    video_bytes BIGINT NOT NULL DEFAULT 0,
    audio_count BIGINT NOT NULL DEFAULT 0,
    audio_bytes BIGINT NOT NULL DEFAULT 0,
    document_count BIGINT NOT NULL DEFAULT 0,
    document_bytes BIGINT NOT NULL DEFAULT 0,
    by_content_type JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_storage_metrics_tenant_id ON storage_metrics(tenant_id);
CREATE INDEX IF NOT EXISTS idx_storage_metrics_tenant_created ON storage_metrics(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_storage_metrics_created_at ON storage_metrics(created_at DESC);
