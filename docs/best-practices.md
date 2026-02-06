# Best Practices

Production-ready patterns and recommendations for using Mindia.

## CDN Setup

**Essential for Production**: Always use a CDN in front of Mindia.

### CloudFront Example

```yaml
# AWS CloudFront configuration
Origins:
  - DomainName: api.example.com
    Id: mindia-api
    CustomOriginConfig:
      HTTPPort: 80
      HTTPSPort: 443
      OriginProtocolPolicy: https-only

CacheBehaviors:
  # Cache transformed images heavily
  - PathPattern: /api/images/*/w_*
    TargetOriginId: mindia-api
    ViewerProtocolPolicy: redirect-to-https
    CachePolicyId: CachingOptimized
    
  # Cache video segments
  - PathPattern: /api/videos/*/stream/*
    TargetOriginId: mindia-api
    ViewerProtocolPolicy: redirect-to-https
    CachePolicyId: CachingOptimized

  # Don't cache API calls
  - PathPattern: /api/*
    TargetOriginId: mindia-api
    ViewerProtocolPolicy: redirect-to-https
    CachePolicyId: CachingDisabled
```

### Cloudflare Setup

1. Add your domain to Cloudflare
2. Create Page Rule for `/api/images/*/w_*`:
   - Cache Level: Cache Everything
   - Edge Cache TTL: 1 year
   - Browser Cache TTL: 1 year

3. Create Page Rule for `/api/videos/*/stream/*`:
   - Cache Level: Cache Everything
   - Edge Cache TTL: 1 day

## Performance Optimization

### Image Optimization

```javascript
// ✅ Good: Request appropriate size
function getThumbnail(imageId) {
  return `${CDN}/api/images/${imageId}/w_200/f_webp/q_low`;
}

function getFullImage(imageId) {
  return `${CDN}/api/images/${imageId}/w_1920/f_webp/q_high`;
}

// ❌ Bad: Always request full size
function getImage(imageId) {
  return image.url; // Original, unoptimized
}
```

### Responsive Images

```html
<img
  srcset="
    /api/images/{id}/w_400/f_webp 400w,
    /api/images/{id}/w_800/f_webp 800w,
    /api/images/{id}/w_1200/f_webp 1200w
  "
  sizes="(max-width: 600px) 400px, (max-width: 1200px) 800px, 1200px"
  src="/api/images/{id}/w_800/f_webp"
  loading="lazy"
  alt="..."
/>
```

### Lazy Loading

```javascript
// Intersection Observer for lazy loading
const observer = new IntersectionObserver((entries) => {
  entries.forEach(entry => {
    if (entry.isIntersecting) {
      const img = entry.target;
      img.src = img.dataset.src;
      observer.unobserve(img);
    }
  });
});

document.querySelectorAll('img[data-src]').forEach(img => {
  observer.observe(img);
});
```

## Security

### Store Tokens Securely

```javascript
// ✅ Good: httpOnly cookie (server-side)
res.cookie('token', token, {
  httpOnly: true,
  secure: true,
  sameSite: 'strict',
  maxAge: 86400000
});

// ✅ Acceptable: localStorage (client-side SPA)
localStorage.setItem('token', token);

// ❌ Bad: URL parameters
window.location.href = `/dashboard?token=${token}`;
```

### CORS Configuration

```env
# Development
CORS_ORIGINS=http://localhost:3000,http://localhost:5173

# Production (specific origins only)
CORS_ORIGINS=https://example.com,https://app.example.com

# ❌ Never in production
CORS_ORIGINS=*
```

### HTTPS Only

```javascript
// ✅ Always use HTTPS in production
const API_URL = 'https://api.example.com';

// ❌ Never use HTTP
const API_URL = 'http://api.example.com';
```

## Scalability

### Pagination

```javascript
// ✅ Good: Paginate large lists
async function loadAllImages() {
  let offset = 0;
  const limit = 50;
  const allImages = [];

  while (true) {
    const batch = await client.listImages(limit, offset);
    allImages.push(...batch);

    if (batch.length < limit) break;
    offset += limit;
  }

  return allImages;
}

// ❌ Bad: Request everything at once
const images = await client.listImages(10000, 0);
```

### Batch Operations

