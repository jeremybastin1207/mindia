# Backup & Recovery Procedures

Comprehensive guide for backing up and recovering Mindia data and configuration.

## Overview

Mindia requires backups for:
- **Database**: PostgreSQL data (images, videos, metadata, users, etc.)
- **Storage**: Files stored in S3 or local filesystem
- **Configuration**: Environment variables and secrets
- **Migrations**: Database schema versions

## Recovery Objectives

- **RTO (Recovery Time Objective)**: 1 hour
- **RPO (Recovery Point Objective)**: 24 hours (daily backups)

## Database Backups

### Automated Backups

#### Neon PostgreSQL

Neon provides automatic backups:
- Point-in-time recovery (PITR) available
- Daily snapshots retained for 7 days
- Manual backup creation available

#### Self-Hosted PostgreSQL

Use `pg_dump` for backups:

```bash
# Full database backup
pg_dump -h $DB_HOST -U $DB_USER -d mindia \
  --format=custom \
  --file=backup-$(date +%Y%m%d-%H%M%S).dump

# Compressed backup
pg_dump -h $DB_HOST -U $DB_USER -d mindia \
  --format=custom \
  --compress=9 \
  --file=backup-$(date +%Y%m%d-%H%M%S).dump.gz
```

### Backup Script Example

To create a backup script, you can use `scripts/backup-database.sh`:

```bash
#!/bin/bash
set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-./backups}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="$BACKUP_DIR/mindia-$TIMESTAMP.dump"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Perform backup
echo "Creating database backup..."
pg_dump "$DATABASE_URL" \
  --format=custom \
  --file="$BACKUP_FILE"

# Compress backup
echo "Compressing backup..."
gzip "$BACKUP_FILE"
BACKUP_FILE="${BACKUP_FILE}.gz"

# Verify backup
echo "Verifying backup..."
if [ -f "$BACKUP_FILE" ] && [ -s "$BACKUP_FILE" ]; then
    echo "✓ Backup created successfully: $BACKUP_FILE"
    
    # Get backup size
    SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
    echo "  Size: $SIZE"
    
    # Cleanup old backups (keep last 30 days)
    find "$BACKUP_DIR" -name "mindia-*.dump.gz" -mtime +30 -delete
    echo "✓ Cleaned up backups older than 30 days"
else
    echo "✗ Backup verification failed"
    exit 1
fi
```

### Scheduled Backups

#### Cron Job

```bash
# Add to crontab (daily at 2 AM)
# Note: Create scripts/backup-database.sh first (see example above)
0 2 * * * /path/to/scripts/backup-database.sh >> /var/log/mindia/backup.log 2>&1
```

#### Systemd Timer

Create `/etc/systemd/system/mindia-backup.service`:

```ini
[Unit]
Description=Mindia Database Backup
After=network.target

[Service]
Type=oneshot
Environment="DATABASE_URL=postgresql://..."
Environment="BACKUP_DIR=/var/backups/mindia"
ExecStart=/usr/local/bin/mindia-backup.sh
```

Create `/etc/systemd/system/mindia-backup.timer`:

```ini
[Unit]
Description=Daily Mindia Database Backup

[Timer]
OnCalendar=daily
OnCalendar=02:00
Persistent=true

[Install]
WantedBy=timers.target
```

Enable timer:
```bash
sudo systemctl enable mindia-backup.timer
sudo systemctl start mindia-backup.timer
```

## Storage Backups

### S3 Backups

#### Versioning

Enable S3 versioning for automatic backups:

```bash
aws s3api put-bucket-versioning \
  --bucket your-bucket-name \
  --versioning-configuration Status=Enabled
```

#### Cross-Region Replication

Set up cross-region replication for disaster recovery:

```bash
aws s3api put-bucket-replication \
  --bucket your-bucket-name \
  --replication-configuration file://replication-config.json
```

#### Manual Backup

```bash
# Sync to backup bucket
aws s3 sync s3://your-bucket-name s3://your-backup-bucket-name \
  --storage-class GLACIER
```

### Local Storage Backups

```bash
# Backup local storage
tar -czf storage-backup-$(date +%Y%m%d).tar.gz \
  /var/lib/mindia/storage

# Copy to remote location
rsync -av storage-backup-*.tar.gz \
  backup-server:/backups/mindia/
```

## Configuration Backups

### Environment Variables

```bash
# Export environment variables (without secrets)
env | grep -E '^(DATABASE_URL|S3_BUCKET|PORT|ENVIRONMENT)=' \
  > config-backup-$(date +%Y%m%d).env

# Store securely (encrypted)
gpg --encrypt --recipient backup@example.com \
  config-backup-*.env
```

### Secrets Backup

**Important**: Never commit secrets to version control.

Use a secrets manager:
- AWS Secrets Manager
- HashiCorp Vault
- Kubernetes Secrets

## Recovery Procedures

### Database Recovery

#### Full Database Restore

```bash
# Stop application
systemctl stop mindia

# Restore from backup
pg_restore -h $DB_HOST -U $DB_USER -d mindia \
  --clean --if-exists \
  backup-20240101-020000.dump

# Verify restoration
psql $DATABASE_URL -c "SELECT COUNT(*) FROM images;"

# Start application
systemctl start mindia
```

