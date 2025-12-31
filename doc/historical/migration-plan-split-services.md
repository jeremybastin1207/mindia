# Migration Plan: Splitting mindia-services

## Overview

The `mindia-services` crate is currently a large catch-all containing:
- Media processing (image, video, audio, document)
- Storage backends (S3, local)
- Message queue implementation
- Infrastructure services (webhooks, analytics, rate limiting)
- Transformations
- Security (ClamAV, content moderation)
- AI services (Ollama)

This plan outlines how to split it into focused, maintainable crates.

## Target Structure

```
mindia-services/ (current - to be split)
    ↓
mindia-storage/ (new)
mindia-media-processing/ (new)
mindia-infrastructure/ (new)
```

## Detailed Split Plan

### 1. mindia-storage (New Crate)

**Purpose**: Storage abstraction and implementations

**Contents from mindia-services**:
- `services/storage/` (factory, s3, local)
- Storage trait (currently in `mindia-core`, should move here)

**Dependencies**:
- `mindia-core` (for error types, models)
- `aws-sdk-s3` (optional, feature-gated)
- `tokio`, `anyhow`, `tracing`

**Features**:
- `storage-s3` - S3 backend
- `storage-local` - Local filesystem backend

**Files to Move**:
```
mindia-services/src/services/storage/
    ├── mod.rs
    ├── factory.rs
    ├── s3.rs
    └── local.rs
```

**New Crate Structure**:
```
mindia-storage/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── traits.rs (move from mindia-core)
    ├── s3.rs
    ├── local.rs
    └── factory.rs
```

### 2. mindia-media-processing (New Crate)

**Purpose**: Media processing services (image, video, audio, document)

**Contents from mindia-services**:
- `services/image.rs` - Image processing
- `services/video.rs` - Video processing (FFmpeg) - if exists
- `services/audio.rs` - Audio processing
- `services/document.rs` - Document processing (if exists)
- `services/transform/` - Image transformations
- `services/compression.rs` - Compression utilities
- `services/media_validator.rs` - Media validation

**Dependencies**:
- `mindia-core`
- `mindia-storage` (for storage operations)
- `mindia-db` (for database operations)
- Image libraries (feature-gated)
- FFmpeg (external, for video)

**Features**:
- `image` - Image processing
- `video` - Video processing
- `audio` - Audio processing
- `document` - Document processing

**Files to Move**:
```
mindia-services/src/services/
    ├── image.rs
    ├── audio.rs
    ├── transform/
    ├── compression.rs
    └── media_validator.rs
```

**New Crate Structure**:
```
mindia-media-processing/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── image/
    │   ├── mod.rs
    │   └── processor.rs
    ├── video/
    │   ├── mod.rs
    │   └── ffmpeg.rs
    ├── audio/
    │   ├── mod.rs
    │   └── processor.rs
    ├── document/
    │   ├── mod.rs
    │   └── processor.rs
    ├── transform/
    │   ├── mod.rs
    │   ├── transformer.rs
    │   ├── smart_crop.rs
    │   └── watermark.rs
    ├── compression.rs
    └── validator.rs
```

### 3. mindia-infrastructure (New Crate)

**Purpose**: Infrastructure services (webhooks, analytics, rate limiting, cleanup)

**Contents from mindia-services**:
- `services/webhook.rs` - Webhook delivery
- `services/webhook_retry.rs` - Webhook retry logic
- `services/analytics.rs` - Analytics collection
- `services/rate_limiter.rs` - Rate limiting
- `services/cleanup.rs` - Cleanup services
- `services/capacity.rs` - Capacity checking
- `services/archive.rs` - Archive creation

**Also merge from mindia-infra**:
- `telemetry/` - OpenTelemetry setup
- `middleware/` - Shared middleware

**Dependencies**:
- `mindia-core`
- `mindia-storage` (for cleanup operations)
- `mindia-db` (for analytics, webhooks)
- `mindia-messaging` (for message queue)
- `reqwest` (for webhooks)
- OpenTelemetry (optional, feature-gated)

