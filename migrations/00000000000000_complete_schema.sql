-- MINDIA COMPLETE SCHEMA (single consolidated migration)
-- Includes default tenant seed.

-- MULTI-TENANCY
CREATE TYPE tenant_status AS ENUM ('active', 'suspended', 'deleted');

CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    status tenant_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tenants_status ON tenants(status);
CREATE INDEX idx_tenants_name ON tenants(name);

-- STORAGE BACKEND ENUM
CREATE TYPE storage_backend AS ENUM ('s3', 'local', 'nfs');

-- TENANT STORAGE CONFIGURATION
-- Decouples storage configuration from tenant entities
CREATE TABLE IF NOT EXISTS tenant_storage_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL UNIQUE REFERENCES tenants(id) ON DELETE CASCADE,
    storage_backend storage_backend NOT NULL DEFAULT 's3',
    s3_bucket VARCHAR(255),
    s3_region VARCHAR(100) DEFAULT 'us-east-1',
    s3_endpoint VARCHAR(255),  -- Optional: for S3-compatible services
    local_path VARCHAR(512),   -- For local storage backend
    nfs_mount VARCHAR(512),    -- For NFS backend
    config JSONB DEFAULT '{}'::jsonb,  -- Additional backend-specific configuration
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Ensure required fields are set based on backend type
    CHECK (
        (storage_backend = 's3' AND s3_bucket IS NOT NULL) OR
        (storage_backend = 'local' AND local_path IS NOT NULL) OR
        (storage_backend = 'nfs' AND nfs_mount IS NOT NULL)
    )
);

CREATE INDEX idx_tenant_storage_config_tenant_id ON tenant_storage_config(tenant_id);
CREATE INDEX idx_tenant_storage_config_backend ON tenant_storage_config(storage_backend);

-- FOLDERS
CREATE TABLE IF NOT EXISTS folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    parent_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, parent_id, name)
);

CREATE INDEX idx_folders_parent_id ON folders(parent_id);
CREATE INDEX idx_folders_tenant_parent ON folders(tenant_id, parent_id);
CREATE INDEX idx_folders_created_at ON folders(created_at DESC);

-- STORAGE LOCATIONS (backend-agnostic storage references)
CREATE TABLE IF NOT EXISTS storage_locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    backend storage_backend NOT NULL,
    bucket VARCHAR(255),
    key VARCHAR(512) NOT NULL,
    url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(backend, bucket, key)
);

CREATE INDEX idx_storage_locations_backend_bucket_key ON storage_locations(backend, bucket, key);
CREATE INDEX idx_storage_locations_created_at ON storage_locations(created_at DESC);

-- MEDIA (unified, references storage_locations)
CREATE TYPE media_type AS ENUM ('image', 'video', 'audio', 'document');
CREATE TYPE processing_status AS ENUM ('pending', 'processing', 'completed', 'failed');

