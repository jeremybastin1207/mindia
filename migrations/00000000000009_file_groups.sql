-- File groups (group media items)

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

CREATE INDEX IF NOT EXISTS idx_file_groups_tenant_id ON file_groups(tenant_id);
CREATE INDEX IF NOT EXISTS idx_file_groups_created_at ON file_groups(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_file_group_items_group_id ON file_group_items(group_id);
CREATE INDEX IF NOT EXISTS idx_file_group_items_media_id ON file_group_items(media_id);
CREATE INDEX IF NOT EXISTS idx_file_group_items_group_index ON file_group_items(group_id, index);
