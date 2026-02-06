# Images

Complete guide to uploading, managing, and serving images with Mindia.

## Table of Contents

- [Overview](#overview)
- [Upload Image](#upload-image)
- [List Images](#list-images)
- [Get Image Metadata](#get-image-metadata)
- [Download Image](#download-image)
- [Delete Image](#delete-image)
- [Storage Behavior](#storage-behavior)
- [Best Practices](#best-practices)

## Overview

Mindia provides a complete image management API with support for:

**Supported Formats**:
- JPEG (`.jpg`, `.jpeg`)
- PNG (`.png`)
- GIF (`.gif`)
- WebP (`.webp`)

**Features**:
- ✅ Automatic format validation
- ✅ Dimension extraction
- ✅ EXIF metadata removal (privacy)
- ✅ Virus scanning (optional)
- ✅ UUID-based naming (prevents collisions)
- ✅ S3 or local storage
- ✅ On-the-fly transformations (see [Image Transformations](image-transformations.md))

**Limits** (configurable):
- Max file size: 10MB (default)
- Max dimensions: Unlimited
- Min dimensions: 1x1 pixel

## Upload Image

Upload an image file to Mindia.

### Endpoint

```
POST /api/images
```

### Headers

```
Authorization: Bearer <token>
Content-Type: multipart/form-data
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `store` | string | `auto` | Storage behavior: `0` (24h), `1` (permanent), `auto` |

### Request Body

Multipart form data with a single `file` field.

### Response

**Status**: `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "550e8400-e29b-41d4-a716-446655440000.jpg",
  "original_filename": "vacation-photo.jpg",
  "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.jpg",
  "content_type": "image/jpeg",
  "file_size": 1048576,
  "width": 1920,
  "height": 1080,
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

### Examples

**curl**:

```bash
TOKEN="your-token"

# Upload with default settings
curl -X POST https://api.example.com/api/images \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@photo.jpg"

# Upload with permanent storage
curl -X POST "https://api.example.com/api/images?store=1" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@photo.jpg"

# Upload with temporary storage (24h)
curl -X POST "https://api.example.com/api/images?store=0" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@temp-image.png"
```

**JavaScript**:

```javascript
async function uploadImage(file) {
  const token = localStorage.getItem('token');
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch('https://api.example.com/api/images', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
    },
    body: formData,
  });

  if (!response.ok) {
    throw new Error(`Upload failed: ${response.status}`);
  }

  return await response.json();
}

// Usage
const fileInput = document.querySelector('input[type="file"]');
fileInput.addEventListener('change', async (e) => {
  const file = e.target.files[0];
  const image = await uploadImage(file);
  console.log('Uploaded:', image.url);
});
```

**React Hook**:

```tsx
import { useState } from 'react';

function useImageUpload() {
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function upload(file: File, storePermanently = true) {
    setUploading(true);
    setError(null);

    try {
      const token = localStorage.getItem('token');
      const formData = new FormData();
      formData.append('file', file);

      const storeParam = storePermanently ? '1' : '0';
      const response = await fetch(
        `https://api.example.com/api/images?store=${storeParam}`,
        {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${token}`,
          },
          body: formData,
        }
      );

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Upload failed');
      }

      return await response.json();
    } catch (err) {
      setError(err.message);
      throw err;
    } finally {
      setUploading(false);
    }
  }

  return { upload, uploading, error };
}

// Usage in component
function ImageUploader() {
  const { upload, uploading, error } = useImageUpload();
  const [image, setImage] = useState(null);

  async function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      const uploadedImage = await upload(file);
      setImage(uploadedImage);
    } catch (err) {
      console.error('Upload failed:', err);
    }
  }

  return (
    <div>
      <input
        type="file"
        accept="image/*"
        onChange={handleFileChange}
        disabled={uploading}
      />
      {uploading && <p>Uploading...</p>}
      {error && <p>Error: {error}</p>}
      {image && <img src={image.url} alt="Uploaded" />}
    </div>
  );
}
```

### Errors

**400 Bad Request** - Invalid file type:
```json
{
  "error": "File type 'image/bmp' is not allowed"
}
```

**413 Payload Too Large** - File too large:
```json
{
  "error": "File size exceeds maximum of 10MB"
}
```

**422 Unprocessable Entity** - Virus detected:
```json
{
  "error": "File contains malware and was rejected"
}
```

## List Images

Retrieve a paginated list of images.

### Endpoint

```
GET /api/images
```

### Headers

```
Authorization: Bearer <token>
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | `50` | Number of results (1-100) |
| `offset` | integer | `0` | Number to skip (for pagination) |

### Response

**Status**: `200 OK`

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "filename": "550e8400-e29b-41d4-a716-446655440000.jpg",
    "original_filename": "photo1.jpg",
    "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.jpg",
    "content_type": "image/jpeg",
    "file_size": 1048576,
    "width": 1920,
    "height": 1080,
    "uploaded_at": "2024-01-01T00:00:00Z"
  },
  {
    "id": "660e9500-f30c-52e5-b827-557766551111",
    "filename": "660e9500-f30c-52e5-b827-557766551111.png",
    "original_filename": "screenshot.png",
    "url": "https://bucket.s3.amazonaws.com/uploads/660e9500-f30c-52e5-b827-557766551111.png",
    "content_type": "image/png",
    "file_size": 2097152,
    "width": 2560,
    "height": 1440,
    "uploaded_at": "2024-01-01T01:00:00Z"
  }
]
```

### Examples

```bash
# Get first 50 images
curl https://api.example.com/api/images \
  -H "Authorization: Bearer $TOKEN"

# Get next 50 images
curl "https://api.example.com/api/images?limit=50&offset=50" \
  -H "Authorization: Bearer $TOKEN"

# Get 10 most recent images
curl "https://api.example.com/api/images?limit=10&offset=0" \
  -H "Authorization: Bearer $TOKEN"
```

```javascript
async function fetchImages(page = 1, perPage = 50) {
  const token = localStorage.getItem('token');
  const offset = (page - 1) * perPage;

  const response = await fetch(
    `https://api.example.com/api/images?limit=${perPage}&offset=${offset}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return await response.json();
}

// Paginated gallery
async function loadImageGallery() {
  let page = 1;
  let hasMore = true;

  while (hasMore) {
    const images = await fetchImages(page, 50);
    
    if (images.length < 50) {
      hasMore = false;
    }

    displayImages(images);
    page++;
  }
}
```

## Get Image Metadata

Retrieve metadata for a specific image.

### Endpoint

```
GET /api/images/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "550e8400-e29b-41d4-a716-446655440000.jpg",
  "original_filename": "vacation-photo.jpg",
  "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.jpg",
  "content_type": "image/jpeg",
  "file_size": 1048576,
  "width": 1920,
  "height": 1080,
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

### Examples

```bash
IMAGE_ID="550e8400-e29b-41d4-a716-446655440000"

curl https://api.example.com/api/images/$IMAGE_ID \
  -H "Authorization: Bearer $TOKEN"
```

```javascript
async function getImageMetadata(imageId) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/images/${imageId}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  if (response.status === 404) {
    throw new Error('Image not found');
  }

  return await response.json();
}
```

### Errors

**404 Not Found**:
```json
{
  "error": "Image not found"
}
```

## Download Image

Download the original image file.

### Endpoint

```
GET /api/images/:id/file
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `200 OK`  
**Content-Type**: `image/jpeg`, `image/png`, etc.  
**Body**: Raw image bytes

### Examples

```bash
# Download to file
curl https://api.example.com/api/images/$IMAGE_ID/file \
  -H "Authorization: Bearer $TOKEN" \
  -o downloaded-image.jpg

# Display inline (if curl supports it)
curl https://api.example.com/api/images/$IMAGE_ID/file \
  -H "Authorization: Bearer $TOKEN" \
  --output -
```

```javascript
async function downloadImage(imageId, filename) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/images/${imageId}/file`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  const blob = await response.blob();
  const url = window.URL.createObjectURL(blob);
  
  // Trigger download
  const a = document.createElement('a');
  a.href = url;
  a.download = filename || `image-${imageId}.jpg`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  window.URL.revokeObjectURL(url);
}
```

## Delete Image

Delete an image and its S3 storage.

### Endpoint

```
DELETE /api/images/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `204 No Content`  
**Body**: Empty

### Examples

```bash
curl -X DELETE https://api.example.com/api/images/$IMAGE_ID \
  -H "Authorization: Bearer $TOKEN"
```

```javascript
async function deleteImage(imageId) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/images/${imageId}`,
    {
      method: 'DELETE',
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  if (!response.ok) {
    throw new Error('Delete failed');
  }

  return true;
}

// With confirmation
async function deleteImageWithConfirm(imageId, imageName) {
  if (!confirm(`Delete "${imageName}"?`)) {
    return false;
  }

  try {
    await deleteImage(imageId);
    alert('Image deleted successfully');
    return true;
  } catch (error) {
    alert('Failed to delete image');
    return false;
  }
}
```

### Errors

**404 Not Found**:
```json
{
  "error": "Image not found"
}
```

## Storage Behavior

Control how long images are stored using the `?store` parameter.

### Options

| Value | Behavior | Use Case |
|-------|----------|----------|
| `auto` | Uses `AUTO_STORE_ENABLED` config (default: permanent) | Default behavior |
| `1` | Permanent storage | Production images, important assets |
| `0` | Delete after 24 hours | Temporary files, previews, uploads in progress |

### Examples

```javascript
// Permanent storage (user profile pictures)
await uploadImage(file, '?store=1');

// Temporary storage (image previews before final upload)
await uploadImage(file, '?store=0');

// Auto (respects server config)
await uploadImage(file); // or ?store=auto
```

### Auto-Cleanup

Images with `store=0` are automatically deleted 24 hours after upload. A background job runs hourly to clean up expired files.

## Best Practices

### 1. Validate Files Client-Side

```javascript
function validateImage(file) {
  // Check file type
  const allowedTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp'];
  if (!allowedTypes.includes(file.type)) {
    throw new Error('Invalid file type. Please upload JPEG, PNG, GIF, or WebP.');
  }

  // Check file size (10MB)
  const maxSize = 10 * 1024 * 1024;
  if (file.size > maxSize) {
    throw new Error('File too large. Maximum size is 10MB.');
  }

  return true;
}
```

### 2. Show Upload Progress

```javascript
async function uploadWithProgress(file, onProgress) {
  return new Promise((resolve, reject) => {
    const token = localStorage.getItem('token');
    const formData = new FormData();
    formData.append('file', file);

    const xhr = new XMLHttpRequest();

    xhr.upload.addEventListener('progress', (e) => {
      if (e.lengthComputable) {
        const percentComplete = (e.loaded / e.total) * 100;
        onProgress(percentComplete);
      }
    });

    xhr.addEventListener('load', () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        resolve(JSON.parse(xhr.responseText));
      } else {
        reject(new Error(`Upload failed: ${xhr.status}`));
      }
    });

    xhr.addEventListener('error', () => {
      reject(new Error('Upload failed'));
    });

    xhr.open('POST', 'https://api.example.com/api/images');
    xhr.setRequestHeader('Authorization', `Bearer ${token}`);
    xhr.send(formData);
  });
}
```

### 3. Handle Errors Gracefully

```javascript
async function safeUpload(file) {
  try {
    // Validate client-side
    validateImage(file);

    // Upload
    const image = await uploadImage(file);
    return { success: true, image };

  } catch (error) {
    if (error.message.includes('413')) {
      return { success: false, error: 'File too large' };
    } else if (error.message.includes('422')) {
      return { success: false, error: 'Virus detected in file' };
    } else if (error.message.includes('401')) {
      return { success: false, error: 'Please log in again' };
    } else {
      return { success: false, error: 'Upload failed. Please try again.' };
    }
  }
}
```

### 4. Use Transformations for Display

Don't download full images for thumbnails - use transformations:

```javascript
// ❌ Bad: Download 5MB original for thumbnail
const thumb = image.url;

// ✅ Good: Request 200px thumbnail
const thumb = `${image.url.replace('/uploads/', '/api/images/')
  .replace(image.filename, `${image.id}/-/resize/200x/`)}`;
```

See [Image Transformations](image-transformations.md) for details.

### 5. Implement Retry Logic

```javascript
async function uploadWithRetry(file, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await uploadImage(file);
    } catch (error) {
      if (attempt === maxRetries) {
        throw error;
      }

      // Exponential backoff
      const delay = Math.pow(2, attempt) * 1000;
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
}
```

## Next Steps

- [Image Transformations](image-transformations.md) - Resize and transform images on-the-fly
- [Best Practices](best-practices.md) - CDN setup and performance optimization
- [API Reference](api-reference.md) - Complete endpoint reference

