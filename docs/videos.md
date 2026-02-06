# Videos

Complete guide to uploading, transcoding, and streaming videos with Mindia's HLS (HTTP Live Streaming) system.

## Table of Contents

- [Overview](#overview)
- [Upload Video](#upload-video)
- [Get Video Metadata](#get-video-metadata)
- [List Videos](#list-videos)
- [Stream Video (HLS)](#stream-video-hls)
- [Delete Video](#delete-video)
- [Video Processing](#video-processing)
- [Client Integration](#client-integration)
- [Best Practices](#best-practices)

## Overview

Mindia provides automatic HLS transcoding for adaptive bitrate video streaming.

**Supported Formats**:
- MP4 (`.mp4`) - H.264/H.265
- MOV (`.mov`) - QuickTime
- AVI (`.avi`)
- WebM (`.webm`)
- MKV (`.mkv`) - Matroska

**Features**:
- ✅ Automatic HLS transcoding
- ✅ Multiple quality variants (360p, 480p, 720p, 1080p)
- ✅ Hardware acceleration (NVENC, QSV, VideoToolbox)
- ✅ Async background processing
- ✅ Progress tracking
- ✅ Adaptive bitrate streaming

**Limits** (configurable):
- Max file size: 500MB (default)
- Supported codecs: H.264, H.265, VP8, VP9

## Upload Video

Upload a video file for HLS transcoding.

### Endpoint

```
POST /api/videos
```

### Headers

```
Authorization: Bearer <token>
Content-Type: multipart/form-data
```

### Request Body

Multipart form data with a single `file` field.

### Response

**Status**: `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "video.mp4",
  "original_filename": "my-video.mp4",
  "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.mp4",
  "content_type": "video/mp4",
  "file_size": 52428800,
  "duration": null,
  "width": null,
  "height": null,
  "processing_status": "pending",
  "hls_url": null,
  "variants": null,
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

**Note**: Video is uploaded immediately but transcoding happens asynchronously. Poll the metadata endpoint to check processing status.

### Examples

```bash
TOKEN="your-token"

# Upload video
curl -X POST https://api.example.com/api/videos \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@video.mp4"
```

```javascript
async function uploadVideo(file) {
  const token = localStorage.getItem('token');
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch('https://api.example.com/api/videos', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
    },
    body: formData,
  });

  if (!response.ok) {
    throw new Error('Upload failed');
  }

  return await response.json();
}

// Usage with progress monitoring
async function uploadAndWaitForProcessing(file) {
  // Upload
  const video = await uploadVideo(file);
  console.log('Uploaded:', video.id);

  // Poll for completion
  while (true) {
    const status = await getVideoMetadata(video.id);
    
    if (status.processing_status === 'completed') {
      console.log('Processing complete!', status.hls_url);
      return status;
    } else if (status.processing_status === 'failed') {
      throw new Error('Processing failed');
    }

    // Wait 2 seconds before next poll
    await new Promise(resolve => setTimeout(resolve, 2000));
  }
}
```

## Get Video Metadata

Retrieve video metadata and processing status.

### Endpoint

```
GET /api/videos/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response (Processing)

**Status**: `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "video.mp4",
  "processing_status": "processing",
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

### Response (Completed)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "video.mp4",
  "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.mp4",
  "content_type": "video/mp4",
  "file_size": 52428800,
  "duration": 120.5,
  "width": 1920,
  "height": 1080,
  "processing_status": "completed",
  "hls_url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000/master.m3u8",
  "variants": [
    {
      "name": "720p",
      "resolution": "1280x720",
      "bitrate": 2800,
      "width": 1280,
      "height": 720,
      "playlist_path": "720p/index.m3u8"
    },
    {
      "name": "1080p",
      "resolution": "1920x1080",
      "bitrate": 5000,
      "width": 1920,
      "height": 1080,
      "playlist_path": "1080p/index.m3u8"
    }
  ],
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

### Processing Status Values

| Status | Description |
|--------|-------------|
| `pending` | Waiting for transcoding to start |
| `processing` | Currently being transcoded |
| `completed` | Ready for streaming |
| `failed` | Transcoding failed (check logs) |

### Examples

```javascript
async function getVideoMetadata(videoId) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/videos/${videoId}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return await response.json();
}

// Poll until ready
async function waitForVideo(videoId, maxWait = 600000) {
  const startTime = Date.now();
  
  while (Date.now() - startTime < maxWait) {
    const video = await getVideoMetadata(videoId);
    
    if (video.processing_status === 'completed') {
      return video;
    } else if (video.processing_status === 'failed') {
      throw new Error('Video processing failed');
    }

    await new Promise(resolve => setTimeout(resolve, 2000));
  }

  throw new Error('Timeout waiting for video processing');
}
```

## List Videos

Retrieve a paginated list of videos.

### Endpoint

```
GET /api/videos
```

### Headers

```
Authorization: Bearer <token>
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | `50` | Number of results (1-100) |
| `offset` | integer | `0` | Number to skip |

### Response

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "filename": "video1.mp4",
    "processing_status": "completed",
    "hls_url": "https://bucket.s3.amazonaws.com/.../master.m3u8",
    "uploaded_at": "2024-01-01T00:00:00Z"
  }
]
```

## Stream Video (HLS)

Access HLS streaming endpoints.

### Master Playlist

```
GET /api/videos/:id/stream/master.m3u8
```

Returns the master playlist with all available quality variants.

### Variant Playlist

```
GET /api/videos/:id/stream/:variant/index.m3u8
```

Returns the playlist for a specific quality (e.g., `720p`, `1080p`).

### Segment

```
GET /api/videos/:id/stream/:variant/:segment
```

Returns a specific video segment (e.g., `segment_000.ts`).

### Examples

```html
<!-- HTML5 Video with HLS.js -->
<video id="video" controls></video>

<script src="https://cdn.jsdelivr.net/npm/hls.js@latest"></script>
<script>
  const video = document.getElementById('video');
  const hls_url = 'https://api.example.com/api/videos/550e8400.../stream/master.m3u8';

  if (Hls.isSupported()) {
    const hls = new Hls();
    hls.loadSource(hls_url);
    hls.attachMedia(video);
    
    hls.on(Hls.Events.MANIFEST_PARSED, () => {
      video.play();
    });
  } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
    // Native HLS support (Safari)
    video.src = hls_url;
    video.addEventListener('loadedmetadata', () => {
      video.play();
    });
  }
</script>
```

## Delete Video

Delete a video and all its HLS variants.

### Endpoint

```
DELETE /api/videos/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `204 No Content`

### Example

```bash
curl -X DELETE https://api.example.com/api/videos/$VIDEO_ID \
  -H "Authorization: Bearer $TOKEN"
```

## Video Processing

### Quality Variants

Videos are transcoded into multiple quality variants based on source resolution:

| Variant | Resolution | Bitrate | Generated When |
|---------|-----------|---------|----------------|
| 360p | 640x360 | 800 Kbps | Source ≥ 360p |
| 480p | 854x480 | 1400 Kbps | Source ≥ 480p |
| 720p | 1280x720 | 2800 Kbps | Source ≥ 720p |
| 1080p | 1920x1080 | 5000 Kbps | Source ≥ 1080p |

Only variants at or below the source resolution are generated.

### Processing Time

Approximate transcoding times (software encoding):
- 1 minute 1080p video: ~2-4 minutes
- 1 minute 720p video: ~1-2 minutes
- 1 minute 480p video: ~30-60 seconds

With hardware acceleration: 2-5x faster

### Hardware Acceleration

FFmpeg automatically detects and uses:
- **NVENC** (NVIDIA GPUs) - Best performance
- **QSV** (Intel Quick Sync) - Good performance
- **VideoToolbox** (Apple Silicon) - Good performance
- **Software** (fallback) - Slower but works everywhere

## Client Integration

### React Video Player

```tsx
import { useEffect, useRef, useState } from 'react';
import Hls from 'hls.js';

interface VideoPlayerProps {
  videoId: string;
}

function VideoPlayer({ videoId }: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [video, setVideo] = useState<any>(null);

  useEffect(() => {
    loadVideo();
  }, [videoId]);

  useEffect(() => {
    if (!video?.hls_url || !videoRef.current) return;

    const videoElement = videoRef.current;

    if (Hls.isSupported()) {
      const hls = new Hls();
      hls.loadSource(video.hls_url);
      hls.attachMedia(videoElement);

      hls.on(Hls.Events.ERROR, (event, data) => {
        if (data.fatal) {
          setError('Failed to load video');
        }
      });

      return () => hls.destroy();
    } else if (videoElement.canPlayType('application/vnd.apple.mpegurl')) {
      videoElement.src = video.hls_url;
    }
  }, [video]);

  async function loadVideo() {
    try {
      const token = localStorage.getItem('token');
      const response = await fetch(
        `https://api.example.com/api/videos/${videoId}`,
        {
          headers: { 'Authorization': `Bearer ${token}` },
        }
      );

      const videoData = await response.json();

      if (videoData.processing_status === 'completed') {
        setVideo(videoData);
        setLoading(false);
      } else if (videoData.processing_status === 'failed') {
        setError('Video processing failed');
        setLoading(false);
      } else {
        // Still processing, check again in 2 seconds
        setTimeout(loadVideo, 2000);
      }
    } catch (err) {
      setError('Failed to load video');
      setLoading(false);
    }
  }

  if (loading) return <div>Processing video...</div>;
  if (error) return <div>Error: {error}</div>;

  return (
    <video
      ref={videoRef}
      controls
      style={{ width: '100%', maxWidth: '800px' }}
    />
  );
}
```

### Vue Video Player

```vue
<template>
  <div>
    <div v-if="loading">Processing video...</div>
    <div v-else-if="error">Error: {{ error }}</div>
    <video
      v-else
      ref="videoEl"
      controls
      style="width: 100%; max-width: 800px"
    />
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, watch } from 'vue';
import Hls from 'hls.js';