CREATE TABLE IF NOT EXISTS media (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    storage_id UUID NOT NULL REFERENCES storage_locations(id) ON DELETE RESTRICT,
    media_type media_type NOT NULL,
    filename VARCHAR(255) NOT NULL,
    original_filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    file_size BIGINT NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    store_behavior VARCHAR(10) NOT NULL DEFAULT 'auto',
    store_permanently BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,
    folder_id UUID REFERENCES folders(id) ON DELETE SET NULL,
    metadata JSONB DEFAULT '{}'::jsonb,
    type_metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_media_tenant_type ON media(tenant_id, media_type);
CREATE INDEX idx_media_tenant_uploaded ON media(tenant_id, uploaded_at DESC);
CREATE INDEX idx_media_tenant_type_uploaded ON media(tenant_id, media_type, uploaded_at DESC);
CREATE INDEX idx_media_storage_id ON media(storage_id);
CREATE INDEX idx_media_uploaded_at ON media(uploaded_at DESC);
CREATE INDEX idx_media_filename ON media(filename);
CREATE INDEX idx_media_original_filename ON media(original_filename);
CREATE INDEX idx_media_expires_at ON media(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_media_store_permanently ON media(store_permanently, expires_at);
CREATE INDEX idx_media_folder_id ON media(folder_id) WHERE folder_id IS NOT NULL;
CREATE INDEX idx_media_tenant_folder ON media(tenant_id, folder_id);
CREATE INDEX idx_media_tenant_type_folder ON media(tenant_id, media_type, folder_id);
CREATE INDEX idx_media_metadata ON media USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_media_metadata_user ON media USING GIN ((metadata->'user'));
CREATE INDEX idx_media_type_metadata ON media USING GIN (type_metadata);
CREATE INDEX idx_media_type_metadata_processing ON media((type_metadata->>'processing_status')) WHERE type_metadata ? 'processing_status';

-- PRESIGNED UPLOADS
CREATE TYPE upload_session_status AS ENUM ('pending', 'uploading', 'completed', 'failed');

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

CREATE INDEX idx_presigned_upload_sessions_tenant_id ON presigned_upload_sessions(tenant_id);
CREATE INDEX idx_presigned_upload_sessions_status ON presigned_upload_sessions(status);
CREATE INDEX idx_presigned_upload_sessions_expires_at ON presigned_upload_sessions(expires_at);
CREATE INDEX idx_presigned_upload_sessions_file_id ON presigned_upload_sessions(file_id) WHERE file_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS upload_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES presigned_upload_sessions(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    s3_key VARCHAR(512) NOT NULL,
    size BIGINT NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, chunk_index)
);

CREATE INDEX idx_upload_chunks_session_id ON upload_chunks(session_id);

-- ANALYTICS
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

CREATE INDEX idx_request_logs_tenant_id ON request_logs(tenant_id);
CREATE INDEX idx_request_logs_tenant_created ON request_logs(tenant_id, created_at DESC);
CREATE INDEX idx_request_logs_path ON request_logs(normalized_path);
CREATE INDEX idx_request_logs_created_at ON request_logs(created_at DESC);
CREATE INDEX idx_request_logs_status_code ON request_logs(status_code);
CREATE INDEX idx_request_logs_method ON request_logs(method);
CREATE INDEX idx_request_logs_path_created ON request_logs(normalized_path, created_at DESC);

CREATE TABLE IF NOT EXISTS storage_metrics (
    id SERIAL PRIMARY KEY,
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE, -- NULL means system-wide metrics
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

CREATE INDEX idx_storage_metrics_tenant_id ON storage_metrics(tenant_id);
CREATE INDEX idx_storage_metrics_tenant_created ON storage_metrics(tenant_id, created_at DESC);
CREATE INDEX idx_storage_metrics_created_at ON storage_metrics(created_at DESC);

-- SEMANTIC SEARCH (pgvector)
CREATE EXTENSION IF NOT EXISTS vector;
CREATE TYPE entity_type AS ENUM ('image', 'video', 'audio', 'document');

CREATE TABLE IF NOT EXISTS embeddings (
    id SERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    entity_id UUID NOT NULL,
    entity_type entity_type NOT NULL,
    description TEXT NOT NULL,
    embedding vector(768) NOT NULL,
    model_name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(entity_id, entity_type)
);

CREATE INDEX idx_embeddings_tenant_id ON embeddings(tenant_id);
CREATE INDEX idx_embeddings_tenant_entity ON embeddings(tenant_id, entity_id, entity_type);
CREATE INDEX idx_embeddings_entity_id ON embeddings(entity_id);

CREATE INDEX idx_embeddings_vector ON embeddings USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- TASKS
CREATE TYPE task_status AS ENUM ('pending', 'running', 'completed', 'failed', 'scheduled', 'cancelled');

CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    task_type TEXT NOT NULL,
    status task_status NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 5,
    payload JSONB NOT NULL DEFAULT '{}',
    result JSONB,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    timeout_seconds INTEGER,
    depends_on UUID[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tasks_tenant_id ON tasks(tenant_id);
CREATE INDEX idx_tasks_tenant_status ON tasks(tenant_id, status);
CREATE INDEX idx_tasks_tenant_queue ON tasks(tenant_id, status, scheduled_at, priority DESC)
    WHERE status IN ('pending', 'scheduled');
CREATE INDEX idx_tasks_queue_polling ON tasks(status, scheduled_at, priority DESC)
    WHERE status IN ('pending', 'scheduled');
CREATE INDEX idx_tasks_depends_on ON tasks USING GIN(depends_on)
    WHERE depends_on IS NOT NULL;
CREATE INDEX idx_tasks_created_at ON tasks(created_at);
CREATE INDEX idx_tasks_completed_at ON tasks(completed_at)
    WHERE completed_at IS NOT NULL;

ALTER TABLE tasks ADD CONSTRAINT check_priority
    CHECK (priority >= 0 AND priority <= 10);

ALTER TABLE tasks ADD CONSTRAINT check_retry_count
    CHECK (retry_count >= 0);

-- WEBHOOKS
CREATE TYPE webhook_event_type AS ENUM (
    'file.uploaded',
    'file.deleted',
    'file.stored',
    'file.processing_completed',
    'file.processing_failed'
);

CREATE TYPE webhook_delivery_status AS ENUM (
    'pending',
    'success',
    'failed',
    'retrying'
);

CREATE TABLE IF NOT EXISTS webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    event_type webhook_event_type NOT NULL,
    signing_secret VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT true,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deactivated_at TIMESTAMPTZ,
    deactivation_reason TEXT
);

CREATE INDEX idx_webhooks_tenant_event_active 
    ON webhooks(tenant_id, event_type, is_active) 
    WHERE is_active = true;
CREATE INDEX idx_webhooks_tenant_id ON webhooks(tenant_id);
CREATE INDEX idx_webhooks_event_type ON webhooks(event_type);
CREATE INDEX idx_webhooks_active ON webhooks(is_active) WHERE is_active = true;

CREATE TABLE IF NOT EXISTS webhook_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    webhook_id UUID NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    event_type webhook_event_type NOT NULL,
    payload JSONB NOT NULL,
    status webhook_delivery_status NOT NULL DEFAULT 'pending',
    response_status_code INTEGER,
    response_body TEXT,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_webhook_events_webhook_id ON webhook_events(webhook_id, created_at DESC);
CREATE INDEX idx_webhook_events_tenant_id ON webhook_events(tenant_id, created_at DESC);
CREATE INDEX idx_webhook_events_status ON webhook_events(status);
CREATE INDEX idx_webhook_events_failed_retry 
    ON webhook_events(webhook_id, status, created_at) 
    WHERE status IN ('failed', 'retrying');

CREATE TABLE IF NOT EXISTS webhook_retry_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    webhook_event_id UUID NOT NULL UNIQUE REFERENCES webhook_events(id) ON DELETE CASCADE,
    webhook_id UUID NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 72,
    next_retry_at TIMESTAMPTZ NOT NULL,
    last_attempt_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_retry_queue_next_retry ON webhook_retry_queue(next_retry_at);
CREATE INDEX idx_webhook_retry_queue_event ON webhook_retry_queue(webhook_event_id);
CREATE INDEX idx_webhook_retry_queue_webhook ON webhook_retry_queue(webhook_id);
CREATE INDEX idx_webhook_retry_queue_tenant ON webhook_retry_queue(tenant_id);

-- API KEYS
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    key_prefix VARCHAR(16) NOT NULL,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_tenant_id ON api_keys(tenant_id);
CREATE INDEX idx_api_keys_key_prefix ON api_keys(key_prefix);
CREATE INDEX idx_api_keys_is_active ON api_keys(is_active);
CREATE INDEX idx_api_keys_tenant_active ON api_keys(tenant_id, is_active) WHERE is_active = true;

-- SERVICE API KEYS
CREATE TABLE IF NOT EXISTS service_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_name VARCHAR(100) NOT NULL UNIQUE,
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    key_prefix VARCHAR(20) NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_service_api_keys_key_prefix ON service_api_keys(key_prefix);
CREATE INDEX idx_service_api_keys_is_active ON service_api_keys(is_active) WHERE is_active = true;

-- FILE GROUPS
CREATE TABLE IF NOT EXISTS file_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS file_group_items (
    group_id UUID NOT NULL REFERENCES file_groups(id) ON DELETE CASCADE,
    media_id UUID NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    index INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, index),
    UNIQUE(group_id, media_id)
);

CREATE INDEX idx_file_groups_tenant_id ON file_groups(tenant_id);
CREATE INDEX idx_file_groups_created_at ON file_groups(created_at DESC);

CREATE INDEX idx_file_group_items_group_id ON file_group_items(group_id);
CREATE INDEX idx_file_group_items_media_id ON file_group_items(media_id);
CREATE INDEX idx_file_group_items_group_index ON file_group_items(group_id, index);

-- PLUGINS
CREATE TYPE plugin_execution_status AS ENUM ('pending', 'running', 'completed', 'failed');

CREATE TABLE IF NOT EXISTS plugin_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    plugin_name VARCHAR(255) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT false,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    encrypted_config TEXT,
    uses_encryption BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, plugin_name)
);

