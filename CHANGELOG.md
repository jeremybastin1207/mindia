# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once stable releases begin.

## [Unreleased]

### Added

- **Integration tests under mindia-api**: New integration tests in `crates/mindia-api/tests/` that build the refactored `AppState` (DbState, MediaConfig, SecurityConfig, TaskState, WebhookState, optional PluginState/WorkflowState), use `mindia_core::Config`, and call `setup_routes`. Run with `cargo test -p mindia-api --tests` (requires Docker for Postgres). See [tests/README.md](tests/README.md).
- **Request timeout middleware**: Global request timeout layer (default 60s, configurable via `REQUEST_TIMEOUT_SECS`) to avoid long-running requests holding connections.
- **CloudWatch alarms (Terraform)**: Alarms for ALB 5XX count, ALB target response time, RDS CPU utilization, and RDS database connections. Optional `alarm_sns_topic_arn` variable for notifications. See `.deployment/cloudwatch_alarms.tf`.
- **Migration naming**: `migrations/README.md` documents a single ordered naming scheme (zero-padded 12-digit prefix) for new migrations.
- **Error handling**: Preferred handler pattern documented in `error.rs`: `Result<impl IntoResponse, HttpAppError>`. Handlers for tasks, transform, batch media, video stream, image download, and media get now return `HttpAppError` consistently.
- **Deep health endpoint**: `GET /health/deep` returns the same checks as `/health` plus webhooks table connectivity. Documented under *Health endpoints* in [docs/configuration.md](docs/configuration.md).

- **Schema migrations split by feature**: The single `00000000000000_complete_schema.sql` has been replaced by 12 feature-based migrations (tenants/storage, folders/media, uploads, analytics, semantic search, tasks, webhooks, API keys, file groups, plugins, named transformations, triggers/RLS/seed). Enum and table creation are idempotent where applicable. See `migrations/` and [Installation](docs/installation.md).
- **Workflows**: Automated processing pipelines that run on upload or on demand. Define ordered steps (plugins), filter by media type/folder/content type/metadata, and optionally stop on first failure. New endpoints: `POST/GET/PUT/DELETE /api/v0/workflows`, `POST /api/v0/workflows/:id/trigger/:media_id`, `GET /api/v0/workflows/:id/executions`, `GET /api/v0/workflow-executions/:id`. Webhook events `workflow.completed` and `workflow.failed`. Enable with the `workflow` Cargo feature. See [docs/workflows.md](docs/workflows.md).
- **AWS deployment**: Terraform and Ansible in `.deployment/` for deploying Mindia to AWS (EC2 app tier, RDS PostgreSQL with pgvector, S3, ALB, CloudFront CDN, ClamAV, Secrets Manager). See [docs/deployment-aws.md](docs/deployment-aws.md).
- Initial open source hardening:
  - Added `SECURITY.md` with vulnerability disclosure process.
  - Added `CODE_OF_CONDUCT.md` based on the Contributor Covenant.
  - Introduced this `CHANGELOG.md`.
  - Prepared GitHub templates and automation (issues, PRs, Dependabot, release workflow).
  - Added examples directory and wired integration tests into CI (see repository for details).

### Changed

- **Unified media state**: All handlers and job/task code now use `state.media.repository` for images and videos (replacing former `state.image`, `state.video`, and `state.video_db`). No API or config changes.

