-- Presigned and chunked uploads

DO $$ BEGIN
    CREATE TYPE upload_session_status AS ENUM ('pending', 'uploading', 'completed', 'failed');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS presigned_upload_sessions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    file_size BIGINT NOT NULL,
    media_type VARCHAR(50) NOT NULL,
    s3_key VARCHAR(512) NOT NULL,
    store_behavior VARCHAR(10) NOT NULL DEFAULT 'auto',
    expires_at TIMESTAMPTZ NOT NULL,
    metadata JSONB DEFAULT '{}'::jsonb,
    status upload_session_status NOT NULL DEFAULT 'pending',
    file_id UUID,
    error_message TEXT,
    chunk_size BIGINT,
    chunk_count INTEGER,
    uploaded_size BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_presigned_upload_sessions_tenant_id ON presigned_upload_sessions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_presigned_upload_sessions_status ON presigned_upload_sessions(status);
CREATE INDEX IF NOT EXISTS idx_presigned_upload_sessions_expires_at ON presigned_upload_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_presigned_upload_sessions_file_id ON presigned_upload_sessions(file_id) WHERE file_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS upload_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES presigned_upload_sessions(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    s3_key VARCHAR(512) NOT NULL,
    size BIGINT NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, chunk_index)
);

CREATE INDEX IF NOT EXISTS idx_upload_chunks_session_id ON upload_chunks(session_id);