CREATE INDEX idx_plugin_configs_tenant_id ON plugin_configs(tenant_id);
CREATE INDEX idx_plugin_configs_plugin_name ON plugin_configs(plugin_name);
CREATE INDEX idx_plugin_configs_enabled ON plugin_configs(tenant_id, enabled) WHERE enabled = true;
CREATE INDEX IF NOT EXISTS idx_plugin_configs_encrypted ON plugin_configs(tenant_id, plugin_name) WHERE uses_encryption = true;

CREATE TABLE IF NOT EXISTS plugin_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    plugin_name VARCHAR(255) NOT NULL,
    media_id UUID NOT NULL,
    task_id UUID REFERENCES tasks(id) ON DELETE SET NULL,
    status plugin_execution_status NOT NULL DEFAULT 'pending',
    result JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    usage_unit_type VARCHAR(50),
    usage_input_units BIGINT,
    usage_output_units BIGINT,
    usage_total_units BIGINT,
    usage_raw JSONB
);

CREATE INDEX idx_plugin_executions_tenant_id ON plugin_executions(tenant_id);
CREATE INDEX idx_plugin_executions_plugin_name ON plugin_executions(plugin_name);
CREATE INDEX idx_plugin_executions_media_id ON plugin_executions(media_id);
CREATE INDEX idx_plugin_executions_task_id ON plugin_executions(task_id);
CREATE INDEX idx_plugin_executions_status ON plugin_executions(status);
CREATE INDEX idx_plugin_executions_tenant_plugin ON plugin_executions(tenant_id, plugin_name);