**Features**:
- `webhook` - Webhook delivery
- `analytics` - Analytics collection
- `rate-limit` - Rate limiting
- `telemetry` - OpenTelemetry (optional)
- `observability-opentelemetry` - Full observability

**Files to Move**:
```
mindia-services/src/services/
    ├── webhook.rs
    ├── webhook_retry.rs
    ├── analytics.rs
    ├── rate_limiter.rs
    ├── cleanup.rs
    ├── capacity.rs
    └── archive.rs

mindia-infra/src/
    ├── telemetry/
    └── middleware/
```

**New Crate Structure**:
```
mindia-infrastructure/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── webhook/
    │   ├── mod.rs
    │   ├── service.rs
    │   └── retry.rs
    ├── analytics/
    │   ├── mod.rs
    │   └── service.rs
    ├── rate_limit/
    │   ├── mod.rs
    │   └── limiter.rs
    ├── cleanup/
    │   ├── mod.rs
    │   └── service.rs
    ├── telemetry/
    │   ├── mod.rs
    │   ├── init_basic.rs
    │   └── init_opentelemetry.rs
    └── middleware/
        ├── mod.rs
        ├── request_id.rs
        └── security_headers.rs
```

### 4. mindia-services (Remaining)

**What stays in mindia-services** (if anything):
- Message queue implementation (`services/message_queue/`)
- ClamAV service (`services/clamav.rs`)
- Content moderation (`services/content_moderation.rs`)
- Ollama service (`services/ollama.rs`)

**OR**: These could also be split:
- `mindia-messaging` - Enhance with implementations (move from services)
- `mindia-security` - ClamAV, content moderation
- `mindia-ai` - Ollama integration

**Recommendation**: Split these too for cleaner structure.

## Migration Steps

### Phase 1: Preparation (Low Risk)

1. **Create new crate directories**
   ```bash
   mkdir -p mindia-storage/src
   mkdir -p mindia-media-processing/src
   mkdir -p mindia-infrastructure/src
   ```

2. **Create Cargo.toml files** for each new crate
   - Copy structure from `mindia-services/Cargo.toml`
   - Adjust dependencies
   - Set up features

3. **Update workspace Cargo.toml**
   - Add new crates to `[workspace.members]`

### Phase 2: Extract mindia-storage (Medium Risk)

1. **Create mindia-storage crate**
   - Move `services/storage/` to new crate
   - Move storage trait from `mindia-core` to `mindia-storage`
   - Update `mindia-core` to re-export from `mindia-storage` (for backward compatibility)

2. **Update dependencies**
   - Update `mindia-services` to depend on `mindia-storage`
   - Update all crates that use storage to depend on `mindia-storage`

3. **Test**
   - Run tests
   - Verify compilation
   - Check feature flags work

### Phase 3: Extract mindia-media-processing (Medium Risk)

1. **Create mindia-media-processing crate**
   - Move media processing services
   - Organize into submodules (image/, video/, audio/, document/)
   - Update dependencies

2. **Update dependencies**
   - Update `mindia-services` to depend on `mindia-media-processing`
   - Update `mindia-api`, `mindia-media-processor` to use new crate
   - Update `mindia-plugins` if needed

3. **Test**
   - Run all media processing tests
   - Verify feature flags
   - Check image/video/audio/document operations

### Phase 4: Extract mindia-infrastructure (Medium Risk)

1. **Create mindia-infrastructure crate**
   - Move infrastructure services from `mindia-services`
   - Merge `mindia-infra` into this crate
   - Organize into submodules

2. **Update dependencies**
   - Update all binaries to use `mindia-infrastructure`
   - Remove `mindia-infra` from workspace
   - Update middleware imports

3. **Test**
   - Test webhook delivery
   - Test analytics
   - Test rate limiting
   - Test telemetry

### Phase 5: Cleanup (Low Risk)

