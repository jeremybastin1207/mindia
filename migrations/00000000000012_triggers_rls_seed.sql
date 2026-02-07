-- Triggers (updated_at), RLS helpers, row-level security policies, seed data

CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

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

-- updated_at triggers (idempotent: drop then create)
DO $$
DECLARE
    tname text;
BEGIN
    FOREACH tname IN ARRAY [
        'tenants', 'folders', 'media', 'tasks', 'webhooks', 'webhook_retry_queue',
        'api_keys', 'service_api_keys', 'presigned_upload_sessions', 'plugin_configs',
        'plugin_executions', 'plugin_cost_summaries', 'embeddings', 'named_transformations'
    ]
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS trigger_%s_updated_at ON %I', tname, tname);
        EXECUTE format('CREATE TRIGGER trigger_%s_updated_at BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION update_updated_at()', tname, tname);
    END LOOP;
END
$$;

-- RLS: session helper and authorization check
CREATE OR REPLACE FUNCTION get_current_tenant_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_tenant_id', true), '')::UUID;
EXCEPTION
    WHEN OTHERS THEN
        RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION is_tenant_authorized(check_tenant_id UUID)
RETURNS BOOLEAN AS $$
BEGIN
    RETURN check_tenant_id = get_current_tenant_id();
END;
$$ LANGUAGE plpgsql STABLE;

-- Enable RLS and create policies (idempotent: drop policy if exists then create)
ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_select_policy ON tenants;
CREATE POLICY tenant_select_policy ON tenants FOR SELECT USING (id = get_current_tenant_id());
DROP POLICY IF EXISTS tenant_update_policy ON tenants;
CREATE POLICY tenant_update_policy ON tenants FOR UPDATE USING (id = get_current_tenant_id());

ALTER TABLE tenant_storage_config ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_storage_config_select_policy ON tenant_storage_config;
CREATE POLICY tenant_storage_config_select_policy ON tenant_storage_config FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS tenant_storage_config_insert_policy ON tenant_storage_config;
CREATE POLICY tenant_storage_config_insert_policy ON tenant_storage_config FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS tenant_storage_config_update_policy ON tenant_storage_config;
CREATE POLICY tenant_storage_config_update_policy ON tenant_storage_config FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS tenant_storage_config_delete_policy ON tenant_storage_config;
CREATE POLICY tenant_storage_config_delete_policy ON tenant_storage_config FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE folders ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS folders_select_policy ON folders;
CREATE POLICY folders_select_policy ON folders FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS folders_insert_policy ON folders;
CREATE POLICY folders_insert_policy ON folders FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS folders_update_policy ON folders;
CREATE POLICY folders_update_policy ON folders FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS folders_delete_policy ON folders;
CREATE POLICY folders_delete_policy ON folders FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE media ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS media_select_policy ON media;
CREATE POLICY media_select_policy ON media FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS media_insert_policy ON media;
CREATE POLICY media_insert_policy ON media FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS media_update_policy ON media;
CREATE POLICY media_update_policy ON media FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS media_delete_policy ON media;
CREATE POLICY media_delete_policy ON media FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE presigned_upload_sessions ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS presigned_upload_sessions_select_policy ON presigned_upload_sessions;
CREATE POLICY presigned_upload_sessions_select_policy ON presigned_upload_sessions FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS presigned_upload_sessions_insert_policy ON presigned_upload_sessions;
CREATE POLICY presigned_upload_sessions_insert_policy ON presigned_upload_sessions FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS presigned_upload_sessions_update_policy ON presigned_upload_sessions;
CREATE POLICY presigned_upload_sessions_update_policy ON presigned_upload_sessions FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS presigned_upload_sessions_delete_policy ON presigned_upload_sessions;
CREATE POLICY presigned_upload_sessions_delete_policy ON presigned_upload_sessions FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE upload_chunks ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS upload_chunks_select_policy ON upload_chunks;
CREATE POLICY upload_chunks_select_policy ON upload_chunks FOR SELECT USING (
    EXISTS (SELECT 1 FROM presigned_upload_sessions WHERE presigned_upload_sessions.id = upload_chunks.session_id AND presigned_upload_sessions.tenant_id = get_current_tenant_id())
);
DROP POLICY IF EXISTS upload_chunks_insert_policy ON upload_chunks;
CREATE POLICY upload_chunks_insert_policy ON upload_chunks FOR INSERT WITH CHECK (
    EXISTS (SELECT 1 FROM presigned_upload_sessions WHERE presigned_upload_sessions.id = upload_chunks.session_id AND presigned_upload_sessions.tenant_id = get_current_tenant_id())
);
DROP POLICY IF EXISTS upload_chunks_update_policy ON upload_chunks;
CREATE POLICY upload_chunks_update_policy ON upload_chunks FOR UPDATE USING (
    EXISTS (SELECT 1 FROM presigned_upload_sessions WHERE presigned_upload_sessions.id = upload_chunks.session_id AND presigned_upload_sessions.tenant_id = get_current_tenant_id())
);
DROP POLICY IF EXISTS upload_chunks_delete_policy ON upload_chunks;
CREATE POLICY upload_chunks_delete_policy ON upload_chunks FOR DELETE USING (
    EXISTS (SELECT 1 FROM presigned_upload_sessions WHERE presigned_upload_sessions.id = upload_chunks.session_id AND presigned_upload_sessions.tenant_id = get_current_tenant_id())
);