-- PLUGIN COST SUMMARIES (aggregated usage per tenant/plugin/period)
CREATE TABLE IF NOT EXISTS plugin_cost_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    plugin_name VARCHAR(255) NOT NULL,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    execution_count BIGINT NOT NULL DEFAULT 0,
    total_units BIGINT NOT NULL DEFAULT 0,
    unit_type VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_plugin_cost_summaries_tenant_id ON plugin_cost_summaries(tenant_id);
CREATE INDEX idx_plugin_cost_summaries_tenant_plugin ON plugin_cost_summaries(tenant_id, plugin_name);
CREATE INDEX idx_plugin_cost_summaries_tenant_period ON plugin_cost_summaries(tenant_id, period_start, period_end);
CREATE INDEX idx_plugin_cost_summaries_period ON plugin_cost_summaries(period_start, period_end);

-- NAMED TRANSFORMATIONS (reusable transformation presets)
CREATE TABLE IF NOT EXISTS named_transformations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    operations TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

CREATE INDEX idx_named_transformations_tenant_id ON named_transformations(tenant_id);
CREATE INDEX idx_named_transformations_tenant_name ON named_transformations(tenant_id, name);

-- TRIGGERS AND FUNCTIONS

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for tenants
CREATE TRIGGER trigger_tenants_updated_at
    BEFORE UPDATE ON tenants
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Trigger for folders
CREATE TRIGGER trigger_folders_updated_at
    BEFORE UPDATE ON folders
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Trigger for media
CREATE TRIGGER trigger_media_updated_at
    BEFORE UPDATE ON media
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Trigger for tasks
CREATE TRIGGER trigger_tasks_updated_at
    BEFORE UPDATE ON tasks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Trigger for webhooks
