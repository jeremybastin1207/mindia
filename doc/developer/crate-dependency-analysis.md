# Crate Dependency Analysis

**Last Updated**: January 30, 2026

## Current Dependency Graph

```
mindia-core (no mindia crate dependencies)
    ↓
    ├── mindia-db (depends on: core)
    │   └── mindia-storage (depends on: core)
    │
    ├── mindia-services (depends on: core)
    │
    ├── mindia-processing (depends on: core)
    │
    ├── mindia-infra (depends on: core, db[optional], storage[optional])
    │
    ├── mindia-worker (depends on: core, db, infra)
    │
    ├── mindia-plugins (depends on: core, db, storage)
    │
    ├── mindia-api (depends on: ALL of the above)
    │
    ├── mindia-cli (depends on: core only)
    │
    └── mindia-mcp (depends on: core)
```

**Notes**:
- `mindia-messaging` has been **merged into mindia-core** (messaging_types.rs)
- `mindia-control-plane` **does not exist** (future planned feature)
- `mindia-media-processor` **does not exist** (consolidated into mindia-api)

## Dependency Details

### mindia-core
- **Dependencies on other mindia crates**: None
- **Status**: ✅ Clean - foundational crate with minimal external dependencies
- **Contains**: Models, config, error types, messaging types (merged from mindia-messaging)
- **Used by**: All other crates

### mindia-db
- **Dependencies**: `mindia-core` (with sqlx feature)
- **Status**: ✅ Clean - proper layering
- **Contains**: Repository implementations for all database operations
- **Pattern**: Repository pattern with tenant isolation

### mindia-storage
- **Dependencies**: `mindia-core`
- **Status**: ✅ Clean - pure storage abstraction
- **Contains**: Storage trait and implementations (S3, local filesystem)

### mindia-services
- **Dependencies**: `mindia-core`
- **Status**: ✅ Focused on external service integrations
- **Contains**: S3 client, Anthropic (Claude) client, ClamAV client, FFmpeg service, Audio service

### mindia-processing
- **Dependencies**: `mindia-core`
- **Status**: ✅ Clean - media processing logic
- **Contains**: Image, video, audio, document processing, transformations, validation

### mindia-infra
- **Dependencies**: `mindia-core`, `mindia-db` (optional), `mindia-storage` (optional)
- **Status**: ✅ Well-organized with feature flags
- **Contains**: Middleware, telemetry, webhooks, analytics, rate limiting, cleanup, capacity, archive

### mindia-worker
- **Dependencies**: `mindia-core`, `mindia-db`, `mindia-infra`
- **Status**: ✅ Clean - focused on task queue
- **Contains**: Task queue, worker pool, LISTEN/NOTIFY, retry logic

### mindia-plugins
- **Dependencies**: `mindia-core`, `mindia-db`, `mindia-storage`
- **Status**: ✅ Clean dependencies
- **Contains**: Plugin registry, AssemblyAI, AWS Rekognition, Google Vision, AWS Transcribe

### mindia-api
- **Dependencies**: All crates listed above
- **Status**: ✅ Main application binary
- **Binary name**: `mindia-api`
- **Contains**: HTTP handlers, authentication, task handlers, business logic

### mindia-cli
- **Dependencies**: `mindia-core` only
- **Status**: ✅ Clean - minimal dependencies
- **Contains**: CLI tools for listing media and stats

### mindia-mcp
- **Dependencies**: `mindia-core`
- **Status**: ✅ Clean - MCP server
- **Contains**: Model Context Protocol server implementation

## Circular Dependency Analysis

**Result**: ✅ No circular dependencies detected

The codebase follows a clean dependency hierarchy:
- `mindia-core` is the foundation with **zero** dependencies on other mindia crates
- All other crates depend on core, creating a clear tree structure
- No cycles exist

## Architecture Status

### ✅ Completed Improvements (2025-2026)

1. **✅ Messaging Consolidation**: `mindia-messaging` merged into `mindia-core/src/messaging_types.rs`
2. **✅ Storage Extraction**: `mindia-storage` created as separate crate
3. **✅ Processing Extraction**: `mindia-processing` created (formerly mindia-media-processing)
4. **✅ Infrastructure Crate**: `mindia-infra` with feature flags for optional components
5. **✅ Worker Queue**: `mindia-worker` created for background task processing
6. **✅ Service Consolidation**: Single `mindia-api` service (no separate control-plane or media-processor)

### Current State

The dependency structure is **clean and well-organized**:
- ✅ Clear separation of concerns
- ✅ No circular dependencies
- ✅ Feature flags for optional components
- ✅ Focused, single-responsibility crates
- ✅ Fast compilation with parallel builds

## Compilation Dependencies

**Build order** (based on dependency graph):
1. `mindia-core` (no deps)
2. `mindia-storage`, `mindia-services`, `mindia-db`, `mindia-processing` (parallel - all depend only on core)
3. `mindia-infra`, `mindia-plugins`, `mindia-worker` (parallel - second layer)
4. `mindia-api`, `mindia-cli`, `mindia-mcp` (final layer)

**Benefits**:
- Parallel compilation of independent crates
- Changes to leaf crates (cli, mcp) don't rebuild core
- Infrastructure updates don't require full rebuild

## Feature Flags

### mindia-api Features:
- `video` - Video upload and transcoding
- `audio` - Audio file support
- `document` - Document upload
- `plugin` - Plugin system
- `clamav` - ClamAV virus scanning
- `semantic-search` - Anthropic embeddings and pgvector
- `content-moderation` - AWS Rekognition moderation
- `observability-opentelemetry` - OpenTelemetry (default)

### mindia-infra Features:
- `middleware`, `webhook`, `analytics`, `rate-limit`, `cleanup`, `capacity`, `archive`
- `observability-opentelemetry` vs `observability-basic`

## Historical Notes

See [`../historical/`](../historical/README.md) for documentation about past refactorings and the evolution of this architecture.
