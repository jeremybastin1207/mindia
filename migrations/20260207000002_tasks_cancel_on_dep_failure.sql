-- Tasks: allow cancelling when a dependency fails (used by workflow chains)

ALTER TABLE tasks ADD COLUMN IF NOT EXISTS cancel_on_dep_failure BOOLEAN NOT NULL DEFAULT false;
