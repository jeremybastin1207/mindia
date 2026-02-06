# Audio

Complete guide to uploading, managing, and serving audio files with Mindia.

## Table of Contents

- [Overview](#overview)
- [Upload Audio](#upload-audio)
- [List Audio Files](#list-audio-files)
- [Get Audio Metadata](#get-audio-metadata)
- [Download Audio](#download-audio)
- [Delete Audio](#delete-audio)
- [Best Practices](#best-practices)

## Overview

Mindia provides audio file management with automatic metadata extraction and semantic search indexing.

**Supported Formats**:
- MP3 (`.mp3`) - MPEG Audio Layer 3
- M4A (`.m4a`) - AAC Audio
- WAV (`.wav`) - Waveform Audio
- FLAC (`.flac`) - Free Lossless Audio Codec
- OGG (`.ogg`) - Ogg Vorbis

**Features**:
- ✅ Automatic metadata extraction (duration, bitrate, sample rate, channels)
- ✅ Semantic search indexing
- ✅ Storage options (temporary or permanent)
- ✅ S3 or local storage
- ✅ UUID-based naming

**Limits** (configurable):
- Max file size: 100MB (default)
- Supported sample rates: Any
- Supported channels: Mono, Stereo, Surround

## Upload Audio

Upload an audio file to Mindia.

### Endpoint

```
POST /api/audios
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
  "filename": "550e8400-e29b-41d4-a716-446655440000.mp3",
  "original_filename": "podcast-episode-01.mp3",
  "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.mp3",
  "content_type": "audio/mpeg",
  "file_size": 5242880,
  "duration": 300.5,
  "bitrate": 128000,
  "sample_rate": 44100,
  "channels": 2,
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

### Examples

```bash
TOKEN="your-token"

# Upload audio
curl -X POST https://api.example.com/api/audios \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@podcast.mp3"

# Upload with permanent storage
curl -X POST "https://api.example.com/api/audios?store=1" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@music.mp3"
```

```javascript
async function uploadAudio(file, permanent = true) {
  const token = localStorage.getItem('token');
  const formData = new FormData();
  formData.append('file', file);

  const storeParam = permanent ? '1' : '0';
  const response = await fetch(
    `https://api.example.com/api/audios?store=${storeParam}`,
    {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
      },
      body: formData,
    }
  );

  if (!response.ok) {
    throw new Error('Upload failed');
  }

  return await response.json();
}

// Usage
const fileInput = document.querySelector('input[type="file"]');
fileInput.addEventListener('change', async (e) => {
  const file = e.target.files[0];
  const audio = await uploadAudio(file);
  console.log('Uploaded:', audio);
  console.log('Duration:', audio.duration, 'seconds');
  console.log('Bitrate:', audio.bitrate / 1000, 'kbps');
});
```

### Metadata Extraction

Mindia automatically extracts audio metadata:

- **Duration**: Length in seconds (e.g., `300.5`)
- **Bitrate**: Bits per second (e.g., `128000` = 128 kbps)
- **Sample Rate**: Hz (e.g., `44100` = 44.1 kHz)
- **Channels**: Number of audio channels (e.g., `2` = stereo)

## List Audio Files

Retrieve a paginated list of audio files.

### Endpoint

```
GET /api/audios
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
    "filename": "550e8400-e29b-41d4-a716-446655440000.mp3",
    "original_filename": "podcast-episode-01.mp3",
    "url": "https://bucket.s3.amazonaws.com/uploads/...",
    "content_type": "audio/mpeg",
    "file_size": 5242880,
    "duration": 300.5,
    "bitrate": 128000,
    "sample_rate": 44100,
    "channels": 2,
    "uploaded_at": "2024-01-01T00:00:00Z"
  }
]
```

### Examples

```javascript
async function fetchAudioFiles(page = 1, perPage = 50) {
  const token = localStorage.getItem('token');
  const offset = (page - 1) * perPage;

  const response = await fetch(
    `https://api.example.com/api/audios?limit=${perPage}&offset=${offset}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return await response.json();
}

// Display audio library
async function loadAudioLibrary() {
  const audios = await fetchAudioFiles();
  
  audios.forEach(audio => {
    const duration = formatDuration(audio.duration);
    const bitrate = Math.round(audio.bitrate / 1000);
    
    console.log(`${audio.original_filename} - ${duration} @ ${bitrate} kbps`);
  });
}

function formatDuration(seconds) {
  const minutes = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}
```

## Get Audio Metadata

Retrieve metadata for a specific audio file.

### Endpoint

```
GET /api/audios/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "550e8400-e29b-41d4-a716-446655440000.mp3",
  "original_filename": "podcast-episode-01.mp3",
  "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.mp3",
  "content_type": "audio/mpeg",
  "file_size": 5242880,
  "duration": 300.5,
  "bitrate": 128000,
  "sample_rate": 44100,
  "channels": 2,
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

## Download Audio

Download the audio file.

### Endpoint

```
GET /api/audios/:id/file
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `200 OK`  
**Content-Type**: `audio/mpeg`, `audio/mp4`, etc.  
**Body**: Raw audio bytes

### Examples

```bash
# Download to file
curl https://api.example.com/api/audios/$AUDIO_ID/file \
  -H "Authorization: Bearer $TOKEN" \
  -o downloaded-audio.mp3
```

```javascript
async function downloadAudio(audioId, filename) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/audios/${audioId}/file`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  const blob = await response.blob();
  const url = window.URL.createObjectURL(blob);
  
  const a = document.createElement('a');
  a.href = url;
  a.download = filename || `audio-${audioId}.mp3`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  window.URL.revokeObjectURL(url);
}
```

## Delete Audio

Delete an audio file and its S3 storage.

### Endpoint

```
DELETE /api/audios/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `204 No Content`

### Example

```bash
curl -X DELETE https://api.example.com/api/audios/$AUDIO_ID \
  -H "Authorization: Bearer $TOKEN"
```

```javascript
async function deleteAudio(audioId) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/audios/${audioId}`,
    {
      method: 'DELETE',
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return response.ok;
}
```

## Best Practices

### 1. Validate File Size

```javascript
function validateAudioFile(file) {
  const maxSize = 100 * 1024 * 1024; // 100MB
  
  if (file.size > maxSize) {
    throw new Error('File too large. Maximum size is 100MB.');
  }

  const allowedTypes = [
    'audio/mpeg',
    'audio/mp4',
    'audio/x-m4a',
    'audio/wav',
    'audio/flac',
    'audio/ogg',
  ];

  if (!allowedTypes.includes(file.type)) {
    throw new Error('Invalid file type. Please upload MP3, M4A, WAV, FLAC, or OGG.');
  }

  return true;
}
```

### 2. Display Audio Metadata

```javascript
function AudioMetadata({ audio }) {
  const duration = formatDuration(audio.duration);
  const bitrate = Math.round(audio.bitrate / 1000);
  const sampleRate = (audio.sample_rate / 1000).toFixed(1);
  const channelText = audio.channels === 1 ? 'Mono' : audio.channels === 2 ? 'Stereo' : `${audio.channels} channels`;

  return (
    <div className="audio-metadata">
      <h3>{audio.original_filename}</h3>
      <ul>
        <li>Duration: {duration}</li>
        <li>Bitrate: {bitrate} kbps</li>
        <li>Sample Rate: {sampleRate} kHz</li>
        <li>Channels: {channelText}</li>
        <li>Size: {formatFileSize(audio.file_size)}</li>
      </ul>
    </div>
  );
}

function formatDuration(seconds) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

function formatFileSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}
```

### 3. Create Audio Player

```javascript
function AudioPlayer({ audio }) {
  const [playing, setPlaying] = useState(false);
  const audioRef = useRef(null);

  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.src = audio.url;
    }
  }, [audio.url]);

  function togglePlay() {
    if (playing) {
      audioRef.current.pause();
    } else {
      audioRef.current.play();
    }
    setPlaying(!playing);
  }

  return (
    <div className="audio-player">
      <audio ref={audioRef} />
      <button onClick={togglePlay}>
        {playing ? 'Pause' : 'Play'}
      </button>
      <span>{audio.original_filename}</span>
      <span>{formatDuration(audio.duration)}</span>
    </div>
  );
}
```

### 4. Implement Search

Audio files are automatically indexed for semantic search:

```javascript
// Search for audio by description or content
const results = await fetch(
  'https://api.example.com/api/search?q=jazz+music&type=audio',
  {
    headers: { 'Authorization': `Bearer ${token}` },
  }
).then(r => r.json());

console.log('Found audio files:', results);
```

See [Semantic Search](semantic-search.md) for details.

### 5. Handle Upload Progress

```javascript
async function uploadAudioWithProgress(file, onProgress) {
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

    xhr.open('POST', 'https://api.example.com/api/audios');
    xhr.setRequestHeader('Authorization', `Bearer ${token}`);
    xhr.send(formData);
  });
}
```

## Next Steps

- [Documents](documents.md) - PDF document storage
- [Semantic Search](semantic-search.md) - Search audio by content
- [Best Practices](best-practices.md) - Optimization tips
- [API Reference](api-reference.md) - Complete endpoint reference