ALTER TABLE request_logs ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS request_logs_select_policy ON request_logs;
CREATE POLICY request_logs_select_policy ON request_logs FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS request_logs_insert_policy ON request_logs;
CREATE POLICY request_logs_insert_policy ON request_logs FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());

ALTER TABLE storage_metrics ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS storage_metrics_select_policy ON storage_metrics;
CREATE POLICY storage_metrics_select_policy ON storage_metrics FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS storage_metrics_insert_policy ON storage_metrics;
CREATE POLICY storage_metrics_insert_policy ON storage_metrics FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS storage_metrics_update_policy ON storage_metrics;
CREATE POLICY storage_metrics_update_policy ON storage_metrics FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS storage_metrics_delete_policy ON storage_metrics;
CREATE POLICY storage_metrics_delete_policy ON storage_metrics FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE embeddings ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS embeddings_select_policy ON embeddings;
CREATE POLICY embeddings_select_policy ON embeddings FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS embeddings_insert_policy ON embeddings;
CREATE POLICY embeddings_insert_policy ON embeddings FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS embeddings_update_policy ON embeddings;
CREATE POLICY embeddings_update_policy ON embeddings FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS embeddings_delete_policy ON embeddings;
CREATE POLICY embeddings_delete_policy ON embeddings FOR DELETE USING (tenant_id = get_current_tenant_id());

-- Tasks, webhook_events, webhook_retry_queue, plugin_executions: RLS disabled for system workers.

ALTER TABLE api_keys ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS api_keys_select_policy ON api_keys;
CREATE POLICY api_keys_select_policy ON api_keys FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS api_keys_insert_policy ON api_keys;
CREATE POLICY api_keys_insert_policy ON api_keys FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS api_keys_update_policy ON api_keys;
CREATE POLICY api_keys_update_policy ON api_keys FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS api_keys_delete_policy ON api_keys;
CREATE POLICY api_keys_delete_policy ON api_keys FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE file_groups ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS file_groups_select_policy ON file_groups;
CREATE POLICY file_groups_select_policy ON file_groups FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS file_groups_insert_policy ON file_groups;
CREATE POLICY file_groups_insert_policy ON file_groups FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS file_groups_update_policy ON file_groups;
CREATE POLICY file_groups_update_policy ON file_groups FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS file_groups_delete_policy ON file_groups;
CREATE POLICY file_groups_delete_policy ON file_groups FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE file_group_items ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS file_group_items_select_policy ON file_group_items;
CREATE POLICY file_group_items_select_policy ON file_group_items FOR SELECT USING (
    EXISTS (SELECT 1 FROM file_groups WHERE file_groups.id = file_group_items.group_id AND file_groups.tenant_id = get_current_tenant_id())
);
DROP POLICY IF EXISTS file_group_items_insert_policy ON file_group_items;
CREATE POLICY file_group_items_insert_policy ON file_group_items FOR INSERT WITH CHECK (
    EXISTS (SELECT 1 FROM file_groups WHERE file_groups.id = file_group_items.group_id AND file_groups.tenant_id = get_current_tenant_id())
    AND EXISTS (SELECT 1 FROM media WHERE media.id = file_group_items.media_id AND media.tenant_id = get_current_tenant_id())
);
DROP POLICY IF EXISTS file_group_items_update_policy ON file_group_items;
CREATE POLICY file_group_items_update_policy ON file_group_items FOR UPDATE USING (
    EXISTS (SELECT 1 FROM file_groups WHERE file_groups.id = file_group_items.group_id AND file_groups.tenant_id = get_current_tenant_id())
    AND EXISTS (SELECT 1 FROM media WHERE media.id = file_group_items.media_id AND media.tenant_id = get_current_tenant_id())
) WITH CHECK (
    EXISTS (SELECT 1 FROM file_groups WHERE file_groups.id = file_group_items.group_id AND file_groups.tenant_id = get_current_tenant_id())
    AND EXISTS (SELECT 1 FROM media WHERE media.id = file_group_items.media_id AND media.tenant_id = get_current_tenant_id())
);
DROP POLICY IF EXISTS file_group_items_delete_policy ON file_group_items;
CREATE POLICY file_group_items_delete_policy ON file_group_items FOR DELETE USING (
    EXISTS (SELECT 1 FROM file_groups WHERE file_groups.id = file_group_items.group_id AND file_groups.tenant_id = get_current_tenant_id())
    AND EXISTS (SELECT 1 FROM media WHERE media.id = file_group_items.media_id AND media.tenant_id = get_current_tenant_id())
);