const props = defineProps({
  videoId: String,
});

const videoEl = ref(null);
const loading = ref(true);
const error = ref(null);
const video = ref(null);
let hls = null;

onMounted(() => {
  loadVideo();
});

onUnmounted(() => {
  if (hls) {
    hls.destroy();
  }
});

watch(() => video.value, (newVideo) => {
  if (!newVideo?.hls_url || !videoEl.value) return;

  if (Hls.isSupported()) {
    hls = new Hls();
    hls.loadSource(newVideo.hls_url);
    hls.attachMedia(videoEl.value);
  } else if (videoEl.value.canPlayType('application/vnd.apple.mpegurl')) {
    videoEl.value.src = newVideo.hls_url;
  }
});

async function loadVideo() {
  try {
    const token = localStorage.getItem('token');
    const response = await fetch(
      `https://api.example.com/api/videos/${props.videoId}`,
      {
        headers: { 'Authorization': `Bearer ${token}` },
      }
    );

    const videoData = await response.json();

    if (videoData.processing_status === 'completed') {
      video.value = videoData;
      loading.value = false;
    } else if (videoData.processing_status === 'failed') {
      error.value = 'Video processing failed';
      loading.value = false;
    } else {
      setTimeout(loadVideo, 2000);
    }
  } catch (err) {
    error.value = 'Failed to load video';
    loading.value = false;
  }
}
</script>
```

## Best Practices

### 1. Show Processing Status

```javascript
function VideoStatus({ status }) {
  const statusInfo = {
    pending: { text: 'Waiting to process...', color: 'gray' },
    processing: { text: 'Processing video...', color: 'blue' },
    completed: { text: 'Ready to stream', color: 'green' },
    failed: { text: 'Processing failed', color: 'red' },
  };

  const info = statusInfo[status] || statusInfo.pending;

  return (
    <div className={`status-${info.color}`}>
      {info.text}
    </div>
  );
}
```

### 2. Implement Retry Logic

```javascript
async function uploadVideoWithRetry(file, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await uploadVideo(file);
    } catch (error) {
      if (attempt === maxRetries) throw error;
      
      const delay = Math.pow(2, attempt) * 1000;
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
}
```

### 3. Use CDN for Streaming

Always use a CDN (CloudFront, Cloudflare) in front of HLS endpoints:

```javascript
// ❌ Bad: Direct API calls for streaming
const hlsUrl = 'https://api.example.com/api/videos/.../master.m3u8';

// ✅ Good: CDN-fronted URLs
const hlsUrl = 'https://cdn.example.com/api/videos/.../master.m3u8';
```

### 4. Handle Errors Gracefully

```javascript
async function safeLoadVideo(videoId) {
  try {
    const video = await getVideoMetadata(videoId);
    
    if (video.processing_status === 'failed') {
      showError('Video processing failed. Please try uploading again.');
      return null;
    }

    return video;
  } catch (error) {
    if (error.message.includes('404')) {
      showError('Video not found');
    } else {
      showError('Failed to load video');
    }
    return null;
  }
}
```

### 5. Optimize Upload Size

```javascript
// Compress before upload if needed
async function compressAndUpload(file) {
  if (file.size > 100 * 1024 * 1024) { // > 100MB
    alert('Video is very large. Consider compressing it first.');
  }

  return await uploadVideo(file);
}
```

## Next Steps

- [Audio](audio.md) - Audio file management
- [Documents](documents.md) - PDF document storage
- [Best Practices](best-practices.md) - CDN setup and optimization
- [API Reference](api-reference.md) - Complete endpoint reference

