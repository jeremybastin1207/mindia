# Authentication System

Mindia's authentication system supports both JWT tokens (from external user services) and API keys for programmatic access. This document explains the architecture and implementation.

## Overview

The authentication system provides:
- **JWT Token Validation**: Validates tokens from external user services
- **API Key Authentication**: Supports programmatic access with API keys
- **Tenant Context**: Injects tenant information into request handlers
- **Role-Based Access**: Supports admin, member, and viewer roles

## Architecture

```
Request → Auth Middleware → Tenant Context → Handler
              ↓
         JWT or API Key
              ↓
         Tenant Lookup
              ↓
         Role Parsing
```

## Components

### 1. JWT Service (`src/auth/jwt.rs`)

Validates JWT tokens from external user services.

**Key Features:**
- Token validation using shared secret
- Claims extraction (user ID, tenant ID, role)
- Role parsing (admin, member, viewer)

**Usage:**
```rust
let jwt_service = JwtService::new(config.jwt_secret.clone());
let claims = jwt_service.validate_token(token)?;
let role = JwtService::parse_role(&claims.role)?;
```

**JWT Claims Structure:**
```rust
pub struct JwtClaims {
    pub sub: Uuid,           // User ID
    pub tenant_id: Uuid,     // Tenant ID
    pub role: String,         // "admin", "member", or "viewer"
    pub exp: i64,            // Expiration timestamp
    // ... standard JWT fields
}
```

### 2. API Key Service (`src/auth/api_key.rs`)

Manages API key authentication and validation.

**Key Features:**
- API key format: `mk_live_<uuid>`
- Tenant association
- Key validation and lookup
- Revocation support

**API Key Format:**
- Prefix: `mk_live_`
- UUID: Unique identifier
- Example: `mk_live_550e8400-e29b-41d4-a716-446655440000`

### 3. Auth Middleware (`src/auth/middleware.rs`)

Axum middleware that authenticates requests and injects tenant context.

**Flow:**
1. Extract `Authorization` header
2. Determine if token is JWT or API key (by prefix)
3. Authenticate and load tenant
4. Create `TenantContext`
5. Inject context into request extensions

**Tenant Context:**
```rust
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,        // None for API keys
    pub role: UserRole,
    pub tenant: Tenant,       // Full tenant record
}
```

**Usage in Handlers:**
```rust
pub async fn handler(
    Extension(ctx): Extension<TenantContext>,
    // ... other params
) -> Result<impl IntoResponse, AppError> {
    let tenant_id = ctx.tenant_id;
    // Use tenant_id for data isolation
}
```

### 4. Tenant Repository (`src/db/tenant.rs`)

Database operations for tenant management.

**Key Operations:**
- Get tenant by ID
- Verify tenant status (active/inactive)
- Tenant creation and updates

## Authentication Flow

### JWT Token Flow

```
1. Client sends: Authorization: Bearer <jwt_token>
2. Middleware extracts token
3. JwtService validates token
4. Extract claims (user_id, tenant_id, role)
5. Load tenant from database
6. Verify tenant is active
7. Create TenantContext
8. Inject into request
```

### API Key Flow

```
1. Client sends: Authorization: Bearer mk_live_<uuid>
2. Middleware detects API key prefix
3. Extract key ID (UUID)
4. Lookup API key in database
5. Verify key is active and not revoked
6. Load associated tenant
7. Verify tenant is active
8. Create TenantContext (user_id = None)
9. Inject into request
```

## User Roles

### Admin
- Full access to all operations
- Can manage API keys
- Can configure plugins
- Can manage webhooks

### Member
- Standard user access
- Can upload/manage media
- Can create folders and groups
- Cannot manage system settings

### Viewer
- Read-only access
- Can view media and metadata
- Cannot upload or modify

## Security Considerations

### JWT Tokens
- **Secret Management**: JWT secret must be kept secure
- **Token Expiration**: Tokens should have reasonable expiration times
- **External Service**: JWT generation is handled by external user service
- **Validation**: Always validate token signature and expiration

