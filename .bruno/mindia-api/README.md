# Mindia API - Bruno Collection

This is a [Bruno](https://www.usebruno.com/) API collection for testing the Mindia image upload service.

## 📋 Contents

This collection includes requests for all Mindia API endpoints:

### Health
- **Health Check** - Verify server and database connectivity

### Images
- **Upload Image** - Upload image files (multipart/form-data) with storage behavior control
- **List Images** - Get all uploaded images (limit 50)
- **Get Image by ID** - Retrieve metadata by UUID (only access method)
- **Download File** - Download original image file
- **Delete Image** - Remove image from S3 and database

### Transformations
On-the-fly image transformations with caching:
- **Resize - Exact Dimensions** - Resize to specific width×height
- **Resize - Width Only** - Resize width, preserve aspect ratio
- **Resize - Height Only** - Resize height, preserve aspect ratio
- **Resize - No Stretch** - Prevent upscaling small images
- **Resize - Fill Background** - Fit in canvas with white background
- **Resize - Stretch On** - Allow upscaling (default behavior)

### Videos
Video upload, transcoding, and HLS streaming:
- **Upload Video** - Upload video for HLS transcoding (MP4, MOV, WebM, etc.) with storage behavior control
- **List Videos** - Get all uploaded videos with processing status
- **Get Video by ID** - Retrieve video metadata and HLS playlist URLs
- **Delete Video** - Remove video and all transcoded variants
- **Stream Master Playlist** - HLS master playlist for adaptive bitrate streaming
- **Stream Variant Playlist** - Quality-specific playlist (360p, 480p, 720p, 1080p)
- **Stream Segment** - Individual video segment files (.ts)

### Documents
Document upload and storage:
- **Upload Document** - Upload document files (PDF) with storage behavior control
- **List Documents** - Get paginated list of all documents
- **Get Document by ID** - Retrieve metadata for a specific document
- **Download Document** - Download the original PDF file
- **Delete Document** - Remove document from storage and database
- **Update Document Metadata** - Update custom metadata for a document

### Metadata
Custom metadata management:
- **Get Media Metadata** - Retrieve metadata for any media file (image, video, audio, or document)
- **Update Image Metadata** - Update custom metadata for an image
- **Update Video Metadata** - Update custom metadata for a video
- **Update Audio Metadata** - Update custom metadata for an audio file
- **Update Document Metadata** - Update custom metadata for a document

### Folders
Hierarchical folder organization:
- **Create Folder** - Create a new folder (root or nested)
- **List Folders** - List all folders or filter by parent
- **Get Folder Tree** - Get complete hierarchical folder structure
- **Get Folder by ID** - Retrieve folder details with counts
- **Update Folder** - Update folder name or move to different parent
- **Delete Folder** - Delete an empty folder
- **Move Media to Folder** - Move a media file to a folder

### Search
Semantic search capabilities:
- **Semantic Search** - Search media files using natural language queries

### Tasks
Background task management:
- **List Tasks** - List tasks with optional filters
- **Get Task by ID** - Retrieve detailed task information
- **Get Task Stats** - Get aggregated task statistics
- **Cancel Task** - Cancel a pending or scheduled task
- **Retry Task** - Retry a failed task

### File Groups
File grouping and archive downloads:
- **Create File Group** - Associate multiple files together
- **Get File Group** - Get full details with all files
- **Get File Group Info** - Get summary information
- **Get File by Index** - Get redirect to specific file in group
- **Download Archive - ZIP** - Download group as ZIP archive
- **Download Archive - TAR** - Download group as TAR archive
- **Delete File Group** - Remove file group association

### Plugins
Plugin management and execution:
- **List Plugins** - List all available plugins
- **Execute Plugin** - Execute a plugin on a media file
- **Get Plugin Config** - Retrieve plugin configuration
- **Update Plugin Config** - Update plugin settings

### Uploads
Advanced upload methods:
- **Start Chunked Upload** - Start chunked upload session for large files
- **Record Chunk Upload** - Record successful chunk upload
- **Complete Chunked Upload** - Complete chunked upload after all chunks uploaded
- **Get Chunked Upload Progress** - Check upload progress

### Analytics
Request analytics and storage metrics:
- **Get Traffic Summary** - Overall traffic stats, popular URLs, and request distribution
- **Get URL Statistics** - Detailed statistics per endpoint
- **Get Storage Summary** - Storage usage by content type (images/videos)
- **Refresh Storage Metrics** - Manually trigger storage calculation

## 🚀 Getting Started

### 1. Install Bruno

Download Bruno from [https://www.usebruno.com/](https://www.usebruno.com/)

Or install via package manager:

```bash
# macOS
brew install bruno

# Windows (Chocolatey)
choco install bruno

# Linux (Snap)
snap install bruno
```

### 2. Open Collection

1. Launch Bruno
2. Click "Open Collection"
3. Navigate to `bruno/mindia-api/` folder
4. Select the folder

### 3. Configure Environment

The collection includes two environments:

#### Local Environment (Default)
- Base URL: `http://localhost:3000` (or `http://localhost:3001` if you use that port)
- **accessToken**: Set to your **master API key** (`MASTER_API_KEY`) or a tenant **API key** (e.g. `mk_live_...`). All requests use `Authorization: Bearer {{accessToken}}`.

#### Production Environment
- Base URL: `https://your-production-url.fly.dev`
- **accessToken**: Your production master API key or API key
- Edit `environments/Production.bru` with your actual base URL and token

**Authentication**: There is no login endpoint. Use the master API key (from `MASTER_API_KEY`) or create API keys via `POST /api/v0/api-keys` and use one as `accessToken`.

To switch environments:
1. Click the environment dropdown (top-right in Bruno)
2. Select "Local" or "Production"

## 📝 Usage Guide

### File Storing Behavior

All upload endpoints support a `?store=` query parameter to control file retention:

- **`store=0`** - File will be **deleted after 24 hours** (temporary storage)
- **`store=1`** - File will be **stored permanently** until explicitly deleted
- **`store=auto`** (default) - Defers to the project's `AUTO_STORE_ENABLED` setting

**Example:**
```
POST /api/images?store=1          # Permanent storage
POST /api/videos?store=0          # Delete after 24h
POST /api/documents?store=auto    # Use project default
```

The response includes:
- `store_behavior`: The original parameter value
- `store_permanently`: Resolved boolean
- `expires_at`: Expiration timestamp (null if permanent)

A background cleanup job runs **hourly** to automatically delete expired files.

### Upload an Image

1. Open **Images → Upload Image**
2. Optionally modify the `store` query parameter (default: `auto`)
3. In the body tab, click the file input
4. Select an image file from your computer
5. Click "Send"
6. The response will include the image ID, URL, and storage information
7. The image ID is automatically saved to `{{imageId}}` variable

### Upload a Video

1. Open **Videos → Upload Video**
2. Optionally modify the `store` query parameter (default: `auto`)
3. In the body tab, click the file input
4. Select a video file (MP4, MOV, WebM, AVI, MKV)
5. Click "Send"
6. The response includes the video ID, storage information, and initial status (pending)
7. Video transcoding happens asynchronously in the background
8. Use **Get Video by ID** to check processing status

### Using Variables

After uploading content, the collection automatically sets:
- `{{imageId}}` - UUID of the last uploaded image
- `{{imageUrl}}` - S3 URL of the last uploaded image
- `{{videoId}}` - UUID of the last uploaded video
- `{{variant}}` - Video quality variant (720p, 1080p, etc.)

These variables are used in other requests. You can also manually set them:

1. Click the environment dropdown
2. Select your active environment
3. Add/edit variables:
   ```
   imageId: 550e8400-e29b-41d4-a716-446655440000
   videoId: 660f9511-f91c-23e4-b567-537725285111
   variant: 720p
   ```

### Testing Transformations

1. First upload an image or set `{{imageId}}`
2. Open any transformation request
3. Modify dimensions in the URL if desired:
   - `320x240` - Exact size
   - `320x` - Width only
   - `x240` - Height only
4. Change stretch mode if needed:
   - `on` - Allow upscaling (default)
   - `off` - No upscaling
   - `fill` - Add white background
5. Click "Send"
6. View the transformed image in the response

## 🔧 Environment Variables

Available variables you can use in requests:

| Variable | Description | Example |
|----------|-------------|---------|
| `baseUrl` | API base URL | `http://localhost:3000` |
| `imageId` | UUID of an image | `550e8400-e29b-41d4-a716-446655440000` |
| `imageUrl` | S3 URL of an image | `https://bucket.s3.amazonaws.com/...` |
| `videoId` | UUID of a video | `660f9511-f91c-23e4-b567-537725285111` |
| `variant` | Video quality variant | `720p` (360p, 480p, 720p, 1080p) |

## 📖 API Documentation

For complete API documentation, see the main project README:
- [../../../README.md](../../../README.md)
- [../../../TRANSFORMS.md](../../../TRANSFORMS.md)

## 🎯 Tips

### Testing Workflow

**Images:**
1. **Upload** an image → Get image ID
2. **List** images → Verify upload
3. **Get** metadata → Check details
4. **Transform** → Test different sizes
5. **Download** → Verify file integrity
6. **Delete** → Clean up test data

**Videos:**
1. **Upload** a video → Get video ID (status: pending)
2. **Get Video** → Monitor processing status
3. Wait for status to become "completed"
4. **Stream Master Playlist** → Get HLS URL for video player
5. **Stream Variant Playlist** → Test specific quality levels
6. **Delete** → Clean up test data

**Analytics:**
1. **Get Traffic Summary** → View overall request stats
2. **Get URL Statistics** → Analyze endpoint usage
3. **Get Storage Summary** → Check storage consumption
4. **Refresh Storage Metrics** → Update cached metrics

### Script Features

The collection includes post-response scripts that:
- Automatically save image IDs after upload
- Log useful information to console
- Set variables for subsequent requests

### Keyboard Shortcuts (in Bruno)

- `Ctrl/Cmd + Enter` - Send request
- `Ctrl/Cmd + S` - Save request
- `Ctrl/Cmd + K` - Quick search
- `Ctrl/Cmd + /` - Toggle sidebar

## 🔗 Related Files

- `bruno.json` - Collection metadata
- `environments/Local.bru` - Local development config
- `environments/Production.bru` - Production config
- `.gitignore` - Git ignore patterns

## 📚 Learn More

- [Bruno Documentation](https://docs.usebruno.com/)
- [Mindia GitHub Repository](https://github.com/yourusername/mindia)
- [Image Transformation Guide](../../../TRANSFORMS.md)

## 🐛 Troubleshooting

### "Connection refused" errors
- Ensure the Mindia server is running: `cargo run`
- Check the `baseUrl` in your environment matches your server

### "Image not found" (404)
- Verify the `imageId` exists: use List Images first
- Check you're using the correct environment (Local vs Production)

### File upload fails
- Ensure file is a valid image (JPG, PNG, GIF, WebP)
- Check file size is under the configured limit (default 10MB)
- Verify the server has proper AWS credentials configured

### Transformations fail
- Confirm the image exists and was uploaded successfully
- Check dimension format: `WxH`, `Wx`, or `xH`
- Verify stretch mode is valid: `on`, `off`, or `fill`

## 📄 License

This collection is part of the Mindia project - MIT License

