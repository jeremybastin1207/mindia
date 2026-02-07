-- Webhooks: add workflow completion/failure event types (idempotent)

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_enum e JOIN pg_type t ON e.enumtypid = t.oid WHERE t.typname = 'webhook_event_type' AND e.enumlabel = 'workflow.completed') THEN
        ALTER TYPE webhook_event_type ADD VALUE 'workflow.completed';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_enum e JOIN pg_type t ON e.enumtypid = t.oid WHERE t.typname = 'webhook_event_type' AND e.enumlabel = 'workflow.failed') THEN
        ALTER TYPE webhook_event_type ADD VALUE 'workflow.failed';
    END IF;
END
$$;
