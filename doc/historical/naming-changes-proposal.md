# Naming Changes Proposal

## Overview

This document proposes specific naming changes to improve clarity and consistency across the Mindia codebase.

## Current Naming Issues

### 1. Crate/Binary Name Mismatch
- **Crate**: `mindia-api`
- **Binary**: `mindia`
- **Issue**: Inconsistent naming causes confusion

### 2. Generic Names
- `mindia-services` - Too generic, doesn't indicate what services
- `mindia-infra` - Too generic, very small crate

### 3. Unclear Service Names
- `mindia-api` - Unclear if it's the main API or a specific API
- `mindia-media-processor` - Unclear relationship to `mindia-api`

## Proposed Changes

### Change 1: Rename mindia-api Binary

**Current**:
```toml
[package]
name = "mindia-api"

[[bin]]
name = "mindia"
path = "src/main.rs"
```

**Proposed**:
```toml
[package]
name = "mindia-api"

[[bin]]
name = "mindia-api"
path = "src/main.rs"
```

**Impact**:
- **Low Risk**: Simple rename
- **Breaking**: Yes, but only for deployment scripts and references
- **Files to Update**:
  - `mindia-api/Cargo.toml`
  - Deployment scripts (`deploy.sh`, `scripts/deploy-to-eks.sh`)
  - Docker files (if they reference binary name)
  - Documentation
  - CI/CD pipelines

**Migration Steps**:
1. Update `Cargo.toml`
2. Search for references to `mindia` binary
3. Update deployment scripts
4. Update documentation
5. Test deployment

**Alternative**: Rename crate to `mindia` instead (less clear)

### Change 2: Rename mindia-api to mindia-media-api

**Current**: `mindia-api`

**Proposed**: `mindia-media-api`

**Rationale**: 
- Clarifies this is the media API service
- Distinguishes from potential other APIs
- Matches pattern: `mindia-control-plane`, `mindia-media-processor`

**Impact**:
- **Medium Risk**: Requires updating all references
- **Breaking**: Yes
- **Files to Update**:
  - Crate directory name: `mindia-api/` → `mindia-media-api/`
  - `Cargo.toml` workspace members
  - All `Cargo.toml` files that depend on it
  - Import statements in source code
  - Documentation
  - Deployment configs

**Migration Steps**:
1. Rename directory
2. Update `Cargo.toml` package name
3. Update workspace `Cargo.toml`
4. Update all dependencies
5. Update imports (search/replace)
6. Update documentation
7. Test compilation

**Alternative**: Keep `mindia-api` if it's the only/main API

### Change 3: Rename mindia-services to mindia-media-services

**Current**: `mindia-services`

**Proposed**: `mindia-media-services` (if keeping as single crate)

**OR**: Split into focused crates (see migration plan):
- `mindia-storage`
- `mindia-media-processing`
- `mindia-infrastructure`

**Rationale**: 
- If keeping single crate, name should reflect it's media-focused
- If splitting, names should be specific to their domain

**Impact**:
- **High Risk** if renaming (many dependencies)
- **Medium Risk** if splitting (requires migration)
- **Files to Update**: Similar to Change 2

**Recommendation**: Split into focused crates rather than rename

### Change 4: Merge mindia-infra into mindia-infrastructure

**Current**: `mindia-infra` (small crate with telemetry/middleware)

**Proposed**: 
1. Create `mindia-infrastructure` (new crate)
2. Merge `mindia-infra` into it
3. Also merge infrastructure services from `mindia-services`
4. Remove `mindia-infra`

**Rationale**:
- `mindia-infra` is too small to be separate
- Consolidates all infrastructure concerns
- Better naming (`infrastructure` vs `infra`)

**Impact**:
- **Medium Risk**: Requires updating dependencies
- **Breaking**: Yes
- **Files to Update**:
  - Remove `mindia-infra` directory
  - Create `mindia-infrastructure` directory
  - Move code from `mindia-infra` and `mindia-services`
  - Update all dependencies
  - Update imports

**Migration Steps**:
1. Create `mindia-infrastructure` crate
2. Move code from `mindia-infra`
3. Move infrastructure code from `mindia-services`
4. Update all dependencies
5. Remove `mindia-infra`
6. Test

### Change 5: Enhance mindia-messaging or Merge into mindia-core

**Current**: `mindia-messaging` (only type definitions)

**Option A: Merge into mindia-core**
- Move message types to `mindia-core/src/messaging_types.rs`
- Move trait to `mindia-core/src/messaging.rs` (already there)
- Remove `mindia-messaging` crate

**Option B: Enhance mindia-messaging**
- Keep crate
- Move trait from `mindia-core` to `mindia-messaging`
- Move implementations from `mindia-services` to `mindia-messaging`
- Make it a full messaging system crate

**Recommendation**: **Option A** - Merge into core
- Types and traits are closely related
- Keeps core as foundation
- Simpler structure

**Impact**:
- **Low Risk**: Simple move
- **Breaking**: Yes, but straightforward
- **Files to Update**:
  - Move `mindia-messaging/src/lib.rs` to `mindia-core`
  - Update `mindia-core` to include message types
  - Remove `mindia-messaging` crate
  - Update all imports
  - Update dependencies

## Naming Convention Standards

### Proposed Conventions

**Library Crates** (no binary):
- Pattern: `mindia-{domain}`
- Examples: `mindia-core`, `mindia-storage`, `mindia-config`

**Service Binaries**:
- Pattern: `mindia-{service-name}`
- Examples: `mindia-media-api`, `mindia-control-plane`

