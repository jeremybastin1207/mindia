# Service Boundaries Documentation

**Last Updated**: January 30, 2026

## Overview

Mindia currently has **one main service binary**:
- **mindia-api** (binary: `mindia-api`) - Unified API server handling all operations

**Historical Note**: Previous architecture planned separate services (`mindia-media-api`, `mindia-media-processor`, `mindia-control-plane`) but these have been **consolidated into a single service** for simplicity and easier deployment.

## Current State Analysis

### mindia-media-api (Main API Server)

**Binary Name**: `mindia-media-api` (matches crate name)

**Purpose**: Unified API gateway handling all media operations and some control plane features.

**Handlers (38 total)**:
- **Media Operations**:
  - Image: upload, get, download, transform
  - Video: upload, get, stream
  - Audio: upload, get, download
  - Document: upload, get, download
  - Media: get, delete
  - File groups, folders
  - Chunked upload, presigned upload
  - Transformations
  - Search
  - Metadata
  - Analytics
  - Tasks
  - Plugins

- **Auth & Tenancy** (no separate control-plane service):
  - API keys (create, list, get, revoke)
  - Tenant-scoped auth via master key or API keys
  - Webhooks

**Routes**: Fully implemented with comprehensive middleware (auth, rate limiting, CORS, telemetry)

**Status**: ✅ Fully functional, actively used

### mindia-media-processor

**Binary Name**: `mindia-media-processor` (matches crate name)

**Purpose**: Intended to be a dedicated media processing service.

**Handlers (28 total)**:
- **Media Operations Only**:
  - Image: upload, get, download, transform
  - Video: upload, get, stream
  - Audio: upload, get, download
  - Document: upload, get, download
  - Media: get, delete
  - File groups, folders
  - Chunked upload, presigned upload
  - Transformations
  - Search
  - Metadata
  - Analytics
  - Tasks
  - Plugins
  - Config

**Routes**: ⚠️ **PLACEHOLDER** - `setup_routes()` returns empty router

**Status**: ⚠️ **INCOMPLETE** - Handlers exist but routes are not wired up

**Note**: Handlers are nearly identical to mindia-api media handlers, suggesting duplication or incomplete migration.

### mindia-control-plane

**Binary Name**: `mindia-control-plane` (matches crate name)

**Purpose**: Control plane operations (auth, billing, organizations).

**Handlers (10 total)**:
- Auth (login, register, OAuth)
- Users
- Organizations
- API keys
- Subscriptions
- Billing
- Usage
- Webhooks
- Stripe

**Routes**: Fully implemented

**Status**: ✅ Fully functional, actively used

## Service Boundary Issues

### 1. Overlap Between mindia-api and mindia-control-plane

**Problem**: `mindia-api` contains control plane handlers that duplicate `mindia-control-plane` functionality:
- Auth endpoints
- User management
- Organization management
- API keys
- Subscriptions
- Billing
- Usage
- Webhooks

**Impact**: 
- Code duplication
- Unclear which service handles what
- Potential inconsistencies

**Recommendation**: 
- Remove control plane handlers from `mindia-api`
- Route control plane requests to `mindia-control-plane` service
- Or: Make `mindia-api` a pure media API gateway

### 2. mindia-media-processor is Incomplete ✅ **RESOLVED**

**Problem**: 
- Handlers exist but routes are not implemented
- Unclear if this service is intended to replace `mindia-api` or run alongside it
- Significant code duplication with `mindia-api`

**Resolution**: 
- ✅ Service has been **deprecated**
- ✅ Marked as deprecated in code with warnings
- ✅ Documentation added explaining deprecation
- ✅ All functionality available in `mindia-media-api`

**Current State**:
- Service returns empty router (routes never implemented)
- Deprecation warnings added to code
- See `mindia-media-processor/DEPRECATED.md` for details

**Decision**: 
- Use `mindia-media-api` for all media operations
- `mindia-media-processor` may be removed in a future release

### 3. Binary Name Mismatch

**Problem**: `mindia-api` crate has binary named `mindia` instead of `mindia-api`

**Impact**: 
- Confusing for developers
- Deployment scripts may reference wrong name
- Inconsistent with other services

**Recommendation**: Rename binary to `mindia-api` or rename crate to `mindia`

