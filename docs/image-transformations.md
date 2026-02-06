# Image Transformations

Transform and resize images on-the-fly with Mindia's URL-based transformation API.

## Overview

Mindia can resize and transform images without storing multiple versions. Simply modify the URL to get different sizes and formats instantly.

**Benefits**:
- ✅ No storage overhead (no duplicate files)
- ✅ Unlimited transformation combinations
- ✅ Cache-friendly (immutable URLs)
- ✅ CDN-ready

**Supported Operations**:
- Resize (width, height, or both)
- Format conversion (JPEG, PNG, WebP)
- Quality adjustment
- Stretch control (upscale behavior)

## URL Structure

```
/api/images/{id}/-/{operation}/{value}/-/...
```

**Operations Format**: URL format with `/-/` separators between operations.

**Examples**:
```
/api/images/{id}/-/resize/500x300/-/format/webp/                # 500x300, WebP format
/api/images/{id}/-/resize/500x/-/format/webp/                   # Width 500px, WebP format
/api/images/{id}/-/resize/500x300/-/format/webp/-/quality/high/ # 500x300, WebP, high quality
/api/images/{id}/-/resize/x300/-/format/webp/                   # Height 300px, WebP format
```

## Resize Operations

### Width Only

Resize to specific width, maintain aspect ratio.

```
/api/images/{id}/-/resize/500x/
```

**Example**:
```bash
curl "https://api.example.com/api/images/$IMAGE_ID/-/resize/320x/" \
  -H "Authorization: Bearer $TOKEN" \
  -o thumb.jpg
```

### Height Only

Resize to specific height, maintain aspect ratio.

```
/api/images/{id}/-/resize/x400/
```

### Both Dimensions

Specify both width and height.

```
/api/images/{id}/-/resize/500x300/
```

## Format Conversion

Convert image format using the `format` operation.

**Supported Formats**:
- `jpeg` or `jpg` - JPEG format
- `png` - PNG format
- `webp` - WebP format (smaller file size)
- `auto` - Automatically select best format based on Accept header

**Examples**:
```
/api/images/{id}/-/resize/500x/-/format/webp/    # 500px width, WebP format
/api/images/{id}/-/resize/500x/-/format/jpeg/    # 500px width, JPEG format
/api/images/{id}/-/resize/500x/-/format/png/     # 500px width, PNG format
/api/images/{id}/-/resize/500x/-/format/auto/    # Auto-detect format from Accept header
```

## Quality

Adjust output quality (affects file size) using the `quality` operation.

**Options**:
- `low` - Fast, smaller files (quality: 60)
- `medium` - Balanced (quality: 80)
- `high` - Best quality (quality: 95)
- `better` - Higher quality than medium
- `lighter` - Lower quality for smaller files
- `fast` - Fastest compression, lower quality

**Example**:
```
/api/images/{id}/-/resize/500x/-/quality/high/   # High quality
/api/images/{id}/-/resize/500x/-/quality/low/    # Smaller file size
/api/images/{id}/-/resize/500x/-/format/webp/-/quality/high/  # Combined operations
```

## Complete Examples

### Thumbnail

200px square, WebP, low quality:
```
/api/images/{id}/-/resize/200x200/-/format/webp/-/quality/low/
```

### Medium Preview

800px wide, maintain aspect ratio, high quality:
```
/api/images/{id}/-/resize/800x/-/quality/high/
```

### Optimized for Web

1200px max width, WebP format, medium quality:
```
/api/images/{id}/-/resize/1200x/-/format/webp/-/quality/medium/
```

## Client Integration

### JavaScript/TypeScript

```javascript
function getImageUrl(imageId, options = {}) {
  const {
    width,
    height,
    format = 'jpeg',
    quality = 'medium',
  } = options;

  const operations = [];
  
  // Build resize operation
  if (width && height) {
    operations.push(`resize/${width}x${height}`);
  } else if (width) {
    operations.push(`resize/${width}x`);
  } else if (height) {
    operations.push(`resize/x${height}`);
  }
  
  // Add format and quality
  if (format) operations.push(`format/${format}`);
  if (quality) operations.push(`quality/${quality}`);

  // Join with /-/ separators
  const path = operations.length > 0 ? `-/${operations.join('/-/')}/` : '';
  return `https://api.example.com/api/images/${imageId}/${path}`;
}

// Usage
const thumbUrl = getImageUrl(imageId, { width: 200, height: 200, format: 'webp', quality: 'low' });
// Returns: /api/images/{id}/-/resize/200x200/-/format/webp/-/quality/low/

