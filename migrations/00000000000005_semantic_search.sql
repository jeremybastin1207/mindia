-- Semantic search (pgvector embeddings)

CREATE EXTENSION IF NOT EXISTS vector;
DO $$ BEGIN
    CREATE TYPE entity_type AS ENUM ('image', 'video', 'audio', 'document');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

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

CREATE INDEX IF NOT EXISTS idx_embeddings_tenant_id ON embeddings(tenant_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_tenant_entity ON embeddings(tenant_id, entity_id, entity_type);
CREATE INDEX IF NOT EXISTS idx_embeddings_entity_id ON embeddings(entity_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_vector ON embeddings USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