**CLI Tools**:
- Pattern: `mindia-cli` with multiple binaries
- Examples: `mindia-cli` with binaries `media_stats`, `list_media`

### Current vs Proposed Naming

| Current | Proposed | Type | Priority |
|--------|----------|------|----------|
| `mindia-api` (crate) | `mindia-media-api` | Service | High |
| `mindia` (binary) | `mindia-api` or `mindia-media-api` | Binary | High |
| `mindia-services` | Split into `mindia-storage`, `mindia-media-processing`, `mindia-infrastructure` | Library | Medium |
| `mindia-infra` | Merge into `mindia-infrastructure` | Library | Medium |
| `mindia-messaging` | Merge into `mindia-core` | Library | Low |

## Impact Analysis Summary

### High Impact Changes

1. **Rename mindia-api to mindia-media-api**
   - Affects: All crates, deployment, documentation
   - Risk: Medium
   - Benefit: High (clarity)

2. **Split mindia-services**
   - Affects: All crates that depend on services
   - Risk: Medium-High
   - Benefit: High (maintainability, compile times)

### Medium Impact Changes

1. **Rename mindia-api binary**
   - Affects: Deployment scripts, documentation
   - Risk: Low
   - Benefit: Medium (consistency)

2. **Merge mindia-infra**
   - Affects: All binaries
   - Risk: Low-Medium
   - Benefit: Medium (consolidation)

### Low Impact Changes

1. **Merge mindia-messaging**
   - Affects: Core and services crates
   - Risk: Low
   - Benefit: Low-Medium (simplification)

## Migration Order

### Phase 1: Low-Risk, High-Value
1. Rename `mindia-api` binary to `mindia-api`
2. Merge `mindia-messaging` into `mindia-core`

### Phase 2: Medium-Risk, High-Value
3. Rename `mindia-api` crate to `mindia-media-api`
4. Merge `mindia-infra` into new `mindia-infrastructure`

### Phase 3: High-Risk, High-Value
5. Split `mindia-services` into focused crates

## Detailed Change Plans

### Change 1: Rename Binary (mindia → mindia-api)

**Files to Update**:

1. `mindia-api/Cargo.toml`:
   ```toml
   [[bin]]
   name = "mindia-api"  # Changed from "mindia"
   path = "src/main.rs"
   ```

2. Search and update references:
   ```bash
   # Find references
   grep -r "mindia" deploy.sh scripts/ Dockerfile* docker-compose.yml
   
   # Update deployment scripts
   # Update Docker files
   # Update documentation
   ```

**Testing**:
- Verify binary builds as `mindia-api`
- Test deployment scripts
- Verify Docker images work

### Change 2: Rename Crate (mindia-api → mindia-media-api)

**Files to Update**:

1. Rename directory:
   ```bash
   mv mindia-api mindia-media-api
   ```

2. `mindia-media-api/Cargo.toml`:
   ```toml
   [package]
   name = "mindia-media-api"
   ```

3. Root `Cargo.toml`:
   ```toml
   [workspace]
   members = [
     "mindia-media-api",  # Changed from "mindia-api"
     ...
   ]
   ```

4. Update all `Cargo.toml` files that depend on it:
   ```toml
   mindia-media-api = { path = "../mindia-media-api" }
   ```

5. Update source code imports (if any):
   ```rust
   // Usually not needed as crate names aren't imported
   ```

**Testing**:
- Verify workspace builds
- Run all tests
- Check deployment

### Change 3: Merge mindia-messaging into mindia-core

**Files to Update**:

1. `mindia-core/src/messaging_types.rs`:
   ```rust
   // Move content from mindia-messaging/src/lib.rs here
   ```

2. `mindia-core/Cargo.toml`:
   ```toml
   # Remove: mindia-messaging = { path = "../mindia-messaging" }
   # Message types are now in this crate
   ```

3. Update `mindia-core/src/messaging.rs`:
   ```rust
   // Already re-exports Message, now it's defined here
   pub use crate::messaging_types::Message;
   ```

4. Remove `mindia-messaging` directory

5. Update root `Cargo.toml`:
   ```toml
   # Remove from members
   ```

6. Update all crates that depend on `mindia-messaging`:
   ```toml
   # Remove: mindia-messaging = { path = "../mindia-messaging" }
   # Use: mindia-core (already depends on it)
   ```

**Testing**:
- Verify message types work
- Test message queue functionality
- Check all services compile

## Rollback Plan

For each change:
1. Keep old code in git (don't delete immediately)
2. Use feature flags if needed for gradual migration
3. Can revert commits if issues arise
4. Document rollback steps

## Success Criteria

1. ✅ All crates compile
2. ✅ All tests pass
3. ✅ Deployment works
4. ✅ Documentation updated
5. ✅ No breaking changes for end users (internal refactoring)
6. ✅ Clearer naming conventions

## Timeline

- **Change 1** (Binary rename): 1 day
- **Change 2** (Crate rename): 2-3 days
- **Change 3** (Messaging merge): 1-2 days
- **Change 4** (Infra merge): 2-3 days
- **Change 5** (Services split): 1-2 weeks (see migration plan)

**Total**: ~3-4 weeks for all changes (can be done incrementally)

## Recommendations

1. **Start with low-risk changes** (binary rename, messaging merge)
2. **Do crate rename before splitting services** (cleaner migration)
3. **Use feature flags** for gradual migration if needed
4. **Test thoroughly** after each change
5. **Update documentation** as you go
6. **Communicate changes** to team
