# Error Handling

Guide to handling errors and status codes in Mindia's API.

## HTTP Status Codes

| Code | Meaning | Common Causes |
|------|---------|---------------|
| 200 | OK | Successful GET request |
| 201 | Created | Successful POST (upload, create) |
| 204 | No Content | Successful DELETE |
| 400 | Bad Request | Invalid input, wrong format |
| 401 | Unauthorized | Missing/invalid token, expired session |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource doesn't exist |
| 413 | Payload Too Large | File exceeds size limit |
| 422 | Unprocessable Entity | Virus detected, invalid file |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server error |
| 503 | Service Unavailable | Server overloaded/maintenance |

## Error Response Format

All errors return JSON in a consistent shape. Use the `code` field for programmatic handling and `recoverable` to decide whether to retry.

| Field | Type | Description |
|-------|------|-------------|
| `error` | string | Human-readable message (always present) |
| `code` | string | Machine-readable code (e.g. `NOT_FOUND`, `INVALID_INPUT`) for client logic |
| `recoverable` | boolean | If `true`, the client may retry; if `false`, retrying is unlikely to help |
| `details` | string? | Extra context (often omitted in production for security) |
| `error_type` | string? | Internal error classification (often omitted in production) |
| `suggested_action` | string? | Hint for the user (e.g. "Wait 60s and retry") |

Example response:

```json
{
  "error": "Media not found",
  "code": "NOT_FOUND",
  "recoverable": false
}
```

With optional fields (e.g. in non-production):

```json
{
  "error": "Database connection failed",
  "details": "Connection timed out after 30s",
  "error_type": "Database",
  "code": "DATABASE_ERROR",
  "recoverable": true,
  "suggested_action": "Wait 60s and retry"
}
```

**Client handling:** Prefer switching on `code` for branching; use `recoverable` to drive retry/backoff. Do not rely on `details` or `error_type` in production, as they may be hidden.

## Common Errors

### Authentication Errors (401)

```json
{
  "error": "Invalid or expired token"
}
```

**Solutions**:
- Check token is valid
- Re-authenticate if expired
- Verify `Authorization` header format

### Permission Errors (403)

```json
{
  "error": "Insufficient permissions"
}
```

**Solutions**:
- Check user role (admin, member, viewer)
- Contact admin for permission upgrade
- Use correct account

### Not Found (404)

```json
{
  "error": "Image not found"
}
```

**Solutions**:
- Verify resource ID is correct
- Check resource belongs to your tenant
- Resource may have been deleted

### File Too Large (413)

```json
{
  "error": "File size exceeds maximum of 10MB"
}
```

**Solutions**:
- Compress file before upload
- Check configured limits
- Use appropriate media type

### Virus Detected (422)

```json
{
  "error": "File contains malware and was rejected"
}
```

**Solutions**:
- Scan file locally
- Don't upload suspicious files
- Contact support if false positive

### Rate Limited (429)

```json
{
  "error": "Too many requests. Please slow down."
}
```

**Solutions**:
- Implement exponential backoff
- Check `Retry-After` header
- Reduce request frequency

## Client Implementation

### JavaScript Error Handler

```javascript
async function apiCall(url, options = {}) {
  try {
    const response = await fetch(url, {
      ...options,
      headers: {
        'Authorization': `Bearer ${localStorage.getItem('token')}`,
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    // Handle specific status codes
    if (response.status === 401) {
      // Token expired or invalid
      handleAuthError();
      throw new Error('Authentication required');
    }

    if (response.status === 403) {
      throw new Error('You don't have permission for this action');
    }

    if (response.status === 404) {
      throw new Error('Resource not found');
    }

    if (response.status === 429) {
      const retryAfter = response.headers.get('Retry-After') || 60;
      throw new Error(`Rate limited. Retry after ${retryAfter} seconds`);
    }

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    return await response.json();

  } catch (error) {
    console.error('API Error:', error);
    throw error;
  }
}

function handleAuthError() {
  // Clear token
  localStorage.removeItem('token');
  
  // Redirect to login
  window.location.href = '/login';
}
```

### Retry Logic

```javascript
async function fetchWithRetry(url, options = {}, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await apiCall(url, options);
    } catch (error) {
      // Don't retry client errors (4xx)
      if (error.message.includes('401') || error.message.includes('403')) {
        throw error;
      }

      // Retry on server errors (5xx) or network errors
      if (attempt === maxRetries) {
        throw error;
      }

      // Exponential backoff
      const delay = Math.min(1000 * Math.pow(2, attempt), 10000);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
}
```

### React Hook

```tsx
function useApi() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function call(url: string, options?: RequestInit) {
    setLoading(true);
    setError(null);

    try {
      const data = await apiCall(url, options);
      return data;
    } catch (err) {
      setError(err.message);
      throw err;
    } finally {
      setLoading(false);
    }
  }

  return { call, loading, error };
}

// Usage
function MyComponent() {
  const { call, loading, error } = useApi();

  async function uploadImage(file) {
    try {
      const result = await call('/api/images', {
        method: 'POST',
        body: formData,
      });
      console.log('Success:', result);
    } catch (err) {
      console.error('Failed:', err);
    }
  }

  return (
    <div>
      {loading && <p>Loading...</p>}
      {error && <p>Error: {error}</p>}
    </div>
  );
}
```

## Best Practices

### 1. Always Check Status Codes

```javascript
// ✅ Good
if (response.status === 401) {
  handleAuthError();
}

// ❌ Bad: Ignore errors
await fetch(url); // No error handling
```

### 2. Provide User-Friendly Messages

```javascript
function getUserMessage(error) {
  if (error.message.includes('401')) {
    return 'Please log in again';
  }
  if (error.message.includes('403')) {
    return 'You don't have permission for this action';
  }
  if (error.message.includes('404')) {
    return 'Item not found';
  }
  if (error.message.includes('413')) {
    return 'File is too large';
  }
  return 'Something went wrong. Please try again.';
}
```

### 3. Log Errors

```javascript
function logError(error, context) {
  console.error('Error:', {
    message: error.message,
    context,
    timestamp: new Date().toISOString(),
    user: getCurrentUser()?.id,
  });

  // Send to error tracking service
  if (window.Sentry) {
    Sentry.captureException(error, { extra: context });
  }
}
```

### 4. Implement Global Error Handler

```javascript
// React Error Boundary
class ErrorBoundary extends React.Component {
  state = { hasError: false, error: null };

  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }

  componentDidCatch(error, errorInfo) {
    logError(error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return <ErrorPage error={this.state.error} />;
    }

    return this.props.children;
  }
}
```

## Next Steps

- [Rate Limiting](rate-limiting.md) - Handle rate limits
- [Authentication](authentication.md) - Token management
- [Best Practices](best-practices.md) - Production tips