1. **Remove old code**
   - Remove moved files from `mindia-services`
   - Update `mindia-services/src/lib.rs` to re-export from new crates (for backward compatibility)
   - Or: Remove `mindia-services` entirely if nothing remains

2. **Update documentation**
   - Update crate documentation
   - Update dependency graphs
   - Update architecture docs

3. **Final testing**
   - Full integration tests
   - Performance testing
   - Verify all features work

## Backward Compatibility Strategy

### Option A: Re-export from mindia-services (Recommended)

Keep `mindia-services` as a facade that re-exports from new crates:

```rust
// mindia-services/src/lib.rs
pub use mindia_storage::*;
pub use mindia_media_processing::*;
pub use mindia_infrastructure::*;
```

**Benefits**:
- No breaking changes for existing code
- Gradual migration possible
- Can deprecate later

### Option B: Breaking Change

Remove `mindia-services` entirely and update all dependencies.

**Benefits**:
- Cleaner structure immediately
- Forces proper dependency management

**Drawbacks**:
- Breaking change
- Requires updating all crates at once

**Recommendation**: Use Option A for Phase 1-4, then Option B in Phase 5 after migration is complete.

## Dependency Updates Required

### Crates that depend on mindia-services:

1. **mindia-api**
   - Update to use `mindia-storage`, `mindia-media-processing`, `mindia-infrastructure`
   - Or: Keep using `mindia-services` (if re-exporting)

2. **mindia-media-processor**
   - Same as mindia-api

3. **mindia-control-plane**
   - Update to use `mindia-infrastructure` for webhooks, analytics
   - May not need media-processing

4. **mindia-plugins**
   - Update to use `mindia-storage`, `mindia-media-processing`

5. **mindia-services** (if keeping as facade)
   - Add dependencies on new crates
   - Re-export public API

## Feature Flag Migration

Current features in `mindia-services`:
- `image`, `video`, `audio`, `document` → Move to `mindia-media-processing`
- `storage-s3`, `storage-local` → Move to `mindia-storage`
- `message-queue` → Move to `mindia-messaging` (enhance)
- `clamav`, `semantic-search`, `content-moderation` → Keep in services or split further
- `observability-opentelemetry` → Move to `mindia-infrastructure`

**Strategy**: 
- Keep feature flags in new crates
- Pass through from `mindia-services` if keeping facade
- Update binary crates to use new crate features directly

## Testing Strategy

1. **Unit Tests**: Move with code, update imports
2. **Integration Tests**: Update to use new crates
3. **Feature Tests**: Verify all feature combinations work
4. **Performance Tests**: Ensure no regressions

## Rollback Plan

If issues arise:
1. Keep old `mindia-services` code in git history
2. Can revert commits
3. Can keep both old and new crates temporarily
4. Gradual migration allows partial rollback

## Timeline Estimate

- **Phase 1**: 1-2 days (preparation)
- **Phase 2**: 2-3 days (storage extraction)
- **Phase 3**: 3-4 days (media processing extraction)
- **Phase 4**: 2-3 days (infrastructure extraction)
- **Phase 5**: 1-2 days (cleanup)

**Total**: ~2 weeks for careful, tested migration

## Risks and Mitigation

### Risk 1: Breaking Changes
**Mitigation**: Use re-export facade, gradual migration

### Risk 2: Circular Dependencies
**Mitigation**: Careful dependency planning, review dependency graph

### Risk 3: Feature Flag Issues
**Mitigation**: Test all feature combinations, document changes

### Risk 4: Performance Regression
**Mitigation**: Benchmark before/after, monitor in production

## Success Criteria

1. ✅ All tests pass
2. ✅ No breaking changes (if using facade)
3. ✅ Compile times improve
4. ✅ Clearer crate boundaries
5. ✅ Documentation updated
6. ✅ All features work as before

## Next Steps

1. Review and approve this plan
2. Create new crate directories
3. Start with Phase 1 (preparation)
4. Proceed phase by phase with testing
5. Document lessons learned
