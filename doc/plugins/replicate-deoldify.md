# Replicate DeOldify Plugin

The Replicate DeOldify plugin integrates with [Replicate's DeOldify model](https://replicate.com/arielreplicate/deoldify_image) to automatically colorize old black and white images.

## Overview

DeOldify is a deep learning-based system for colorizing and restoring old images. This plugin provides seamless integration with Replicate's hosted version of the DeOldify image model, which offers both artistic (vibrant) and stable (natural) rendering modes.

## Features

- **Automatic Image Colorization**: Transform black and white images into color
- **Artistic & Stable Modes**: Higher render factors (30-40) for vibrant artistic results, lower (10-20) for natural stable colors
- **Quality Control**: Adjustable render factor (1-40) for balancing quality and processing time
- **File Grouping**: Automatically creates file groups linking original and colorized images
- **Asynchronous Processing**: Non-blocking execution with task queue integration
- **Secure Configuration**: API tokens are encrypted at rest using AES-256-GCM
- **Result Storage**: Colorized images are automatically uploaded to your storage backend

## Requirements

1. **Replicate Account**: Sign up at https://replicate.com
2. **API Token**: Get your API token from https://replicate.com/account/api-tokens
3. **Credits**: Replicate charges per prediction based on processing time

## Configuration

### Step 1: Get Your Replicate API Token

1. Go to https://replicate.com/account/api-tokens
2. Create a new API token or copy your existing token
3. The token format is: `r8_xxxxxxxxxxxxxxxxxxxxxxxxxxxxx`

### Step 2: Configure the Plugin

```bash
PUT /api/v0/plugins/replicate_deoldify/config
```

**Request Body:**
```json
{
  "enabled": true,
  "config": {
    "api_token": "r8_xxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "render_factor": 35
  }
}
```

**Configuration Options:**

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `api_token` | string | Yes | - | Your Replicate API token |
| `render_factor` | integer | No | 35 | Image quality factor (1-40) |
| `model_version` | string | No | latest | Specific model version to use |

### Render Factor Guidelines

The `render_factor` parameter controls the colorization style and quality:

- **Stable/Natural (10-20)**: More natural colors, good for portraits and landscapes
- **Balanced (25-30)**: Mix of artistic and natural
- **Artistic (35-40)**: Vibrant, highly saturated colors (default: 35)

## Usage

### Step 1: Upload an Image

```bash
POST /api/v0/media/images/upload
Content-Type: multipart/form-data

file: old_photo.jpg
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "old_photo.jpg",
  ...
}
```

### Step 2: Execute the Plugin

```bash
POST /api/v0/plugins/replicate_deoldify/execute
```

**Request Body:**
```json
{
  "media_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response:**
```json
{
  "task_id": "123e4567-e89b-12d3-a456-426614174000",
  "status": "queued",
  "plugin_name": "replicate_deoldify",
  "media_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Step 3: Check Task Status

```bash
GET /api/v0/tasks/123e4567-e89b-12d3-a456-426614174000
```

**Response (Processing):**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "status": "processing",
  "progress": 45,
  ...
}
```

**Response (Completed):**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "status": "completed",
  "result": {
    "prediction_id": "abc123xyz",
    "colorized_storage_key": "media/def456.mp4",
    "colorized_storage_url": "https://...",
    "replicate_output_url": "https://replicate.delivery/...",
    "render_factor": 20,
    "processed_at": "2024-01-15T10:30:00Z"
  },
  ...
}
```

### Step 4: Access the Colorized Image

The colorized image is available in multiple ways:

1. **New Media Entry**: The colorized image gets its own UUID (`colorized_media_id`)
2. **Storage Backend**: Access via `colorized_storage_url` in the task result
3. **File Group**: Both images are linked in a file group
4. **Media Metadata**: Check the original media item's metadata for plugin results

```bash
# Get the original image metadata (includes plugin results)
GET /api/v0/media/550e8400-e29b-41d4-a716-446655440000/metadata

# Get the colorized image directly
GET /api/v0/media/{colorized_media_id}

# Get the file group (shows both images)
GET /api/v0/file-groups/{file_group_id}
```

## Output Structure

When the plugin completes successfully, it stores the following data:

```json
{
  "replicate_deoldify": {
    "prediction_id": "abc123xyz",
    "colorized_media_id": "def456-7890-...",
    "colorized_storage_key": "media/def456.jpg",
    "colorized_storage_url": "https://storage.example.com/media/def456.jpg",
    "file_group_id": "group-uuid-here",
    "replicate_output_url": "https://replicate.delivery/...",
    "render_factor": 35,
    "processed_at": "2024-01-15T10:30:00Z",
    "analyzed_at": "2024-01-15T10:30:00Z",
    "config": {
      "model": "arielreplicate/deoldify_image",
      "render_factor": 35
    }
  }
}
```

## Error Handling

### Common Errors

**Invalid Media Type:**
```json
{
  "error": "Image not found"
}
```

**File Too Large:**
```json
{
  "error": "Image file too large for processing"
}
```

**API Token Not Configured:**
```json
{
  "error": "Replicate API token not configured. Please set a valid Replicate API token."
}
```

**Replicate API Error:**
```json
{
  "error": "Replicate prediction failed: insufficient credits"
}
```

## Pricing

Replicate charges approximately **$0.0018 per prediction** for DeOldify image colorization. Processing typically takes 5-10 seconds on Nvidia T4 GPU hardware.

Actual costs depend on:
- Image resolution
- Render factor setting
- Current Replicate pricing

Check current pricing at: https://replicate.com/arielreplicate/deoldify_image

## Usage Tracking

The plugin tracks Replicate's prediction time for cost analysis:

```json
{
  "usage": {
    "unit_type": "seconds",
    "total_units": 45,
    "raw_usage": {
      "predict_time": 45.2
    }
  }
}
```

## Limitations

1. **File Size**: Maximum 50 MB for images
2. **Processing Time**: Typically 5-10 seconds per image
3. **Historical Accuracy**: AI-generated colors may not be historically accurate
4. **Credits Required**: You must have sufficient Replicate credits
5. **Supported Formats**: JPG, PNG, WebP

## Best Practices

1. **Test with Sample Images**: Start with a few images to verify configuration
2. **Choose Appropriate Render Factor**: 
   - Use 35-40 for artistic/vibrant results
   - Use 10-20 for natural/stable colors
3. **Monitor Credits**: Check your Replicate account balance regularly
4. **Batch Processing**: Process multiple images efficiently
5. **Use File Groups**: Leverage automatic grouping to keep originals and colorized versions together

## Troubleshooting

### Plugin Execution Hangs

- Check Replicate API status: https://status.replicate.com
- Verify your API token is valid and has credits
- Check the task queue for other pending tasks

### Poor Colorization Quality

- For more vibrant colors: Increase `render_factor` to 35-40
- For natural colors: Decrease `render_factor` to 10-20
- Ensure source image has good contrast and resolution
- Try preprocessing the image (adjust brightness/contrast)

### Timeout Errors

- The plugin waits up to 5 minutes (300 attempts × 1 second)
- Image processing typically completes in 5-10 seconds
- Check the Replicate dashboard for prediction status

## API Reference

### Plugin Name
```
replicate_deoldify
```

### Supported Media Types
- Image (JPG, PNG, WebP)

### Configuration Schema
```typescript
{
  api_token: string;        // Required: Replicate API token
  render_factor?: number;   // Optional: 1-40, default 35
  model_version?: string;   // Optional: specific model version
}
```

## Security Notes

- API tokens are encrypted at rest using AES-256-GCM
- Tokens are redacted in API responses (shows only `r8_xxx***`)
- Never log or expose API tokens in plaintext
- Rotate tokens periodically for security

## Related Resources

- [Replicate DeOldify Image Model](https://replicate.com/arielreplicate/deoldify_image)
- [DeOldify Website](https://deoldify.ai/)
- [DeOldify GitHub](https://github.com/jantic/DeOldify) (archived)
- [Replicate API Documentation](https://replicate.com/docs)
- [Plugin Configuration Security](../../SECURITY_PLUGIN_KEYS.md)

## Example Workflow

```bash
# 1. Configure the plugin
curl -X PUT http://localhost:3000/api/v0/plugins/replicate_deoldify/config \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "config": {
      "api_token": "r8_your_token_here",
      "render_factor": 25
    }
  }'

# 2. Upload an image
curl -X POST http://localhost:3000/api/v0/media/images/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@old_photo.jpg"

# 3. Execute the plugin
curl -X POST http://localhost:3000/api/v0/plugins/replicate_deoldify/execute \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "media_id": "550e8400-e29b-41d4-a716-446655440000"
  }'

# 4. Check status
curl http://localhost:3000/api/v0/tasks/$TASK_ID \
  -H "Authorization: Bearer $TOKEN"

# 5. Download colorized image
curl -O $(curl http://localhost:3000/api/v0/tasks/$TASK_ID \
  -H "Authorization: Bearer $TOKEN" \
  | jq -r '.result.colorized_storage_url')
```

## Support

For issues specific to:
- **Plugin Integration**: Open an issue in the Mindia repository
- **DeOldify Model**: Check the Replicate model page or DeOldify GitHub
- **Replicate API**: Contact Replicate support at https://replicate.com/docs
