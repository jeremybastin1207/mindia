# Rate Limiting

Understanding and handling API rate limits in Mindia.

## Overview

Mindia implements rate limiting to ensure fair usage and protect server resources.

**Current Status**: Rate limiting configuration available but specific limits depend on your deployment configuration.

**When Enabled**:
- Limits per tenant/user
- Tracked by IP address
- Sliding window algorithm
- 429 status code when exceeded

## Response Headers

When rate limiting is active, responses include:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640000000
```

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Total requests allowed in window |
| `X-RateLimit-Remaining` | Requests remaining |
| `X-RateLimit-Reset` | Unix timestamp when limit resets |

## Rate Limited Response

**Status**: `429 Too Many Requests`

```json
{
  "error": "Too many requests. Please slow down."
}
```

**Headers**:
```
Retry-After: 60
```

## Client Implementation

### Respect Rate Limits

```javascript
async function apiCallWithRateLimit(url, options = {}) {
  const response = await fetch(url, options);

  // Check remaining requests
  const remaining = parseInt(response.headers.get('X-RateLimit-Remaining') || '999');
  
  if (remaining < 10) {
    console.warn(`Low rate limit: ${remaining} requests remaining`);
  }

  if (response.status === 429) {
    const retryAfter = parseInt(response.headers.get('Retry-After') || '60');
    throw new RateLimitError(`Rate limited. Retry after ${retryAfter}s`, retryAfter);
  }

  return response;
}

class RateLimitError extends Error {
  constructor(message, retryAfter) {
    super(message);
    this.retryAfter = retryAfter;
  }
}
```

### Automatic Retry with Backoff

```javascript
async function fetchWithRetry(url, options = {}, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const response = await apiCallWithRateLimit(url, options);
      return await response.json();
    } catch (error) {
      if (error instanceof RateLimitError) {
        if (attempt === maxRetries) {
          throw error;
        }

        // Wait for retry-after period
        await new Promise(resolve => 
          setTimeout(resolve, error.retryAfter * 1000)
        );
        
        continue;
      }

      throw error;
    }
  }
}
```

### Rate Limit Manager

```javascript
class RateLimitManager {
  constructor() {
    this.limit = null;
    this.remaining = null;
    this.resetTime = null;
  }

  updateFromHeaders(headers) {
    this.limit = parseInt(headers.get('X-RateLimit-Limit') || '0');
    this.remaining = parseInt(headers.get('X-RateLimit-Remaining') || '0');
    this.resetTime = parseInt(headers.get('X-RateLimit-Reset') || '0');
  }

  getStatus() {
    return {
      limit: this.limit,
      remaining: this.remaining,
      resetTime: this.resetTime,
      resetIn: this.resetTime ? this.resetTime - Math.floor(Date.now() / 1000) : null,
      percentUsed: this.limit ? ((this.limit - this.remaining) / this.limit) * 100 : 0,
    };
  }

  shouldWait() {
    return this.remaining !== null && this.remaining < 5;
  }

  async waitIfNeeded() {
    if (this.shouldWait()) {
      const waitTime = Math.max(0, this.resetTime - Math.floor(Date.now() / 1000));
      console.log(`Rate limit low. Waiting ${waitTime}s...`);
      await new Promise(resolve => setTimeout(resolve, waitTime * 1000));
    }
  }
}

// Usage
const rateLimiter = new RateLimitManager();

async function apiCall(url, options) {
  await rateLimiter.waitIfNeeded();

  const response = await fetch(url, options);
  rateLimiter.updateFromHeaders(response.headers);

  return response;
}
```

## Best Practices

### 1. Cache Responses

```javascript
const cache = new Map();

async function cachedApiCall(url, ttl = 60000) {
  const cached = cache.get(url);
  
  if (cached && Date.now() - cached.time < ttl) {
    return cached.data;
  }

  const data = await apiCall(url);
  cache.set(url, { data, time: Date.now() });

  return data;
}
```

### 2. Batch Requests

```javascript
// ❌ Bad: Many individual requests
for (const id of imageIds) {
  await apiCall(`/api/images/${id}`);
}

// ✅ Good: Batch request
const images = await apiCall('/api/images');
```

### 3. Implement Backoff

```javascript
async function exponentialBackoff(fn, maxRetries = 5) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (error.status !== 429 || i === maxRetries - 1) {
        throw error;
      }

      const delay = Math.min(1000 * Math.pow(2, i), 30000);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
}
```

### 4. Monitor Usage

```javascript
function displayRateLimitStatus() {
  const status = rateLimiter.getStatus();

  console.log(`Rate Limit: ${status.remaining}/${status.limit}`);
  console.log(`Resets in: ${status.resetIn}s`);
  console.log(`Used: ${status.percentUsed.toFixed(1)}%`);

  if (status.percentUsed > 80) {
    console.warn('High rate limit usage!');
  }
}
```

## Next Steps

- [Error Handling](error-handling.md) - Handle API errors
- [Best Practices](best-practices.md) - Optimization tips