#### Point-in-Time Recovery (Neon)

```bash
# Create new branch from point in time
neonctl branches create \
  --project-id $PROJECT_ID \
  --name recovery-branch \
  --timestamp "2024-01-01T12:00:00Z"

# Update DATABASE_URL to use recovery branch
# Verify data
# Merge or switch if recovery successful
```

#### Partial Recovery

```bash
# Restore specific table
pg_restore -h $DB_HOST -U $DB_USER -d mindia \
  --table=images \
  --data-only \
  backup-20240101-020000.dump
```

### Storage Recovery

#### S3 Recovery

```bash
# Restore from version
aws s3api restore-object \
  --bucket your-bucket-name \
  --key path/to/file.jpg \
  --version-id VERSION_ID

# Restore from backup bucket
aws s3 sync s3://your-backup-bucket-name s3://your-bucket-name
```

#### Local Storage Recovery

```bash
# Extract backup
tar -xzf storage-backup-20240101.tar.gz -C /tmp

# Restore files
rsync -av /tmp/var/lib/mindia/storage/ \
  /var/lib/mindia/storage/
```

## Backup Verification

### Automated Verification

Create `scripts/verify-backup.sh`:

```bash
#!/bin/bash
set -euo pipefail

BACKUP_FILE="$1"

# Verify backup file exists and is not empty
if [ ! -f "$BACKUP_FILE" ] || [ ! -s "$BACKUP_FILE" ]; then
    echo "✗ Backup file is missing or empty"
    exit 1
fi

# Test restore to temporary database
TEMP_DB="mindia_backup_test_$(date +%s)"
createdb "$TEMP_DB"

# Attempt restore
if pg_restore -d "$TEMP_DB" "$BACKUP_FILE" 2>&1; then
    echo "✓ Backup is valid"
    
    # Verify data
    RECORD_COUNT=$(psql -d "$TEMP_DB" -t -c "SELECT COUNT(*) FROM images;")
    echo "  Images in backup: $RECORD_COUNT"
    
    # Cleanup
    dropdb "$TEMP_DB"
    exit 0
else
    echo "✗ Backup restore failed"
    dropdb "$TEMP_DB" 2>/dev/null || true
    exit 1
fi
```

### Manual Verification

```bash
# List backup contents
pg_restore --list backup.dump

# Verify backup integrity
pg_restore --schema-only backup.dump | head -20
```

## Disaster Recovery Plan

### Scenario 1: Database Corruption

1. **Detect**: Monitor for database errors
2. **Isolate**: Stop application writes
3. **Assess**: Determine corruption scope
4. **Recover**: Restore from most recent backup
5. **Verify**: Run integrity checks
6. **Resume**: Restart application

### Scenario 2: Storage Loss

1. **Detect**: Monitor storage errors
2. **Assess**: Determine affected files
3. **Recover**: Restore from backup/versioning
4. **Verify**: Check file integrity
5. **Resume**: Application continues normally

### Scenario 3: Complete System Failure

1. **Assess**: Determine recovery point
2. **Provision**: Set up new infrastructure
3. **Restore Database**: From backup
4. **Restore Storage**: From backup/replication
5. **Restore Configuration**: From secrets manager
6. **Verify**: Run health checks
7. **Resume**: Application operational

## Backup Retention Policy

- **Daily backups**: Retained for 30 days
- **Weekly backups**: Retained for 12 weeks
- **Monthly backups**: Retained for 12 months
- **Yearly backups**: Retained indefinitely

## Testing Recovery

### Regular Testing

Test recovery procedures quarterly:

1. Create test environment
2. Restore from backup
3. Verify data integrity
4. Test application functionality
5. Document any issues
6. Update procedures if needed

### Recovery Test Checklist

- [ ] Database restore successful
- [ ] All tables present
- [ ] Data integrity verified
- [ ] Storage files accessible
- [ ] Application starts successfully
- [ ] Health checks passing
- [ ] Functional tests passing

## Monitoring

### Backup Monitoring

Monitor backup success:

```bash
# Check backup logs
tail -f /var/log/mindia/backup.log

# Verify latest backup
ls -lh /var/backups/mindia/

# Check backup age
find /var/backups/mindia -name "*.dump.gz" -mtime +1
```

### Alerting

Set up alerts for:
- Backup failures
- Backup age > 25 hours
- Backup size anomalies
- Storage quota warnings

## Best Practices

1. **Automate**: Use cron/systemd for scheduled backups
2. **Verify**: Regularly test backup restoration
3. **Encrypt**: Encrypt backups at rest
4. **Offsite**: Store backups in different region
5. **Document**: Keep recovery procedures updated
6. **Test**: Regularly test disaster recovery
7. **Monitor**: Alert on backup failures
8. **Retain**: Follow retention policy

## Tools

- **pg_dump/pg_restore**: PostgreSQL backup/restore
- **aws s3 sync**: S3 backup
- **rsync**: Local storage backup
- **tar/gzip**: Compression
- **gpg**: Encryption

## Next Steps

- [ ] Set up automated backups
- [ ] Test recovery procedures
- [ ] Document recovery times
- [ ] Set up monitoring
- [ ] Schedule regular testing

