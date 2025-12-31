# Authentication

Mindia uses a master API key for authentication. All API requests must include this key in the Authorization header.

## Table of Contents

- [Authentication Overview](#authentication-overview)
- [Setting Up Authentication](#setting-up-authentication)
- [Using the API Key](#using-the-api-key)
- [Best Practices](#best-practices)

## Authentication Overview

Mindia is a single-tenant media processing system that uses a simple master API key for authentication. There is no user registration or login system - all authenticated requests use the same master key.

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

All API endpoints (except health checks) require authentication using the master API key.

### Header Format

All requests must include the master API key in the Authorization header:

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

### Authentication Validation

On each request, Mindia:
1. Extracts the token from the `Authorization` header
2. Verifies it matches the configured `MASTER_API_KEY`
3. Allows the request to proceed if valid

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

- [API Reference](api-reference.md) - Complete API documentation
- [Best Practices](best-practices.md) - Security and performance tips
- [Quick Start](quick-start.md) - Get started with Mindia

