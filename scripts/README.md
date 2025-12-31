# Mindia Scripts

This directory contains utility scripts for managing and deploying Mindia.

## Available Scripts

### Development & Setup

#### `generate-encryption-key.sh`

Generate a secure encryption key for the `ENCRYPTION_KEY` environment variable.

**Usage:**
```bash
./scripts/generate-encryption-key.sh
```

**Purpose:**
- Generates a secure 32-byte base64-encoded encryption key
- Used for encrypting sensitive plugin configuration data (API keys, secrets, tokens)
- Output should be added to your `.env` file

**Makefile Target:**
```bash
make generate-encryption-key
```

---

#### `prepare-sqlx.sh`

Prepare SQLx query cache for offline builds.

**Usage:**
```bash
export DATABASE_URL='postgresql://user:password@localhost:5432/mindia'
./scripts/prepare-sqlx.sh
```

**Purpose:**
- Generates SQLx query cache files (`.sqlx/` directory)
- Allows `cargo check` and `cargo build` to work without a database connection
- Required for CI/CD pipelines and offline development

**Requirements:**
- `DATABASE_URL` environment variable must be set
- Database must be accessible and have migrations applied

**Makefile Target:**
```bash
make prepare-sqlx
```

---

### Database Management

#### `clear-media-tasks.sh`

Clear media and tasks from the database (for development/testing).

**Usage:**
```bash
# Interactive (with confirmation)
./scripts/clear-media-tasks.sh

# Clear only tasks
./scripts/clear-media-tasks.sh --tasks-only

# Clear only media
./scripts/clear-media-tasks.sh --media-only

# Also delete files from storage directory
./scripts/clear-media-tasks.sh --storage

# Skip confirmation prompt
./scripts/clear-media-tasks.sh --yes
```

**Purpose:**
- Clears media records (images, videos, audio, documents)
- Clears task records (processing tasks, webhooks)
- Optionally deletes physical files from `storage/media/`
- Useful for resetting development/test environments

**⚠️ Warning:**
- This permanently deletes data!
- Only use in development/testing environments
- Always backup production data before running

**Makefile Target:**
```bash
make clear-media
```

---

### Deployment

#### `deploy-to-fly.sh`

Deploy Mindia to Fly.io with automated setup.

**Usage:**
```bash
# Interactive deployment (will prompt for database creation)
./scripts/deploy-to-fly.sh

# With environment variables (non-interactive)
DATABASE_URL='...' JWT_SECRET='...' ./scripts/deploy-to-fly.sh

# Set secrets only (don't deploy)
./scripts/deploy-to-fly.sh --no-deploy

# Use S3 storage
USE_S3=1 S3_BUCKET='...' S3_REGION='...' \
  AWS_ACCESS_KEY_ID='...' AWS_SECRET_ACCESS_KEY='...' \
  ./scripts/deploy-to-fly.sh
```

**Purpose:**
- Automates Fly.io deployment process
- Creates and attaches Fly Postgres database if needed
- Generates JWT_SECRET if not provided
- Configures storage (S3 or local volume)
- Sets all required secrets

**Prerequisites:**
- `flyctl` or `fly` CLI installed
- Logged in: `fly auth login`
- `fly.toml` configured (included in repo)

**Environment Variables:**
- `DATABASE_URL` - PostgreSQL connection string (optional, can be created)
- `JWT_SECRET` - Min 32 chars (optional, will be generated)
- `USE_S3` - Set to `1` to use S3 storage
- `S3_BUCKET`, `S3_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` - For S3 storage
- `FLY_APP_NAME` - Override app name (default: from `fly.toml`)

**Makefile Target:**
```bash
make deploy
```

---

### Version Management

#### `bump-version.sh`

Bump the workspace version, commit the change, and create a git tag.

**Usage:**
```bash
# Bump to a specific version
./scripts/bump-version.sh 0.2.0

# With 'v' prefix (automatically stripped)
./scripts/bump-version.sh v0.2.0

# Dry run (show what would change)
./scripts/bump-version.sh --dry-run 0.2.0

# Update files only (no commit)
./scripts/bump-version.sh --no-commit 0.2.0

# Update and commit, but don't create tag
./scripts/bump-version.sh --no-tag 0.2.0
```

**Purpose:**
- Updates version in root `Cargo.toml` (workspace version)
- Updates version in `fly.toml` if present
- Creates git commit with version change
- Creates git tag (e.g., `v0.2.0`)
- Optionally pushes tag to origin

**Version Format:**
- Must follow semantic versioning: `MAJOR.MINOR.PATCH`
- Pre-release suffixes supported: `1.2.3-alpha.1`, `1.2.3-rc.2`

**Makefile Target:**
```bash
make bump-version VERSION=0.2.0
```

---

## Script Conventions

### Exit Codes

All scripts follow standard exit code conventions:
- `0` - Success
- `1` - General error
- `2` - Invalid arguments

### Error Handling

All scripts use:
```bash
set -euo pipefail
```

This ensures:
- `set -e` - Exit on error
- `set -u` - Exit on undefined variable
- `set -o pipefail` - Exit on pipe failure

### Help Messages

All scripts support `--help` or `-h` flags:
```bash
./scripts/script-name.sh --help
```

## Creating New Scripts

When creating new scripts:

1. **Add shebang**: `#!/usr/bin/env bash`
2. **Add error handling**: `set -euo pipefail`
3. **Add usage function**: Include help message
4. **Make executable**: `chmod +x scripts/your-script.sh`
5. **Add to Makefile**: Create a make target
6. **Update this README**: Document the new script

## Makefile Integration

All scripts are integrated with the Makefile for easier use:

```bash
# View all available make targets
make help

# Common commands
make generate-encryption-key  # Generate encryption key
make prepare-sqlx             # Prepare SQLx offline mode
make clear-media              # Clear media and tasks
make deploy                   # Deploy to Fly.io
make bump-version VERSION=x.y.z  # Bump version
```

## Best Practices

1. **Always backup** before running destructive operations
2. **Test in development** before running in production
3. **Review output** - scripts provide detailed feedback
4. **Use make targets** - easier than remembering script paths
5. **Set environment variables** - required for database operations

## Troubleshooting

### Permission Denied

```bash
# Make script executable
chmod +x scripts/script-name.sh
```

### Database Connection Failed

```bash
# Verify DATABASE_URL is set
echo $DATABASE_URL

# Test database connection
psql "$DATABASE_URL" -c "SELECT 1"
```

### Command Not Found

```bash
# Check if required tools are installed
make check-deps

# Install missing dependencies
make install-deps
```

## Contributing

When adding new scripts:
- Follow existing conventions
- Include comprehensive error handling
- Add help messages
- Update this README
- Add Makefile target
- Test thoroughly

## Related Documentation

- [Production Runbook](../doc/developer/production-runbook.md)
- [Database Migrations](../doc/developer/database-migrations.md)
- [Backup & Recovery](../doc/developer/backup-recovery.md)
