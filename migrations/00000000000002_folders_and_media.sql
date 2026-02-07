-- Folders and media (hierarchical folders, unified media table)

CREATE TABLE IF NOT EXISTS folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    parent_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, parent_id, name)
);

CREATE INDEX IF NOT EXISTS idx_folders_parent_id ON folders(parent_id);
CREATE INDEX IF NOT EXISTS idx_folders_tenant_parent ON folders(tenant_id, parent_id);
CREATE INDEX IF NOT EXISTS idx_folders_created_at ON folders(created_at DESC);

DO $$ BEGIN
    CREATE TYPE media_type AS ENUM ('image', 'video', 'audio', 'document');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;
DO $$ BEGIN
    CREATE TYPE processing_status AS ENUM ('pending', 'processing', 'completed', 'failed');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

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

CREATE INDEX IF NOT EXISTS idx_media_tenant_type ON media(tenant_id, media_type);
CREATE INDEX IF NOT EXISTS idx_media_tenant_uploaded ON media(tenant_id, uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_media_tenant_type_uploaded ON media(tenant_id, media_type, uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_media_storage_id ON media(storage_id);
CREATE INDEX IF NOT EXISTS idx_media_uploaded_at ON media(uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_media_filename ON media(filename);
CREATE INDEX IF NOT EXISTS idx_media_original_filename ON media(original_filename);
CREATE INDEX IF NOT EXISTS idx_media_expires_at ON media(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_store_permanently ON media(store_permanently, expires_at);
CREATE INDEX IF NOT EXISTS idx_media_folder_id ON media(folder_id) WHERE folder_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_tenant_folder ON media(tenant_id, folder_id);
CREATE INDEX IF NOT EXISTS idx_media_tenant_type_folder ON media(tenant_id, media_type, folder_id);
CREATE INDEX IF NOT EXISTS idx_media_metadata ON media USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_media_metadata_user ON media USING GIN ((metadata->'user'));
CREATE INDEX IF NOT EXISTS idx_media_type_metadata ON media USING GIN (type_metadata);
CREATE INDEX IF NOT EXISTS idx_media_type_metadata_processing ON media((type_metadata->>'processing_status')) WHERE type_metadata ? 'processing_status';