CREATE TRIGGER trigger_webhooks_updated_at
    BEFORE UPDATE ON webhooks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Trigger for webhook_retry_queue
CREATE TRIGGER trigger_webhook_retry_queue_updated_at
    BEFORE UPDATE ON webhook_retry_queue
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Trigger for api_keys
CREATE TRIGGER trigger_api_keys_updated_at
    BEFORE UPDATE ON api_keys
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Trigger for service_api_keys
CREATE TRIGGER trigger_service_api_keys_updated_at
    BEFORE UPDATE ON service_api_keys
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Trigger for presigned_upload_sessions
CREATE TRIGGER trigger_presigned_upload_sessions_updated_at
    BEFORE UPDATE ON presigned_upload_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_plugin_configs_updated_at
    BEFORE UPDATE ON plugin_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_plugin_executions_updated_at
    BEFORE UPDATE ON plugin_executions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_plugin_cost_summaries_updated_at
    BEFORE UPDATE ON plugin_cost_summaries
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_embeddings_updated_at
    BEFORE UPDATE ON embeddings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_named_transformations_updated_at
    BEFORE UPDATE ON named_transformations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Function to calculate next retry time based on exponential backoff
CREATE OR REPLACE FUNCTION calculate_next_retry_time(retry_count INTEGER)
RETURNS INTERVAL AS $$
BEGIN
    RETURN CASE
        WHEN retry_count = 0 THEN INTERVAL '1 minute'
        WHEN retry_count = 1 THEN INTERVAL '5 minutes'
        WHEN retry_count = 2 THEN INTERVAL '10 minutes'
        WHEN retry_count = 3 THEN INTERVAL '30 minutes'
        WHEN retry_count = 4 THEN INTERVAL '60 minutes'
        ELSE INTERVAL '1 hour'
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- RLS
CREATE OR REPLACE FUNCTION get_current_tenant_id()
RETURNS UUID AS $$
BEGIN
    -- Try to get tenant_id from session variable
    RETURN NULLIF(current_setting('app.current_tenant_id', true), '')::UUID;
EXCEPTION
    WHEN OTHERS THEN
        -- If not set or invalid, return NULL (will deny access)
        RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION is_tenant_authorized(check_tenant_id UUID)
RETURNS BOOLEAN AS $$
BEGIN
    RETURN check_tenant_id = get_current_tenant_id();
END;
$$ LANGUAGE plpgsql STABLE;

ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_select_policy ON tenants
    FOR SELECT
    USING (id = get_current_tenant_id());

CREATE POLICY tenant_update_policy ON tenants
    FOR UPDATE
    USING (id = get_current_tenant_id());

-- ============================================================================
-- RLS POLICIES FOR TENANT STORAGE CONFIG TABLE
-- ============================================================================

ALTER TABLE tenant_storage_config ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_storage_config_select_policy ON tenant_storage_config
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY tenant_storage_config_insert_policy ON tenant_storage_config
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY tenant_storage_config_update_policy ON tenant_storage_config
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY tenant_storage_config_delete_policy ON tenant_storage_config
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

-- ============================================================================
-- RLS POLICIES FOR FOLDERS TABLE
-- ============================================================================

ALTER TABLE folders ENABLE ROW LEVEL SECURITY;

CREATE POLICY folders_select_policy ON folders
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY folders_insert_policy ON folders
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY folders_update_policy ON folders
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY folders_delete_policy ON folders
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

ALTER TABLE media ENABLE ROW LEVEL SECURITY;

CREATE POLICY media_select_policy ON media
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY media_insert_policy ON media
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY media_update_policy ON media
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY media_delete_policy ON media
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

ALTER TABLE presigned_upload_sessions ENABLE ROW LEVEL SECURITY;

CREATE POLICY presigned_upload_sessions_select_policy ON presigned_upload_sessions
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY presigned_upload_sessions_insert_policy ON presigned_upload_sessions
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY presigned_upload_sessions_update_policy ON presigned_upload_sessions
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY presigned_upload_sessions_delete_policy ON presigned_upload_sessions
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