### API Keys
- **Key Format**: Use consistent prefix for easy identification
- **Storage**: Keys are hashed in database (never store plaintext)
- **Revocation**: Support immediate key revocation
- **Rotation**: Implement key rotation policies

### Tenant Isolation
- **Data Isolation**: All queries must filter by `tenant_id`
- **Context Injection**: Always use `TenantContext` from middleware
- **Validation**: Verify tenant exists and is active

## Implementation Details

### Middleware Setup

```rust
// In routes setup
let auth_middleware_state = AuthMiddlewareState {
    jwt_service: jwt_service.clone(),
    tenant_repository: tenant_db.clone(),
    api_key_repository: api_key_db.clone(),
};

let protected_routes = Router::new()
    .route("/api/images", post(upload_image))
    .layer(axum::middleware::from_fn_with_state(
        auth_middleware_state,
        auth_middleware,
    ));
```

### Handler Usage

```rust
use crate::auth::models::TenantContext;

pub async fn upload_image(
    Extension(ctx): Extension<TenantContext>,
    State(state): State<Arc<AppState>>,
    // ... other params
) -> Result<impl IntoResponse, AppError> {
    // ctx.tenant_id is automatically available
    // All database queries should filter by tenant_id
    let images = state.image_repo
        .list_images(ctx.tenant_id)
        .await?;
    
    Ok(Json(images))
}
```

### API Key Creation

```rust
// In API key handler
pub async fn create_api_key(
    Extension(ctx): Extension<TenantContext>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // Only admins can create API keys
    if !matches!(ctx.role, UserRole::Admin) {
        return Err(AppError::Forbidden("Only admins can create API keys".to_string()));
    }
    
    let api_key = state.api_key_repo
        .create_key(ctx.tenant_id, "My API Key")
        .await?;
    
    Ok(Json(api_key))
}
```

## Error Handling

### Unauthorized (401)
- Missing `Authorization` header
- Invalid token format
- Expired or invalid JWT token
- Invalid or revoked API key
- Tenant not found or inactive

### Forbidden (403)
- Insufficient permissions for operation
- Role-based access control violations

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_validation() {
        let jwt_service = JwtService::new("test_secret".to_string());
        // Test valid/invalid tokens
    }

    #[test]
    fn test_role_parsing() {
        assert!(matches!(
            JwtService::parse_role("admin"),
            Ok(UserRole::Admin)
        ));
    }
}
```

### Integration Tests

Test authentication flow end-to-end:
- Valid JWT token
- Invalid JWT token
- Valid API key
- Revoked API key
- Tenant isolation

## Configuration

### Environment Variables

```env
# JWT secret (must match external user service)
JWT_SECRET=your-secret-key-here

# API key settings (optional)
API_KEY_PREFIX=mk_live_
```

### Database Schema

**Tenants Table:**
- `id`: UUID primary key
- `name`: Tenant name
- `status`: Active/Inactive
- `created_at`, `updated_at`: Timestamps

**API Keys Table:**
- `id`: UUID primary key
- `tenant_id`: Foreign key to tenants
- `key_hash`: Hashed API key
- `name`: Human-readable name
- `revoked_at`: Revocation timestamp
- `created_at`, `updated_at`: Timestamps

## Best Practices

1. **Always Use Tenant Context**: Never trust client-provided tenant IDs
2. **Validate Early**: Check authentication before any business logic
3. **Role Checks**: Verify permissions at handler level
4. **Error Messages**: Don't leak sensitive information in errors
5. **Logging**: Log authentication failures for security monitoring
6. **Rate Limiting**: Apply rate limits to authentication endpoints

## Related Documentation

- [API Keys](../user/api-keys.md) - User guide for API keys
- [Authorization](../user/authorization.md) - Role-based access control
- [Multi-Tenancy](../user/multi-tenancy.md) - Tenant isolation
- [Code Structure](code-structure.md) - Project organization