```javascript
// ✅ Good: Batch deletes
async function deleteMultiple(ids) {
  await Promise.all(
    ids.map(id => client.deleteImage(id))
  );
}

// Limit concurrency
async function deleteBatches(ids, batchSize = 10) {
  for (let i = 0; i < ids.length; i += batchSize) {
    const batch = ids.slice(i, i + batchSize);
    await Promise.all(batch.map(id => client.deleteImage(id)));
  }
}
```

### Caching

```javascript
class ResponseCache {
  constructor(ttl = 300000) { // 5 minutes
    this.cache = new Map();
    this.ttl = ttl;
  }

  get(key) {
    const item = this.cache.get(key);
    if (item && Date.now() - item.timestamp < this.ttl) {
      return item.data;
    }
    return null;
  }

  set(key, data) {
    this.cache.set(key, {
      data,
      timestamp: Date.now()
    });
  }

  clear() {
    this.cache.clear();
  }
}

const cache = new ResponseCache();

async function getCachedImages() {
  const cached = cache.get('images');
  if (cached) return cached;

  const images = await client.listImages();
  cache.set('images', images);
  return images;
}
```

## Error Handling

### Retry Logic

```javascript
async function withRetry(fn, maxAttempts = 3) {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (error) {
      if (attempt === maxAttempts) throw error;
      
      const delay = Math.min(1000 * Math.pow(2, attempt), 10000);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
}
```

### Graceful Degradation

```javascript
async function loadImagesWithFallback() {
  try {
    return await client.listImages();
  } catch (error) {
    console.error('Failed to load images:', error);
    // Return cached data or empty array
    return getCachedImages() || [];
  }
}
```

## Monitoring

### Track Errors

```javascript
function logError(error, context) {
  // Log to console
  console.error('Error:', error, 'Context:', context);

  // Send to monitoring service
  if (window.Sentry) {
    Sentry.captureException(error, { extra: context });
  }

  // Track in analytics
  if (window.gtag) {
    gtag('event', 'exception', {
      description: error.message,
      fatal: false
    });
  }
}
```

### Performance Monitoring

```javascript
async function measureUpload(file) {
  const start = performance.now();

  try {
    const result = await client.uploadImage(file);
    const duration = performance.now() - start;

    // Log metric
    console.log(`Upload took ${duration}ms`);

    // Track in analytics
    if (window.gtag) {
      gtag('event', 'timing_complete', {
        name: 'image_upload',
        value: Math.round(duration),
        event_category: 'media'
      });
    }

    return result;
  } catch (error) {
    logError(error, { file: file.name, size: file.size });
    throw error;
  }
}
```

## Production Checklist

### Before Launch

- [ ] **CDN configured** for image/video delivery
- [ ] **HTTPS enabled** everywhere
- [ ] **CORS** set to specific origins (not `*`)
- [ ] **Authentication** implemented
- [ ] **Error tracking** setup (Sentry, etc.)
- [ ] **Analytics** configured
- [ ] **Rate limiting** understood and handled
- [ ] **Monitoring** in place
- [ ] **Backup strategy** defined
- [ ] **Security review** completed

### Environment Variables

```env
# ✅ Production settings
ENVIRONMENT=production
CORS_ORIGINS=https://example.com
OTEL_ENABLED=true
CLAMAV_ENABLED=true
REMOVE_EXIF=true
AUTO_STORE_ENABLED=false
RUST_LOG=mindia=info
```

### Performance Targets

- Image upload: < 2s for 5MB file
- Image transformation: < 200ms (CDN cold)
- Image transformation: < 50ms (CDN warm)
- Video upload: < 5s for 50MB file
- API response time: < 100ms (p95)
- Search query: < 500ms

## Backup & Recovery

### Regular Backups

```bash
# Database backup (daily)
pg_dump $DATABASE_URL > backup_$(date +%Y%m%d).sql

# S3 versioning (enable in bucket settings)
aws s3api put-bucket-versioning \
  --bucket your-bucket \
  --versioning-configuration Status=Enabled
```

### Disaster Recovery

1. **Database**: Neon has automatic backups
2. **S3**: Enable versioning and cross-region replication
3. **Configuration**: Store in version control
4. **Secrets**: Use secret management service

## Next Steps

- [Quick Start](quick-start.md) - Get started quickly
- [Configuration](configuration.md) - Environment variables
- [API Reference](api-reference.md) - Complete API docs

