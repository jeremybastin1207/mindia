# API Versioning Strategy

Guide for API versioning in Mindia to ensure backward compatibility and smooth evolution.

## Current Status

**Current Version**: v1 (implicit)

All current API endpoints are considered v1. They are accessible at:
- `/api/images`
- `/api/videos`
- `/api/documents`
- `/api/audios`
- etc.

## Versioning Strategy

### URL Path Versioning

We use URL path versioning: `/api/v1/`, `/api/v2/`, etc.

**Benefits**:
- Clear and explicit
- Easy to route different versions
- Allows gradual migration
- Supports multiple versions simultaneously

### Version Format

- **Major versions**: `/api/v1/`, `/api/v2/` (breaking changes)
- **Minor versions**: Not in URL (non-breaking additions)
- **Patch versions**: Not in URL (bug fixes)

## Version Lifecycle

### v1 (Current)

- **Status**: Stable
- **Endpoints**: All current endpoints
- **Deprecation**: Not planned
- **Support**: Long-term support

### Future Versions

#### v2 (Planned)

- **When**: When breaking changes are needed
- **Migration**: 6-month overlap period
- **Deprecation**: v1 deprecated 12 months after v2 release

## Breaking vs Non-Breaking Changes

### Breaking Changes (New Major Version)

- Removing endpoints
- Changing request/response formats
- Changing authentication methods
- Removing required fields
- Changing data types

### Non-Breaking Changes (Same Version)

- Adding new endpoints
- Adding optional fields
- Adding new query parameters
- Adding new response fields
- Performance improvements

## Migration Strategy

### Gradual Migration

1. **Release v2** alongside v1
2. **Announce deprecation** of v1 (12-month notice)
3. **Provide migration guide**
4. **Monitor usage** of both versions
5. **Sunset v1** after deprecation period

### Client Migration

```javascript
// v1 (deprecated)
const response = await fetch('/api/images');

// v2 (new)
const response = await fetch('/api/v2/images');
```

## Deprecation Process

### 1. Announcement

- Email to API users
- Documentation update
- Deprecation header in responses

### 2. Deprecation Header

```http
API-Deprecated: true
API-Deprecation-Date: 2025-12-31
API-Sunset-Date: 2026-12-31
```

### 3. Migration Period

- 6-12 months overlap
- Support for both versions
- Migration documentation
- Support assistance

### 4. Sunset

- Remove deprecated version
- Update documentation
- Notify users of removal

## Version Detection

### Request Headers

Clients can specify version preference:

```http
API-Version: 2
```

### Default Behavior

- No version specified: Latest stable version
- Deprecated version: Returns with deprecation headers

## Documentation

### Version-Specific Docs

- `/docs/v1/` - v1 API documentation
- `/docs/v2/` - v2 API documentation
- `/docs/latest/` - Latest version docs

### Changelog

Maintain changelog for each version:
- New features
- Breaking changes
- Deprecations
- Bug fixes

## Implementation

### Route Structure

```rust
// v1 routes (current)
.route("/api/images", ...)
.route("/api/videos", ...)

// v2 routes (future)
.route("/api/v2/images", ...)
.route("/api/v2/videos", ...)
```

### Version Middleware

```rust
// Extract API version from path or header
let api_version = extract_api_version(request)?;
request.extensions_mut().insert(api_version);
```

## Best Practices

1. **Minimize Breaking Changes**: Design APIs carefully
2. **Clear Communication**: Document all changes
3. **Adequate Notice**: Give users time to migrate
4. **Backward Compatibility**: Maintain as long as feasible
5. **Version Documentation**: Keep docs up to date

## Examples

### Adding New Endpoint (Non-Breaking)

```rust
// Add to v1 (non-breaking)
.route("/api/v1/search", get(search_files))
```

### Changing Response Format (Breaking)

```rust
// v1 response
{
  "id": "...",
  "url": "..."
}

// v2 response (breaking change)
{
  "data": {
    "id": "...",
    "url": "..."
  },
  "meta": {...}
}
```

## Client Guidelines

### Version Pinning

```javascript
// Pin to specific version
const API_BASE = 'https://api.example.com/api/v1';

// Or use latest
const API_BASE = 'https://api.example.com/api';
```

### Handling Deprecation

```javascript
// Check for deprecation headers
const deprecationDate = response.headers.get('API-Sunset-Date');
if (deprecationDate) {
  console.warn(`API version deprecated. Sunset: ${deprecationDate}`);
  // Plan migration
}
```

## Next Steps

- [ ] Document current v1 API
- [ ] Plan v2 features
- [ ] Create migration guides
- [ ] Set up version routing
- [ ] Implement deprecation headers

