-- Named transformations (reusable presets)

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

CREATE INDEX IF NOT EXISTS idx_named_transformations_tenant_id ON named_transformations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_named_transformations_tenant_name ON named_transformations(tenant_id, name);
