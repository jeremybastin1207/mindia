# Audio Transformations

Current behavior and options for audio in Mindia.

## Current behavior

Audio files are **served as stored**. Mindia does not perform on-the-fly transformation or transcoding for audio. After upload, files are delivered at their original format and quality.

**What Mindia provides for audio:**

- Upload and storage (MP3, M4A, WAV, FLAC, OGG)
- Automatic metadata extraction (duration, bitrate, sample rate, channels)
- Semantic search indexing
- Direct download via the API

For full details on uploading, listing, and downloading audio, see [Audio](audio).

## Future

On-the-fly audio transformations (e.g. format conversion, bitrate variants, or clip extraction) may be added in future releases.
