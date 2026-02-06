# Authentication

Mindia authenticates requests using a **master API key** or **generated API keys**. Every request must include one of these in the Authorization header; all data access is scoped to the authenticated tenant.

## Table of Contents

- [Authentication Overview](#authentication-overview)
- [Setting Up Authentication](#setting-up-authentication)
- [Using the API Key](#using-the-api-key)
- [Best Practices](#best-practices)

## Authentication Overview

Mindia is a **multi-tenant** media processing system: it manages tenancy and isolates data per tenant. Authentication is by **master API key** (admin) or **generated API keys** (per-tenant). There is no user registration or login system—each request is authenticated by key and automatically scoped to that key’s tenant. See [Multi-Tenancy](multi-tenancy.md) and [API Keys](api-keys.md) for details.

## Setting Up Authentication

### Configure Master API Key

Set the `MASTER_API_KEY` environment variable in your `.env` file. Generate a secure key using:

```bash
openssl rand -hex 32
```

Add to your `.env` file:

```env
MASTER_API_KEY=your-secure-master-api-key-generated-above
```

The master API key must be at least 32 characters long. The server will not start without a valid master API key configured.

## Using the API Key

All API endpoints (except health checks) require authentication using the master API key or a generated API key.

### Header Format

Include the API key (master or generated) in the Authorization header:

```
Authorization: Bearer YOUR_MASTER_API_KEY
```

### Example Requests

```bash
# Using curl
MASTER_KEY="your-master-api-key-here"

curl http://localhost:3001/api/images \
  -H "Authorization: Bearer $MASTER_KEY"
```

```javascript
// Using fetch
const MASTER_API_KEY = process.env.MASTER_API_KEY;

const response = await fetch('http://localhost:3001/api/images', {
  headers: {
    'Authorization': `Bearer ${MASTER_API_KEY}`,
  },
});
```

```javascript
// Using axios
import axios from 'axios';

const api = axios.create({
  baseURL: 'http://localhost:3001',
  headers: {
    'Authorization': `Bearer ${process.env.MASTER_API_KEY}`,
  },
});

const images = await api.get('/api/images');
```

### Master Key vs Generated API Key

| | Master API key | Generated API key |
|--|----------------|-------------------|
| **Format** | Any string (32+ chars) | `mk_live_` + 40 hex chars |
| **Source** | `MASTER_API_KEY` env var | Created via `POST /api/v0/api-keys` |
| **Tenant** | Default tenant | Tenant scoped when created |
| **Use case** | Admin, CI/CD, initial setup | Per-app, per-tenant, revocable |

Use the master key for server-side admin tasks and to create generated keys. Use generated keys for applications, integrations, and per-tenant access. See [API Keys](api-keys.md) to create and manage keys.

### Authentication Validation

On each request, Mindia:
1. Extracts the token from the `Authorization` header
2. Verifies it matches the master API key, or is a valid generated API key (starts with `mk_live_`)
3. Resolves the tenant for that key and scopes the request to that tenant

If authentication fails, you'll receive a `401 Unauthorized` response.

## Best Practices

### Security

**1. Store API Key Securely**

```javascript
// ✅ Good: Use environment variables (server-side)
const MASTER_API_KEY = process.env.MASTER_API_KEY;

// ✅ Good: Use server-side proxy to hide key from clients
// Your frontend calls your backend, which adds the API key

// ❌ Bad: Hardcode in client-side JavaScript
// ❌ Bad: Commit to version control
// ❌ Bad: Share publicly
```

**2. Never Expose Master Key to Clients**

The master API key should only be used server-side. If you're building a web application:

```javascript
// Frontend (browser) - calls your backend
async function uploadImage(file) {
  const formData = new FormData();
  formData.append('image', file);
  
  // Your backend API (not Mindia directly)
  const response = await fetch('/api/upload', {
    method: 'POST',
    body: formData,
  });
  
  return response.json();
}

// Backend (Node.js/Express) - proxies to Mindia with master key
app.post('/api/upload', async (req, res) => {
  const formData = new FormData();
  formData.append('image', req.files.image);
  
  // Call Mindia with master key
  const response = await fetch('http://mindia:3001/api/images', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${process.env.MASTER_API_KEY}`,
    },
    body: formData,
  });
  
  res.json(await response.json());
});
```

**3. Use HTTPS**

Always use HTTPS in production to prevent API key interception:

```javascript
// ✅ Good
const API_URL = 'https://api.example.com';

// ❌ Bad (production)
const API_URL = 'http://api.example.com';
```

**4. Rotate Keys Regularly**

Periodically generate a new master API key and update your configuration:

```bash
# Generate new key
openssl rand -hex 32

# Update .env file
# Restart the service
```

**5. Monitor Access**

Watch for suspicious activity in your logs:
- Unusual request patterns
- Failed authentication attempts
- Requests from unexpected IPs

### Error Handling

```javascript
async function makeAuthenticatedRequest(endpoint, options = {}) {
  try {
    const response = await fetch(endpoint, {
      ...options,
      headers: {
        ...options.headers,
        'Authorization': `Bearer ${process.env.MASTER_API_KEY}`,
      },
    });

    if (response.status === 401) {
      throw new Error('Invalid API key - check MASTER_API_KEY configuration');
    }

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `API error: ${response.status}`);
    }

    return await response.json();
    
  } catch (error) {
    console.error('API request failed:', error);
    throw error;
  }
}
```

## Next Steps

- [Multi-Tenancy](multi-tenancy.md) - Tenants, API keys, and isolation
- [API Reference](api-reference.md) - Complete API documentation
- [Best Practices](best-practices.md) - Security and performance tips
- [Quick Start](quick-start.md) - Get started with Mindia

