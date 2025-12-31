# Database Migration Strategy

Guide for managing database migrations in Mindia with rollback procedures and best practices.

## Overview

Mindia uses SQLx migrations located in `migrations/` directory. Migrations run automatically on application startup.

## Migration Files

### Naming Convention

```
YYYYMMDDHHMMSS_description.sql
```

Example: `20240101120000_add_webhooks_table.sql`

### Current Migration

- `00000000000000_complete_schema.sql` - Initial schema

## Running Migrations

### Automatic (Default)

Migrations run automatically on application startup:

```rust
sqlx::migrate!("./migrations").run(&pool).await?;
```

### Manual

```bash
# Using sqlx-cli
sqlx migrate run

# Verify migrations
sqlx migrate info
```

## Creating Migrations

### New Migration

```bash
# Create new migration file
sqlx migrate add description_of_change
```

### Migration Template

```sql
-- Migration: Add new column to images table
-- Created: 2024-01-01

-- Up migration
ALTER TABLE images ADD COLUMN new_field VARCHAR(255);

-- Down migration (rollback)
ALTER TABLE images DROP COLUMN new_field;
```

## Migration Best Practices

### 1. Always Include Rollback

Every migration should be reversible:

```sql
-- Up
ALTER TABLE images ADD COLUMN metadata JSONB;

-- Down
ALTER TABLE images DROP COLUMN metadata;
```

### 2. Test Migrations

```bash
# Test up migration
sqlx migrate run

# Test down migration
sqlx migrate revert

# Test again
sqlx migrate run
```

### 3. Backup Before Migration

```bash
# Create backup
pg_dump "$DATABASE_URL" --format=custom --file="backup-$(date +%Y%m%d-%H%M%S).dump"

# Run migration
sqlx migrate run
```

### 4. Idempotent Migrations

Make migrations safe to run multiple times:

```sql
-- Good: Check if column exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'images' AND column_name = 'new_field'
    ) THEN
        ALTER TABLE images ADD COLUMN new_field VARCHAR(255);
    END IF;
END $$;
```

### 5. Large Migrations

For large data migrations:
- Run during maintenance window
- Use transactions
- Add progress logging
- Consider batch processing

## Rollback Procedures

### Automatic Rollback

SQLx supports rollback with down migrations:

```bash
# Revert last migration
sqlx migrate revert
```

### Manual Rollback

If automatic rollback fails:

1. **Stop application**
2. **Restore from backup**
3. **Verify data integrity**
4. **Fix migration issue**
5. **Re-apply migration**

### Rollback Script

```bash
#!/bin/bash
# rollback-migration.sh

set -euo pipefail

echo "Rolling back database migration..."

# Create backup before rollback
pg_dump "$DATABASE_URL" --format=custom --file="backup-$(date +%Y%m%d-%H%M%S).dump"

# Revert migration
sqlx migrate revert

# Verify
sqlx migrate info

echo "Rollback complete!"
```

## Migration Testing

### Test Environment

1. **Create test database**
2. **Run migrations**
3. **Test application**
4. **Test rollback**
5. **Verify data integrity**

### Migration Validation

```bash
# Validate migration syntax
psql $DATABASE_URL -f migrations/00000000000000_complete_schema.sql --dry-run

# Check for syntax errors
sqlx migrate info
```

## Production Migration Process

### Pre-Migration Checklist

- [ ] Migration tested in staging
- [ ] Backup created
- [ ] Rollback plan documented
- [ ] Maintenance window scheduled
- [ ] Team notified

### Migration Steps

1. **Create backup**
   ```bash
   pg_dump "$DATABASE_URL" --format=custom --file="backup-$(date +%Y%m%d-%H%M%S).dump"
   ```

2. **Verify backup**
   ```bash
   # Check backup file exists and has size
   ls -lh backup-*.dump
   # Test backup by listing contents
   pg_restore --list backup-*.dump | head -20
   ```

3. **Run migration** (automatic on startup, or manual)
   ```bash
   sqlx migrate run
   ```

4. **Verify migration**
   ```bash
   sqlx migrate info
   psql $DATABASE_URL -c "SELECT * FROM images LIMIT 1;"
   ```

5. **Monitor application**
   - Check health endpoint
   - Monitor error logs
   - Verify functionality

### Post-Migration

- [ ] Verify application functionality
- [ ] Check error logs
- [ ] Monitor performance
- [ ] Update documentation
- [ ] Archive migration logs

## Migration Versioning

### Version Tracking

SQLx tracks migrations in `_sqlx_migrations` table:

```sql
SELECT * FROM _sqlx_migrations ORDER BY installed_on DESC;
```

### Version Verification

```rust
// Verify migration version in application
let version = sqlx::migrate!("./migrations")
    .version()
    .await?;
```

## Common Migration Patterns

### Adding Column

```sql
-- Up
ALTER TABLE images ADD COLUMN description TEXT;

-- Down
ALTER TABLE images DROP COLUMN description;
```

### Adding Index

```sql
-- Up
CREATE INDEX idx_images_tenant_id ON images(tenant_id);

-- Down
DROP INDEX idx_images_tenant_id;
```

### Adding Table

```sql
-- Up
CREATE TABLE webhooks (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Down
DROP TABLE webhooks;
```

### Data Migration

```sql
-- Up
UPDATE images SET new_field = old_field WHERE new_field IS NULL;

-- Down
-- Note: Data migrations may not be fully reversible
-- Consider keeping old data or documenting data loss
```

## Troubleshooting

### Migration Fails

1. **Check error message**
2. **Verify database connection**
3. **Check migration syntax**
4. **Review migration order**
5. **Restore from backup if needed**

### Migration Already Applied

SQLx skips already-applied migrations automatically.

### Migration Out of Order

Migrations are applied in filename order. Ensure proper naming.

## Best Practices

1. **Always backup** before migration
2. **Test in staging** first
3. **Include rollback** in every migration
4. **Use transactions** when possible
5. **Document breaking changes**
6. **Monitor after migration**
7. **Keep migrations small** when possible

## Tools

- **sqlx-cli**: Migration management
- **pg_dump/pg_restore**: Backup/restore
- **psql**: Database client
- **pgAdmin**: GUI database tool

## Next Steps

- [ ] Review existing migrations
- [ ] Set up migration testing
- [ ] Document migration procedures
- [ ] Create rollback scripts
- [ ] Schedule migration reviews