ALTER TABLE plugin_configs ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS plugin_configs_select_policy ON plugin_configs;
CREATE POLICY plugin_configs_select_policy ON plugin_configs FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS plugin_configs_insert_policy ON plugin_configs;
CREATE POLICY plugin_configs_insert_policy ON plugin_configs FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS plugin_configs_update_policy ON plugin_configs;
CREATE POLICY plugin_configs_update_policy ON plugin_configs FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS plugin_configs_delete_policy ON plugin_configs;
CREATE POLICY plugin_configs_delete_policy ON plugin_configs FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE plugin_cost_summaries ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS plugin_cost_summaries_select_policy ON plugin_cost_summaries;
CREATE POLICY plugin_cost_summaries_select_policy ON plugin_cost_summaries FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS plugin_cost_summaries_insert_policy ON plugin_cost_summaries;
CREATE POLICY plugin_cost_summaries_insert_policy ON plugin_cost_summaries FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS plugin_cost_summaries_update_policy ON plugin_cost_summaries;
CREATE POLICY plugin_cost_summaries_update_policy ON plugin_cost_summaries FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS plugin_cost_summaries_delete_policy ON plugin_cost_summaries;
CREATE POLICY plugin_cost_summaries_delete_policy ON plugin_cost_summaries FOR DELETE USING (tenant_id = get_current_tenant_id());

ALTER TABLE named_transformations ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS named_transformations_select_policy ON named_transformations;
CREATE POLICY named_transformations_select_policy ON named_transformations FOR SELECT USING (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS named_transformations_insert_policy ON named_transformations;
CREATE POLICY named_transformations_insert_policy ON named_transformations FOR INSERT WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS named_transformations_update_policy ON named_transformations;
CREATE POLICY named_transformations_update_policy ON named_transformations FOR UPDATE USING (tenant_id = get_current_tenant_id()) WITH CHECK (tenant_id = get_current_tenant_id());
DROP POLICY IF EXISTS named_transformations_delete_policy ON named_transformations;
CREATE POLICY named_transformations_delete_policy ON named_transformations FOR DELETE USING (tenant_id = get_current_tenant_id());

-- Seed default tenant (master key auth uses DEFAULT_TENANT_ID, see mindia_core::constants)
INSERT INTO tenants (id, name, status)
VALUES ('d2e8f4a1-7b3c-5d6e-8f9a-0b1c2d3e4f5a'::uuid, 'default', 'active')
ON CONFLICT (id) DO NOTHING;

-- Seed default tenant storage (S3_BUCKET / S3_REGION at runtime)
INSERT INTO tenant_storage_config (tenant_id, storage_backend, s3_bucket, s3_region)
VALUES ('d2e8f4a1-7b3c-5d6e-8f9a-0b1c2d3e4f5a'::uuid, 's3', 'default', 'us-east-1')
ON CONFLICT (tenant_id) DO NOTHING;