## Recommended Service Architecture

### Option A: Unified Media API (Simplest)

Keep `mindia-api` as the single media API service, remove `mindia-media-processor`:

```
mindia-api (media operations only)
mindia-control-plane (control plane operations)
```

**Benefits**:
- Simple architecture
- No code duplication
- Clear boundaries

**Action Items**:
1. Remove control plane handlers from `mindia-api`
2. Remove or archive `mindia-media-processor`
3. Rename `mindia-api` binary to `mindia-api`

### Option B: Separate Media Processing Service

Complete `mindia-media-processor` and use it for heavy processing:

```
mindia-api (API gateway, lightweight operations)
mindia-media-processor (heavy processing: transcoding, transformations)
mindia-control-plane (control plane operations)
```

**Benefits**:
- Can scale media processing independently
- Separates concerns

**Action Items**:
1. Complete `mindia-media-processor` routes
2. Move heavy processing to media-processor
3. Make `mindia-api` a lightweight gateway
4. Document communication patterns

### Option C: Microservices Architecture (Most Complex)

Split into focused services:

```
mindia-media-api (API gateway)
mindia-media-processor (processing)
mindia-storage-service (storage operations)
mindia-control-plane (control plane)
```

**Benefits**:
- Maximum scalability
- Independent deployment

**Action Items**:
1. Significant refactoring
2. Service discovery
3. Inter-service communication

## Recommended Approach

**Recommendation**: **Option A** - Unified Media API

**Rationale**:
1. `mindia-media-processor` is incomplete and appears to be abandoned
2. Current architecture works well with two services
3. Simpler to maintain and deploy
4. Can always split later if needed

**Implementation Steps**:

1. **Phase 1: Cleanup** ✅ **COMPLETED**
   - ✅ Remove control plane handlers from `mindia-media-api`
   - ✅ Deprecate `mindia-media-processor` (marked as deprecated, not removed)
   - ✅ Binary renamed to `mindia-media-api` (completed in previous refactoring)

2. **Phase 2: Documentation** ✅ **COMPLETED**
   - ✅ Document that `mindia-media-api` handles all media operations
   - ✅ Document that `mindia-control-plane` handles all control plane operations
   - ✅ Created migration guide (see `service-boundaries-migration.md`)

3. **Phase 3: Future Consideration**
   - If scaling becomes an issue, consider Option B
   - Monitor performance to determine if separation is needed

## Current Status

✅ **Control plane handlers have been removed from `mindia-media-api`**

See [Service Boundaries Migration](service-boundaries-migration.md) for details on the migration and breaking changes.

## Service Communication

### Current Communication Pattern

- **mindia-api** ↔ **mindia-control-plane**: Likely via shared database
- **mindia-media-processor**: Not active, no communication

### Recommended Communication Pattern

- **mindia-api** ↔ **mindia-control-plane**: 
  - Shared database for data
  - HTTP API calls for operations (if needed)
  - Message queue for async operations (if implemented)

## Deployment Considerations

### Current Deployment

Based on deployment scripts:
- `mindia-api` is deployed as main service
- `mindia-control-plane` is deployed separately
- `mindia-media-processor` deployment unclear (may not be deployed)

### Recommended Deployment

- **mindia-api**: Deploy as media API service
- **mindia-control-plane**: Deploy as control plane service
- Both can scale independently
- Both share same database

## Questions to Resolve

1. **Is `mindia-media-processor` actively used?**
   - Check deployment configs
   - Check if routes are actually being used
   - Determine if it's legacy code

2. **Why does `mindia-api` have control plane handlers?**
   - Historical reasons?
   - Backward compatibility?
   - Should they be removed?

3. **What is the intended architecture?**
   - Two-service model (api + control-plane)?
   - Three-service model (api + media-processor + control-plane)?
   - Something else?

## Conclusion

The current service boundaries are unclear due to:
- Incomplete `mindia-media-processor` service
- Overlap between `mindia-api` and `mindia-control-plane`
- Binary name mismatch

**Immediate Actions**:
1. Document current state (this document)
2. Decide on architecture (Option A, B, or C)
3. Implement cleanup based on decision

**Long-term**:
- Monitor performance
- Consider splitting if scaling requires it
- Maintain clear service boundaries
