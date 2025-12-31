# Video API Endpoints

This folder contains Bruno requests for video upload, management, and HLS streaming.

## Available Endpoints

### Video Management
- **Upload Video** - Upload video file for HLS transcoding
- **List Videos** - Get all uploaded videos
- **Get Video by ID** - Get specific video metadata
- **Delete Video** - Delete video and all variants

### Video Streaming
- **Stream Master Playlist** - Get HLS master playlist for adaptive bitrate streaming
- **Stream Variant Playlist** - Get playlist for specific quality variant (360p, 480p, 720p, 1080p)
- **Stream Segment** - Get individual video segment file (.ts)

## Video Processing Workflow

1. **Upload** - Video is uploaded to S3
2. **Queued** - Video is queued for transcoding (status: `pending`)
3. **Processing** - FFmpeg transcodes to HLS variants (status: `processing`)
4. **Complete** - HLS variants ready for streaming (status: `completed`)
5. **Stream** - Use master playlist URL for adaptive bitrate streaming

## Processing Status

Videos have one of these statuses:
- `pending` - Uploaded, waiting for transcoding
- `processing` - Currently being transcoded
- `completed` - Ready for streaming
- `failed` - Transcoding failed

## HLS Streaming

Once a video has status `completed`, you can stream it using:

```html
<video controls>
  <source src="{{baseUrl}}/api/videos/{{videoId}}/stream/master.m3u8" 
          type="application/x-mpegURL">
</video>
```

### Supported Players
- Video.js (with HLS.js)
- HLS.js
- Native Safari/iOS
- Native Android (on supported devices)

### Quality Variants

The transcoder creates multiple quality levels:
- 1080p (if source is HD)
- 720p
- 480p
- 360p

The player automatically switches between variants based on network conditions.

## File Requirements

### Supported Formats
- MP4 (H.264/H.265)
- WebM
- MOV (QuickTime)
- AVI
- MKV

### Limits
- Maximum file size: Configured in server (typically 500MB-2GB)
- Supported codecs: H.264, H.265, VP8, VP9

## Security

Optional ClamAV virus scanning is performed on upload if enabled.

## Storage

Videos are stored in S3:
- Original video: `/videos/{uuid}.{ext}`
- HLS variants: `/videos/{uuid}/hls/{quality}/`
- Master playlist: `/videos/{uuid}/hls/master.m3u8`

## Background Processing

Video transcoding happens asynchronously in the background:
- Doesn't block the upload response
- Configurable concurrent transcode limit
- Uses FFmpeg for transcoding
- Creates HLS segments for adaptive streaming

