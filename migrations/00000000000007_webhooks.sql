-- Webhooks (event types, delivery, retry queue)

DO $$ BEGIN
    CREATE TYPE webhook_event_type AS ENUM (
        'file.uploaded', 'file.deleted', 'file.stored',
        'file.processing_completed', 'file.processing_failed'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;
DO $$ BEGIN
    CREATE TYPE webhook_delivery_status AS ENUM ('pending', 'success', 'failed', 'retrying');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

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

CREATE INDEX IF NOT EXISTS idx_webhooks_tenant_event_active
    ON webhooks(tenant_id, event_type, is_active)
    WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_webhooks_tenant_id ON webhooks(tenant_id);
CREATE INDEX IF NOT EXISTS idx_webhooks_event_type ON webhooks(event_type);
CREATE INDEX IF NOT EXISTS idx_webhooks_active ON webhooks(is_active) WHERE is_active = true;

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

CREATE INDEX IF NOT EXISTS idx_webhook_events_webhook_id ON webhook_events(webhook_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_webhook_events_tenant_id ON webhook_events(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_webhook_events_status ON webhook_events(status);
CREATE INDEX IF NOT EXISTS idx_webhook_events_failed_retry
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

CREATE INDEX IF NOT EXISTS idx_webhook_retry_queue_next_retry ON webhook_retry_queue(next_retry_at);
CREATE INDEX IF NOT EXISTS idx_webhook_retry_queue_event ON webhook_retry_queue(webhook_event_id);
CREATE INDEX IF NOT EXISTS idx_webhook_retry_queue_webhook ON webhook_retry_queue(webhook_id);
CREATE INDEX IF NOT EXISTS idx_webhook_retry_queue_tenant ON webhook_retry_queue(tenant_id);