ALTER TABLE upload_chunks ENABLE ROW LEVEL SECURITY;

CREATE POLICY upload_chunks_select_policy ON upload_chunks
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM presigned_upload_sessions
            WHERE presigned_upload_sessions.id = upload_chunks.session_id
            AND presigned_upload_sessions.tenant_id = get_current_tenant_id()
        )
    );

CREATE POLICY upload_chunks_insert_policy ON upload_chunks
    FOR INSERT
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM presigned_upload_sessions
            WHERE presigned_upload_sessions.id = upload_chunks.session_id
            AND presigned_upload_sessions.tenant_id = get_current_tenant_id()
        )
    );

CREATE POLICY upload_chunks_update_policy ON upload_chunks
    FOR UPDATE
    USING (
        EXISTS (
            SELECT 1 FROM presigned_upload_sessions
            WHERE presigned_upload_sessions.id = upload_chunks.session_id
            AND presigned_upload_sessions.tenant_id = get_current_tenant_id()
        )
    );

CREATE POLICY upload_chunks_delete_policy ON upload_chunks
    FOR DELETE
    USING (
        EXISTS (
            SELECT 1 FROM presigned_upload_sessions
            WHERE presigned_upload_sessions.id = upload_chunks.session_id
            AND presigned_upload_sessions.tenant_id = get_current_tenant_id()
        )
    );

-- ============================================================================
-- RLS POLICIES FOR REQUEST LOGS TABLE
-- ============================================================================

ALTER TABLE request_logs ENABLE ROW LEVEL SECURITY;

CREATE POLICY request_logs_select_policy ON request_logs
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY request_logs_insert_policy ON request_logs
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

ALTER TABLE storage_metrics ENABLE ROW LEVEL SECURITY;

CREATE POLICY storage_metrics_select_policy ON storage_metrics
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY storage_metrics_insert_policy ON storage_metrics
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY storage_metrics_update_policy ON storage_metrics
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY storage_metrics_delete_policy ON storage_metrics
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

ALTER TABLE embeddings ENABLE ROW LEVEL SECURITY;

CREATE POLICY embeddings_select_policy ON embeddings
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY embeddings_insert_policy ON embeddings
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY embeddings_update_policy ON embeddings
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY embeddings_delete_policy ON embeddings
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

-- ============================================================================
-- RLS DISABLED FOR SYSTEM OPERATION TABLES
-- ============================================================================
-- Tasks, webhooks, webhook_events, webhook_retry_queue, and plugin_executions 
-- have RLS DISABLED to allow system workers and background operations to access 
-- these tables without session context. These tables are already scoped by 
-- tenant_id column and application code verifies tenant access appropriately.
-- This allows the shared worker pool to claim tasks, trigger webhooks, and 
-- execute plugins across tenants.

-- ============================================================================
-- RLS POLICIES FOR API KEYS TABLE
-- ============================================================================

ALTER TABLE api_keys ENABLE ROW LEVEL SECURITY;

CREATE POLICY api_keys_select_policy ON api_keys
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY api_keys_insert_policy ON api_keys
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY api_keys_update_policy ON api_keys
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY api_keys_delete_policy ON api_keys
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

ALTER TABLE file_groups ENABLE ROW LEVEL SECURITY;

CREATE POLICY file_groups_select_policy ON file_groups
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY file_groups_insert_policy ON file_groups
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY file_groups_update_policy ON file_groups
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY file_groups_delete_policy ON file_groups
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

ALTER TABLE file_group_items ENABLE ROW LEVEL SECURITY;

CREATE POLICY file_group_items_select_policy ON file_group_items
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM file_groups
            WHERE file_groups.id = file_group_items.group_id
            AND file_groups.tenant_id = get_current_tenant_id()
        )
    );

CREATE POLICY file_group_items_insert_policy ON file_group_items
    FOR INSERT
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM file_groups
            WHERE file_groups.id = file_group_items.group_id
            AND file_groups.tenant_id = get_current_tenant_id()
        )
        AND
        EXISTS (
            SELECT 1 FROM media
            WHERE media.id = file_group_items.media_id
            AND media.tenant_id = get_current_tenant_id()
        )
    );

