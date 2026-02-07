-- Plugins (config, executions, cost summaries)

DO $$ BEGIN
    CREATE TYPE plugin_execution_status AS ENUM ('pending', 'running', 'completed', 'failed');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

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

CREATE INDEX IF NOT EXISTS idx_plugin_configs_tenant_id ON plugin_configs(tenant_id);
CREATE INDEX IF NOT EXISTS idx_plugin_configs_plugin_name ON plugin_configs(plugin_name);
CREATE INDEX IF NOT EXISTS idx_plugin_configs_enabled ON plugin_configs(tenant_id, enabled) WHERE enabled = true;
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

CREATE INDEX IF NOT EXISTS idx_plugin_executions_tenant_id ON plugin_executions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_plugin_executions_plugin_name ON plugin_executions(plugin_name);
CREATE INDEX IF NOT EXISTS idx_plugin_executions_media_id ON plugin_executions(media_id);
CREATE INDEX IF NOT EXISTS idx_plugin_executions_task_id ON plugin_executions(task_id);
CREATE INDEX IF NOT EXISTS idx_plugin_executions_status ON plugin_executions(status);
CREATE INDEX IF NOT EXISTS idx_plugin_executions_tenant_plugin ON plugin_executions(tenant_id, plugin_name);

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

CREATE INDEX IF NOT EXISTS idx_plugin_cost_summaries_tenant_id ON plugin_cost_summaries(tenant_id);
CREATE INDEX IF NOT EXISTS idx_plugin_cost_summaries_tenant_plugin ON plugin_cost_summaries(tenant_id, plugin_name);
CREATE INDEX IF NOT EXISTS idx_plugin_cost_summaries_tenant_period ON plugin_cost_summaries(tenant_id, period_start, period_end);
CREATE INDEX IF NOT EXISTS idx_plugin_cost_summaries_period ON plugin_cost_summaries(period_start, period_end);
