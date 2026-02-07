-- Workflows: execution status enum and workflow/workflow_execution tables

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workflow_execution_status') THEN
        CREATE TYPE workflow_execution_status AS ENUM (
            'pending',
            'running',
            'completed',
            'failed',
            'cancelled'
        );
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS workflows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT true,
    steps JSONB NOT NULL DEFAULT '[]'::jsonb,
    trigger_on_upload BOOLEAN NOT NULL DEFAULT true,
    stop_on_failure BOOLEAN NOT NULL DEFAULT true,
    media_types TEXT[],
    folder_ids UUID[],
    content_types TEXT[],
    metadata_filter JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_workflows_tenant_id ON workflows(tenant_id);
CREATE INDEX IF NOT EXISTS idx_workflows_tenant_enabled_upload ON workflows(tenant_id, enabled, trigger_on_upload)
    WHERE enabled = true AND trigger_on_upload = true;

CREATE TABLE IF NOT EXISTS workflow_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    media_id UUID NOT NULL,
    status workflow_execution_status NOT NULL DEFAULT 'pending',
    task_ids UUID[] NOT NULL DEFAULT '{}',
    current_step INTEGER NOT NULL DEFAULT 0,
    stop_on_failure BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_workflow_executions_workflow_id ON workflow_executions(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_executions_tenant_id ON workflow_executions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_workflow_executions_media_id ON workflow_executions(media_id);
CREATE INDEX IF NOT EXISTS idx_workflow_executions_status ON workflow_executions(status);
CREATE INDEX IF NOT EXISTS idx_workflow_executions_task_ids ON workflow_executions USING GIN(task_ids);
