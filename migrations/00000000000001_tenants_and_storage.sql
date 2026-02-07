-- Tenants and storage backend (multi-tenancy, storage config, storage locations)

DO $$ BEGIN
    CREATE TYPE tenant_status AS ENUM ('active', 'suspended', 'deleted');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    status tenant_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenants_status ON tenants(status);
CREATE INDEX IF NOT EXISTS idx_tenants_name ON tenants(name);

DO $$ BEGIN
    CREATE TYPE storage_backend AS ENUM ('s3', 'local', 'nfs');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS tenant_storage_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL UNIQUE REFERENCES tenants(id) ON DELETE CASCADE,
    storage_backend storage_backend NOT NULL DEFAULT 's3',
    s3_bucket VARCHAR(255),
    s3_region VARCHAR(100) DEFAULT 'us-east-1',
    s3_endpoint VARCHAR(255),
    local_path VARCHAR(512),
    nfs_mount VARCHAR(512),
    config JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        (storage_backend = 's3' AND s3_bucket IS NOT NULL) OR
        (storage_backend = 'local' AND local_path IS NOT NULL) OR
        (storage_backend = 'nfs' AND nfs_mount IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_tenant_storage_config_tenant_id ON tenant_storage_config(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_storage_config_backend ON tenant_storage_config(storage_backend);

CREATE TABLE IF NOT EXISTS storage_locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    backend storage_backend NOT NULL,
    bucket VARCHAR(255),
    key VARCHAR(512) NOT NULL,
    url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(backend, bucket, key)
);

CREATE INDEX IF NOT EXISTS idx_storage_locations_backend_bucket_key ON storage_locations(backend, bucket, key);
CREATE INDEX IF NOT EXISTS idx_storage_locations_created_at ON storage_locations(created_at DESC);
