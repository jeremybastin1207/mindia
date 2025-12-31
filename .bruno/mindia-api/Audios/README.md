# Audio Endpoints

This collection contains API endpoints for managing audio files in Mindia.

## Overview

The audio API allows you to:
- Upload audio files (mp3, m4a, wav, flac, ogg)
- List uploaded audios with pagination
- Retrieve audio metadata by ID
- Download audio files
- Delete audio files

## Supported Audio Formats

- **MP3** (audio/mpeg)
- **M4A** (audio/mp4, audio/x-m4a)
- **WAV** (audio/wav)
- **FLAC** (audio/flac)
- **OGG** (audio/ogg)

## Maximum File Size

Default: 100 MB (configurable via `MAX_AUDIO_SIZE_MB` environment variable)

## Audio Metadata

When you upload an audio file, the system automatically extracts metadata using ffprobe:
- **Duration**: Length of the audio in seconds
- **Bitrate**: Audio bitrate in kbps
- **Sample Rate**: Audio sample rate in Hz
- **Channels**: Number of audio channels (e.g., 1 for mono, 2 for stereo)

## Storage Behavior

Audio files support three storage modes via the `store` query parameter:

1. **Temporary (`store=0`)**: File expires after 24 hours
2. **Permanent (`store=1`)**: File is stored indefinitely
3. **Auto (`store=auto`)**: Uses server default setting (default)

## Semantic Search

If semantic search is enabled on the server, audio files are automatically:
- Analyzed for content
- Indexed with embeddings
- Made searchable via the `/api/search` endpoint

## Example Workflow

1. **Upload an audio file**:
   ```bash
   curl -X POST "http://localhost:3000/api/audios?store=1" \
     -F "file=@podcast-episode.mp3"
   ```

2. **List all audios**:
   ```bash
   curl "http://localhost:3000/api/audios?limit=10&offset=0"
   ```

3. **Get audio metadata**:
   ```bash
   curl "http://localhost:3000/api/audios/{audio_id}"
   ```

4. **Download audio file**:
   ```bash
   curl "http://localhost:3000/api/audios/{audio_id}/file" -o audio.mp3
   ```

5. **Delete audio**:
   ```bash
   curl -X DELETE "http://localhost:3000/api/audios/{audio_id}"
   ```

## Testing

Before running these tests:
1. Ensure the server is running
2. Update the `base_url` environment variable
3. For Get/Download/Delete operations, replace `YOUR_AUDIO_ID_HERE` with an actual audio ID
4. For Upload, ensure you have a test audio file available

## Notes

- All endpoints support multi-tenancy (tenant isolation is automatic)
- ClamAV virus scanning is performed if enabled
- Webhooks are triggered for upload and delete events (if configured)
- Expired temporary files are cleaned up automatically by the cleanup service