const fullUrl = getImageUrl(imageId, { width: 1920, quality: 'high' });
// Returns: /api/images/{id}/-/resize/1920x/-/quality/high/
```

### React Component

```tsx
interface ImageProps {
  imageId: string;
  width?: number;
  height?: number;
  format?: 'jpeg' | 'png' | 'webp';
  quality?: 'low' | 'medium' | 'high';
  alt?: string;
}

function OptimizedImage({ imageId, width, height, format = 'webp', quality = 'medium', alt }: ImageProps) {
  const url = getImageUrl(imageId, { width, height, format, quality });

  return <img src={url} alt={alt} loading="lazy" />;
}

// Usage
<OptimizedImage imageId="uuid" width={300} height={200} format="webp" alt="Product photo" />
```

### Responsive Images

```tsx
function ResponsiveImage({ imageId, alt }: { imageId: string; alt: string }) {
  const srcSet = [
    `${getImageUrl(imageId, { width: 400, format: 'webp' })} 400w`,
    `${getImageUrl(imageId, { width: 800, format: 'webp' })} 800w`,
    `${getImageUrl(imageId, { width: 1200, format: 'webp' })} 1200w`,
  ].join(', ');

  return (
    <img
      srcSet={srcSet}
      sizes="(max-width: 600px) 400px, (max-width: 1200px) 800px, 1200px"
      src={getImageUrl(imageId, { width: 800, format: 'webp' })}
      alt={alt}
      loading="lazy"
    />
  );
}
```

## Best Practices

### 1. Use WebP Format

WebP provides better compression than JPEG/PNG:

```javascript
// ✅ Good: WebP for modern browsers
const url = getImageUrl(id, { width: 500, format: 'webp' });
// Returns: /api/images/{id}/-/resize/500x/-/format/webp/

// ❌ Less optimal: JPEG
const url = getImageUrl(id, { width: 500, format: 'jpeg' });
```

### 2. Choose Appropriate Quality

```javascript
// Thumbnails: low quality is fine
const thumb = getImageUrl(id, { width: 150, quality: 'low' });
// Returns: /api/images/{id}/-/resize/150x/-/quality/low/

// Hero images: use high quality
const hero = getImageUrl(id, { width: 1920, quality: 'high' });
// Returns: /api/images/{id}/-/resize/1920x/-/quality/high/

// Content images: medium is balanced
const content = getImageUrl(id, { width: 800, quality: 'medium' });
// Returns: /api/images/{id}/-/resize/800x/-/quality/medium/
```

### 3. Use CDN

Always use a CDN in front of transformation endpoints:

```javascript
// ✅ Good: CDN-fronted
const CDN_URL = 'https://cdn.example.com';
const url = `${CDN_URL}/api/images/${id}/-/resize/500x/-/format/webp/`;

// ❌ Bad: Direct API calls (slow, expensive)
const url = `https://api.example.com/api/images/${id}/-/resize/500x/-/format/webp/`;
```

### 4. Cache Transformations

Transformed images have cache headers set for 1 year:

```
Cache-Control: public, max-age=31536000, immutable
```

Browsers and CDNs will cache automatically.

### 5. Lazy Load Images

```html
<img src="{url}" loading="lazy" />
```

### 6. Provide Width/Height

Prevent layout shift:

```html
<img src="{url}" width="500" height="300" alt="..." />
```

## Performance

### Transformation Speed

- Small images (< 1MB): ~50-100ms
- Medium images (1-5MB): ~100-300ms
- Large images (> 5MB): ~300-1000ms

### With CDN

After first request (cache warm):
- Subsequent requests: ~10-50ms (CDN edge)

### Optimization Tips

1. **Pre-warm common sizes** - Request popular sizes on upload
2. **Use appropriate dimensions** - Don't request huge images for thumbnails
3. **Enable CDN** - Essential for production
4. **Use WebP** - Smaller files = faster downloads

## Troubleshooting

### Image Not Transforming

Check the URL format:
```
# ✅ Correct format
/api/images/{id}/-/resize/500x/

# ✅ Also supported (short format)
/api/images/{id}/resize/500x/

# ❌ Wrong
/api/images/{id}/w_500  # Short format
```

### Poor Quality

Increase quality setting:
```
/api/images/{id}/-/resize/500x/-/quality/high/
```

### Slow Performance

1. Enable CDN
2. Reduce image size requests
3. Use WebP format
4. Check origin image size

## Next Steps

- [Images](images.md) - Image upload and management
- [Best Practices](best-practices.md) - CDN setup and optimization
- [API Reference](api-reference.md) - Complete API docs

