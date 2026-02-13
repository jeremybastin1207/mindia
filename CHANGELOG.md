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

- **Code review (security and correctness)**:
  - **Transactions**: Upload handlers (image, video, audio, document) now create media records inside a DB transaction; workflow service creates execution after all step tasks are submitted and persists in a single transaction to avoid partial state.
  - **Metadata search**: Range and text_contains filters now use parameterized `jsonb_each` (key bound as parameter) instead of interpolating keys into SQL.
  - **SSRF**: DNS resolution failure in SSRF validation now fails closed (rejects request) in both API and webhook code paths.
  - **CSRF**: In production, missing `CSRF_SECRET` (and `JWT_SECRET` fallback) now returns 503 instead of using an insecure default.
  - **S3 upload_stream**: Uses multipart upload with 8 MiB parts instead of loading the entire stream into memory.
  - **Config**: Added validation for PORT (1–65535), HTTP_RATE_LIMIT_PER_MINUTE (1–100000), semantic search (requires ANTHROPIC_API_KEY or VOYAGE_API_KEY when enabled). Added `CONTENT_MODERATION_ENABLED` to config and upload service (no longer read from env only).
  - **Chunked upload**: Rejects `chunk_size == 0` to avoid panic; cleanup delete failures are logged as warnings; filename sanitization returns an error on path traversal instead of a generic name.
  - **Webhook retry**: Initial delivery failure now uses `event.retry_count` for backoff when scheduling retry.
  - **Analytics handlers**: Errors are converted via `HttpAppError::from` to preserve context.
  - **Local storage**: Path-under-base validation uses `strip_prefix` where applicable.
  - **Auth**: Per-IP rate limiting for failed auth attempts (10 failures per 15 minutes, then 429 Too Many Requests).
- **Unified media state**: All handlers and job/task code now use `state.media.repository` for images and videos (replacing former `state.image`, `state.video`, and `state.video_db`). No API or config changes.
- **Upload media (security and consistency)**:
  - **Content-Type validation**: Multipart uploads now validate MIME type by exact match (normalized, no parameters) instead of substring, preventing allowlist bypass (e.g. `x/image/jpeg`).
  - **Chunked completion**: Actual file size is verified against the declared size; DB stores the actual size. Chunked completion runs ClamAV when enabled (same as multipart).
  - **Chunked uploads**: Max file size per media type and max chunk count (10,000) are enforced at session start. Completion rejects assembled size exceeding declared size.
  - **Multipart**: Only one field named `file` is accepted; requests with multiple file fields are rejected.
  - **Audit**: File upload audit log now includes `user_id` and `client_ip` when available (set by auth middleware and passed from upload handlers).
  - **Workflow**: Fixed `notify_upload` workflow block using correct parameters (`_folder_id`, `_metadata`) when the `workflow` feature is enabled.
  - **Storage**: New `content_length` method on the Storage trait for size verification. Chunked completion documents in-code that the assembled file is buffered in memory (bounded by per-type limits).
- **Code review follow-ups**:
  - **Security**: Fixed comment typo in SSRF validation ("i6ternal" → "internal"). Confirmed no logging of MASTER_API_KEY or database URLs.
  - **Documentation**: Added *Operational security* subsection in [docs/configuration.md](docs/configuration.md) (MASTER_API_KEY, CORS, TRUSTED_PROXY_COUNT, URL_UPLOAD_ALLOWLIST, CLAMAV_FAIL_CLOSED).
  - **Analytics handlers**: All analytics endpoints now return `Result<_, HttpAppError>` for consistent error handling and logging.
  - **Routes**: Route registration split into `setup/routes/` (mod.rs, domains.rs, health.rs) for readability.
  - **Health checks**: Shared `run_check` helper in `routes/health.rs` to deduplicate timeout-and-status logic used by `/health` and `/health/deep`.
  - **Dead code**: Justified `#[allow(dead_code)]` on app sub-state types (DbState, TaskState, WebhookState, PluginState) and compile-time assertion in handlers.
- **Search API**: Query parameter `q` limited to 16 KB. Metadata filter keys validated with `validate_metadata_key` at the API boundary; filter count (max 10) validated in the handler. Query embeddings normalized to DB dimension before use. OpenAPI and [docs/semantic-search.md](docs/semantic-search.md) document pagination and `min_similarity` (count is after filtering; offset applies before filtering).

### Removed

- **Presigned upload API**: Endpoints `POST /api/v0/uploads/presigned` and `POST /api/v0/uploads/complete` have been removed. Direct-to-S3 single-file uploads bypass server-side validation (size, virus scan, content-type) and are not secure. Use multipart upload (`POST /api/v0/images`, etc.) or chunked upload (`/api/v0/uploads/chunked/*`) instead.

