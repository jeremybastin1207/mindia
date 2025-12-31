# Service Boundaries Migration

## Overview

This document describes the migration of control plane handlers from `mindia-media-api` to `mindia-control-plane` service to clarify service boundaries and eliminate code duplication.

## Changes Made

### Removed from mindia-media-api

The following control plane handlers have been removed from `mindia-media-api`:

- **Authentication**: `/api/v0/auth/me`
- **OAuth**: `/api/v0/oauth/*` (if implemented)
- **Users**: `/api/v0/users/*` (if implemented)
- **Organizations**: `/api/v0/organizations/*` (if implemented)
- **Subscriptions**: `/api/v0/subscriptions/*` (if implemented)
- **Billing**: `/api/v0/billing/*` (if implemented)
- **Usage**: `/api/v0/usage/*` (if implemented)
- **Webhooks**: `/api/v0/webhooks/*`
- **API Keys**: `/api/v0/api-keys/*`

### Available in mindia-control-plane

All control plane operations are now exclusively available through the `mindia-control-plane` service:

- Authentication endpoints
- User management
- Organization management
- API key management
- Subscription management
- Billing endpoints
- Usage tracking
- Webhook management

## Migration Guide

### For API Consumers

If you are currently using control plane endpoints through `mindia-media-api`, you need to:

1. **Update API Base URL**: Change your API base URL from the media-api service to the control-plane service for control plane operations.

   **Before:**
   ```
   POST https://media-api.example.com/api/v0/webhooks
   ```

   **After:**
   ```
   POST https://control-plane.example.com/api/v0/webhooks
   ```

2. **Update SDK Configuration**: If using the Mindia SDK, configure separate endpoints for media operations and control plane operations.

3. **Update API Documentation**: Refer to the control-plane service OpenAPI documentation for control plane endpoints.

### For Developers

#### Handler Files

The handler files have been kept in `mindia-media-api/src/handlers/` but are commented out in `mod.rs` for reference. They can be removed in a future cleanup if desired.

#### Route Configuration

Control plane routes have been removed from `mindia-media-api/src/setup/routes.rs`. The `protected_routes()` function now only includes media-related routes.

#### API Documentation

Control plane endpoints have been removed from the OpenAPI specification in `mindia-media-api/src/api_doc.rs`.

## Service Architecture

### mindia-media-api

**Purpose**: Media operations only

**Handles**:
- Image upload, management, and transformations
- Video upload, transcoding, and HLS streaming
- Audio upload and management
- Document upload and management
- Media metadata operations
- Folder organization
- File groups
- Search operations
- Analytics (media-related)
- Task management
- Plugin execution

**Does NOT handle**:
- Authentication (except for media operations)
- User management
- Organization management
- API key management
- Subscriptions
- Billing
- Usage tracking
- Webhook management

### mindia-control-plane

**Purpose**: Control plane operations

**Handles**:
- Authentication (`/api/v0/auth/*`)
- OAuth (`/api/v0/oauth/*`)
- User management (`/api/v0/users/*`)
- Organization management (`/api/v0/organizations/*`)
- API key management (`/api/v0/api-keys/*`)
- Subscription management (`/api/v0/subscriptions/*`)
- Billing (`/api/v0/billing/*`)
- Usage tracking (`/api/v0/usage/*`)
- Webhook management (`/api/v0/webhooks/*`)

## Benefits

1. **Clear Separation of Concerns**: Media operations and control plane operations are now clearly separated
2. **No Code Duplication**: Control plane handlers exist in only one service
3. **Independent Scaling**: Media API and control plane can scale independently
4. **Easier Maintenance**: Changes to control plane don't affect media API and vice versa
5. **Better Documentation**: Each service has a clear, focused API surface

## Breaking Changes

⚠️ **This is a breaking change** for any clients using control plane endpoints through `mindia-media-api`.

### Affected Endpoints

All control plane endpoints that were previously available through `mindia-media-api` must now be accessed through `mindia-control-plane`:

- `/api/v0/auth/*`
- `/api/v0/oauth/*`
- `/api/v0/users/*`
- `/api/v0/organizations/*`
- `/api/v0/api-keys/*`
- `/api/v0/subscriptions/*`
- `/api/v0/billing/*`
- `/api/v0/usage/*`
- `/api/v0/webhooks/*`

### Migration Timeline

1. **Phase 1 (Current)**: Control plane endpoints removed from `mindia-media-api`
2. **Phase 2 (Future)**: Remove handler files from `mindia-media-api` after migration period
3. **Phase 3 (Future)**: Update all client SDKs and documentation

## Rollback Plan

If you need to rollback this change:

1. Uncomment the control plane handler modules in `mindia-media-api/src/handlers/mod.rs`
2. Restore the route functions in `mindia-media-api/src/setup/routes.rs`
3. Restore the API documentation entries in `mindia-media-api/src/api_doc.rs`
4. Re-add `ApiKeyRepository` import in `mindia-media-api/src/setup/routes.rs`

However, rollback is **not recommended** as it reintroduces code duplication and unclear service boundaries.

## Related Documentation

- [Service Boundaries](service-boundaries.md) - Detailed service boundary documentation
- [Architecture](architecture.md) - Overall system architecture
- [Deployment](deployment.md) - Deployment guide for both services
