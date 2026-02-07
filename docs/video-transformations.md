# Video Transformations

Video in Mindia is transformed via **HLS transcoding**: uploaded videos are converted into adaptive bitrate HLS streams with multiple quality variants.

## How it works

1. You upload a video (e.g. MP4, MOV, WebM).
2. Mindia transcodes it asynchronously into HLS (.m3u8 + segments).
3. The API returns an `hls_url` once processing completes. Use this URL for playback; players (e.g. hls.js) switch quality automatically.

No on-the-fly URL parameters for video—transcoding is a one-time background job per upload.

## Quality variants

Videos are transcoded into variants based on source resolution:

| Variant | Resolution | Bitrate | Generated when |
|---------|------------|---------|----------------|
| 360p | 640×360 | 800 Kbps | Source ≥ 360p |
| 480p | 854×480 | 1400 Kbps | Source ≥ 480p |
| 720p | 1280×720 | 2800 Kbps | Source ≥ 720p |
| 1080p | 1920×1080 | 5000 Kbps | Source ≥ 1080p |

Only variants at or below the source resolution are generated. Hardware acceleration (NVENC, QSV, VideoToolbox) is used when available.

## Full guide

For upload, metadata, `hls_url`, processing status, and client integration, see [Videos](videos).