CREATE POLICY file_group_items_update_policy ON file_group_items
    FOR UPDATE
    USING (
        EXISTS (
            SELECT 1 FROM file_groups
            WHERE file_groups.id = file_group_items.group_id
            AND file_groups.tenant_id = get_current_tenant_id()
        )
        AND
        EXISTS (
            SELECT 1 FROM media
            WHERE media.id = file_group_items.media_id
            AND media.tenant_id = get_current_tenant_id()
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM file_groups
            WHERE file_groups.id = file_group_items.group_id
            AND file_groups.tenant_id = get_current_tenant_id()
        )
        AND
        EXISTS (
            SELECT 1 FROM media
            WHERE media.id = file_group_items.media_id
            AND media.tenant_id = get_current_tenant_id()
        )
    );

CREATE POLICY file_group_items_delete_policy ON file_group_items
    FOR DELETE
    USING (
        EXISTS (
            SELECT 1 FROM file_groups
            WHERE file_groups.id = file_group_items.group_id
            AND file_groups.tenant_id = get_current_tenant_id()
        )
        AND
        EXISTS (
            SELECT 1 FROM media
            WHERE media.id = file_group_items.media_id
            AND media.tenant_id = get_current_tenant_id()
        )
    );

ALTER TABLE plugin_configs ENABLE ROW LEVEL SECURITY;

CREATE POLICY plugin_configs_select_policy ON plugin_configs
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY plugin_configs_insert_policy ON plugin_configs
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY plugin_configs_update_policy ON plugin_configs
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY plugin_configs_delete_policy ON plugin_configs
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

ALTER TABLE plugin_cost_summaries ENABLE ROW LEVEL SECURITY;

CREATE POLICY plugin_cost_summaries_select_policy ON plugin_cost_summaries
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY plugin_cost_summaries_insert_policy ON plugin_cost_summaries
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY plugin_cost_summaries_update_policy ON plugin_cost_summaries
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY plugin_cost_summaries_delete_policy ON plugin_cost_summaries
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

-- RLS disabled for plugin_executions (see SYSTEM OPERATION TABLES section above)

-- ============================================================================
-- RLS POLICIES FOR NAMED TRANSFORMATIONS TABLE
-- ============================================================================

ALTER TABLE named_transformations ENABLE ROW LEVEL SECURITY;

CREATE POLICY named_transformations_select_policy ON named_transformations
    FOR SELECT
    USING (tenant_id = get_current_tenant_id());

CREATE POLICY named_transformations_insert_policy ON named_transformations
    FOR INSERT
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY named_transformations_update_policy ON named_transformations
    FOR UPDATE
    USING (tenant_id = get_current_tenant_id())
    WITH CHECK (tenant_id = get_current_tenant_id());

CREATE POLICY named_transformations_delete_policy ON named_transformations
    FOR DELETE
    USING (tenant_id = get_current_tenant_id());

-- Seed default tenant used by master key authentication
-- Auth middleware uses DEFAULT_TENANT_ID (d2e8f4a1-7b3c-5d6e-8f9a-0b1c2d3e4f5a), see mindia_core::constants
INSERT INTO tenants (id, name, status)
VALUES (
    'd2e8f4a1-7b3c-5d6e-8f9a-0b1c2d3e4f5a'::uuid,
    'default',
    'active'
)
ON CONFLICT (id) DO NOTHING;

-- Seed default tenant storage configuration
-- Uses environment variables S3_BUCKET and S3_REGION at runtime
INSERT INTO tenant_storage_config (tenant_id, storage_backend, s3_bucket, s3_region)
VALUES (
    'd2e8f4a1-7b3c-5d6e-8f9a-0b1c2d3e4f5a'::uuid,
    's3',
    'default',  -- Override with environment variable S3_BUCKET
    'us-east-1' -- Override with environment variable S3_REGION
)
ON CONFLICT (tenant_id) DO NOTHING;
