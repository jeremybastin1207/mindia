# Plugins

Mindia's plugin system extends functionality by processing media files with third-party AI and cloud services. Plugins run asynchronously via the task queue—you trigger execution, and results are stored when processing completes.

## Overview

| Plugin | Media Type | Purpose |
|--------|------------|---------|
| [Assembly AI](#assembly-ai-audio-transcription) | Audio | Speech-to-text transcription with word-level timestamps |
| [AWS Transcribe](#aws-transcribe-audio-transcription) | Audio | Amazon transcription with diarization, medical/legal support |
| [AWS Rekognition](#aws-rekognition-image-analysis) | Image | Object and label detection (AWS ecosystem) |
| [Google Cloud Vision](#google-cloud-vision-image-analysis) | Image | Labels, OCR, faces, landmarks, content moderation |
| [Claude Vision](#claude-vision-image-analysis) | Image | Natural language image analysis (Anthropic Claude) |
| [Replicate DeOldify](#replicate-deoldify-image-colorization) | Image | Colorize black & white photos |

## Available Plugins & How to Use

### Assembly AI (Audio Transcription)

Transcribes audio files into text with word-level timestamps. Ideal for podcasts, meetings, and voice content.

**Supported formats:** MP3, M4A, WAV, FLAC, OGG

**Features:** Automatic language detection, word-level timestamps, full transcript

**Configure:**
```bash
PUT /api/v0/plugins/assembly_ai/config
Content-Type: application/json

{
  "enabled": true,
  "config": {
    "api_key": "your-assembly-ai-api-key",
    "language_code": "en"   // Optional: omit for auto-detect
  }
}
```

**Execute:** `POST /api/v0/plugins/assembly_ai/execute` with `{"media_id": "<audio-id>"}`

### AWS Transcribe (Audio Transcription)

Uses Amazon's transcription service. Best for AWS users who need diarization, custom vocabularies, or medical/legal transcription.

**Supported formats:** MP3, MP4, WAV, FLAC, OGG, AMR, WebM

**Features:** 100+ languages, speaker labels, custom vocabularies, medical/legal domains

**Configure:**
```bash
PUT /api/v0/plugins/aws_transcribe/config
Content-Type: application/json

{
  "enabled": true,
  "config": {
    "region": "us-east-1",
    "s3_bucket": "your-transcription-bucket",
    "language_code": "en-US",        // Optional
    "show_speaker_labels": true      // Optional: diarization
  }
}
```

**Execute:** `POST /api/v0/plugins/aws_transcribe/execute` with `{"media_id": "<audio-id>"}`

**Note:** Requires `plugin-aws-transcribe` feature and AWS credentials. Audio must be in the configured S3 bucket.

### AWS Rekognition (Image Analysis)

Detects objects and labels in images using Amazon Rekognition.

**Supported formats:** JPEG, PNG, GIF, WebP, BMP

**Features:** Label detection, text detection, custom labels, bounding boxes

**Configure:**
```bash
PUT /api/v0/plugins/aws_rekognition/config
Content-Type: application/json

{
  "enabled": true,
  "config": {
    "region": "us-east-1",
    "min_confidence": 70.0,
    "detect_labels": true,
    "detect_text": true
  }
}
```

**Execute:** `POST /api/v0/plugins/aws_rekognition/execute` with `{"media_id": "<image-id>"}`

**Note:** Requires `content-moderation` feature and AWS credentials.

### Google Cloud Vision (Image Analysis)

Comprehensive image analysis: labels, OCR, faces, landmarks, content moderation.

**Supported formats:** JPEG, PNG, GIF, WebP, BMP, TIFF

**Features:** Labels, text extraction, face analysis, object localization, safe search

**Configure:**
```bash
PUT /api/v0/plugins/google_vision/config
Content-Type: application/json

{
  "enabled": true,
  "config": {
    "api_key": "your-google-cloud-api-key",
    "features": ["LABEL_DETECTION", "TEXT_DETECTION", "FACE_DETECTION", "SAFE_SEARCH_DETECTION"],
    "min_score": 0.5
  }
}
```

**Execute:** `POST /api/v0/plugins/google_vision/execute` with `{"media_id": "<image-id>"}`

### Claude Vision (Image Analysis)

Uses Anthropic Claude for natural language image analysis with context and relationships.

**Supported formats:** JPEG, PNG, GIF, WebP

**Features:** Objects, text, colors, scene understanding, content moderation, descriptions

**Configure:**
```bash
PUT /api/v0/plugins/claude_vision/config
Content-Type: application/json

{
  "enabled": true,
  "config": {
    "api_key": "your-anthropic-api-key",
    "model": "claude-sonnet-4-20250514",
    "features": ["objects", "text", "colors", "scene", "content_moderation"]
  }
}
```

**Execute:** `POST /api/v0/plugins/claude_vision/execute` with `{"media_id": "<image-id>"}`

### Replicate DeOldify (Image Colorization)

Colorizes black and white images using Replicate's DeOldify model.

**Supported formats:** JPEG, PNG, GIF, WebP

**Features:** AI-powered colorization, configurable render quality

**Configure:**
```bash
PUT /api/v0/plugins/replicate_deoldify/config
Content-Type: application/json

{
  "enabled": true,
  "config": {
    "api_token": "your-replicate-api-token",
    "render_factor": 35
  }
}
```

**Execute:** `POST /api/v0/plugins/replicate_deoldify/execute` with `{"media_id": "<image-id>"}`

**Note:** `render_factor` (1–40) controls quality vs speed. Higher = better quality, slower.

## Getting Started

### 1. List Available Plugins

```bash
GET /api/v0/plugins
```

**Response:**
```json
[
  {
    "name": "assembly_ai",
    "description": "Assembly AI transcription service for audio files",
    "supported_media_types": ["audio"]
  },
  {
    "name": "aws_transcribe",
    "description": "AWS Transcribe transcription service for audio files",
    "supported_media_types": ["audio"]
  },
  {
    "name": "google_vision",
    "description": "Google Cloud Vision API for comprehensive image analysis",
    "supported_media_types": ["image"]
  }
]
```

### 2. Configure a Plugin

Before using a plugin, you must configure it with your API credentials:

```bash
PUT /api/v0/plugins/{plugin_name}/config
Content-Type: application/json

{
  "enabled": true,
  "config": {
    "api_key": "your-assembly-ai-api-key",
    "language_code": "en"  // Optional: "en", "es", "fr", etc. Omit for auto-detect
  }
}
```

**Response:**
```json
{
  "plugin_name": "assembly_ai",
  "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
  "enabled": true,
  "config": {
    "api_key": "***",
    "language_code": "en"
  },
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

### 3. Get Plugin Configuration

```bash
GET /api/v0/plugins/{plugin_name}/config
```

**Response:**
```json
{
  "plugin_name": "assembly_ai",
  "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
  "enabled": true,
  "config": {
    "api_key": "***",
    "language_code": "en"
  },
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

### 4. Execute a Plugin

Execute a plugin on a media file:

```bash
POST /api/v0/plugins/{plugin_name}/execute
Content-Type: application/json

{
  "media_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response:**
```json
{
  "task_id": "660e8400-e29b-41d4-a716-446655440001",
  "execution_id": "770e8400-e29b-41d4-a716-446655440002",
  "status": "pending"
}
```

The plugin execution runs asynchronously. Use the `task_id` to check the task status via the [Tasks API](tasks.md).

### 5. Check Execution Status

Query the task status to see when the plugin execution completes:

```bash
GET /api/v0/tasks/{task_id}
```

Once the task status is `completed`, the plugin results are stored in the database and can be retrieved via the plugin execution record.

## Plugin Details

### Assembly AI Plugin Details

### Configuration Options

- **`api_key`** (required): Your Assembly AI API key from [assemblyai.com](https://www.assemblyai.com)
- **`language_code`** (optional): Language code for transcription. If omitted, Assembly AI will auto-detect the language.
  - Supported codes: `en`, `es`, `fr`, `de`, `it`, `pt`, `nl`, `ja`, `ko`, `zh`, `ar`, `hi`, `pl`, `ru`, `tr`, `vi`, `uk`, `cs`, `el`, `fi`, `he`, `id`, `ms`, `no`, `ro`, `sv`, `th`, `da`

### Execution Flow

1. **Upload**: Audio file is uploaded to Assembly AI's servers
2. **Transcription**: Assembly AI processes the audio and generates a transcript
3. **Polling**: The plugin polls Assembly AI until transcription completes
4. **Storage**: Transcript data (text, words with timestamps, status) is stored in the database

### Transcript Data Structure

The plugin stores the following data:

```json
{
  "transcript_id": "assembly_ai_transcript_id",
  "text": "Full transcript text...",
  "words": [
    {
      "text": "Hello",
      "start": 0,
      "end": 500,
      "confidence": 0.99
    }
  ],
  "status": "completed",
  "language_code": "en"
}
```

### Error Handling

If plugin execution fails:
- The task status will be `failed`
- Check the task error message for details
- Common errors:
  - Invalid API key
  - Unsupported audio format
  - Network timeouts
  - Assembly AI service errors

## Best Practices

1. **API Key Security**: Store your API keys securely. Never commit them to version control.

2. **Language Detection**: For best results, specify the language code if you know it. Auto-detection works well but may be slower.

3. **File Size**: Large audio files take longer to process. Monitor task status for completion.

4. **Rate Limits**: Be aware of Assembly AI's rate limits. The plugin includes automatic retry logic.

5. **Error Handling**: Always check task status after execution. Handle failures gracefully in your application.

## Example: Complete Workflow

```javascript
// 1. Configure plugin
await fetch('/api/v0/plugins/assembly_ai/config', {
  method: 'PUT',
  headers: {
    'Authorization': `Bearer ${apiKey}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    enabled: true,
    config: {
      api_key: process.env.ASSEMBLY_AI_API_KEY,
      language_code: 'en'
    }
  })
});

// 2. Upload audio file
const uploadResponse = await fetch('/api/v0/audios', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${apiKey}`
  },
  body: formData
});
const { id: audioId } = await uploadResponse.json();

// 3. Execute plugin
const executeResponse = await fetch(`/api/v0/plugins/assembly_ai/execute`, {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${apiKey}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({ media_id: audioId })
});
const { task_id } = await executeResponse.json();

// 4. Poll for completion
let task;
do {
  await new Promise(resolve => setTimeout(resolve, 2000)); // Wait 2 seconds
  const taskResponse = await fetch(`/api/v0/tasks/${task_id}`, {
    headers: { 'Authorization': `Bearer ${apiKey}` }
  });
  task = await taskResponse.json();
} while (task.status === 'pending' || task.status === 'processing');

if (task.status === 'completed') {
  console.log('Transcription complete!');
  // Retrieve transcript data from plugin execution record
} else {
  console.error('Transcription failed:', task.error);
}
```

## Troubleshooting

**Plugin not found:**
- Ensure the plugin name is correct (e.g., `assembly_ai`)
- Check that the plugin is available in your Mindia instance

**Plugin not configured:**
- Configure the plugin with valid API credentials before execution
- Ensure `enabled` is set to `true`

**Execution fails:**
- Verify your API key is valid
- Check that the media file is a supported audio format
- Review task error messages for specific issues
- Ensure your Assembly AI account has sufficient credits

**Slow execution:**
- Large audio files take longer to process
- Network latency can affect upload/polling times
- Assembly AI processing time varies by file length and complexity

## AWS Transcribe Plugin Details

### Configuration Options

- **`region`** (required): AWS region (e.g., "us-east-1", "eu-west-1")
- **`s3_bucket`** (required): S3 bucket name where audio files are stored (must be accessible to AWS Transcribe)
- **`language_code`** (optional): Language code for transcription (e.g., "en-US", "es-US", "fr-FR"). If omitted, AWS Transcribe will auto-detect.
  - Supported codes: See [AWS Transcribe Language Codes](https://docs.aws.amazon.com/transcribe/latest/dg/supported-languages.html)
- **`media_format`** (optional): Audio format (e.g., "mp3", "mp4", "wav", "flac", "ogg", "amr", "webm")
- **`vocabulary_name`** (optional): Name of custom vocabulary to use
- **`vocabulary_filter_name`** (optional): Name of vocabulary filter for profanity filtering
- **`show_speaker_labels`** (optional, default: false): Enable speaker identification (diarization)
- **`max_speaker_labels`** (optional): Maximum number of speakers to identify (if speaker labels enabled)

### Execution Flow

1. **S3 Access**: Audio file must already be stored in the specified S3 bucket
2. **Job Creation**: AWS Transcribe job is created with the S3 URI
3. **Polling**: The plugin polls AWS Transcribe until transcription completes
4. **Storage**: Transcript data (text, words with timestamps, speaker labels) is stored in the database

### Transcript Data Structure

```json
{
  "transcription_job_name": "mindia-uuid-12345",
  "text": "Full transcript text...",
  "words": [
    {
      "text": "Hello",
      "start": 0,
      "end": 500,
      "confidence": 0.99,
      "type": "pronunciation"
    }
  ],
  "language_code": "en-US",
  "status": "COMPLETED",
  "completed_at": "2024-01-01T00:00:00Z"
}
```

### Prerequisites

- AWS account with Transcribe service enabled
- S3 bucket accessible to AWS Transcribe service
- Audio files must be stored in S3 (this plugin assumes S3 storage backend)
- AWS credentials configured (via environment variables, IAM role, or credentials file)

### Error Handling

Common errors:
- Invalid AWS credentials or region
- S3 bucket not accessible
- Unsupported audio format
- Transcription job timeout (default: 10 minutes)
- Insufficient permissions for AWS Transcribe

## AWS Rekognition Plugin Details

### Configuration Options

- **`region`** (required): AWS region (e.g., "us-east-1", "eu-west-1")
- **`min_confidence`** (optional, default: 70.0): Minimum confidence threshold (0-100) for detections
- **`detect_labels`** (optional, default: true): Enable general label detection
- **`detect_text`** (optional, default: true): Enable text detection (OCR)
- **`detect_custom_labels`** (optional, default: false): Enable custom model labels
- **`custom_model_arn`** (optional): ARN of custom Rekognition model

### Execution Flow

1. **Download**: Image is downloaded from storage
2. **Detection**: Image is sent to AWS Rekognition for analysis
3. **Storage**: Labels, text, and bounding boxes are stored in media metadata

### Prerequisites

- AWS account with Rekognition enabled
- AWS credentials (environment variables, IAM role, or credentials file)
- `content-moderation` feature flag enabled in Mindia build

## Google Cloud Vision Plugin Details

### Configuration Options

- **`api_key`** (required): Google Cloud API key from [Google Cloud Console](https://console.cloud.google.com)
- **`project_id`** (optional): Google Cloud project ID (required for some advanced features)
- **`features`** (optional, default: all enabled): List of features to enable
  - Available: `LABEL_DETECTION`, `TEXT_DETECTION`, `FACE_DETECTION`, `OBJECT_LOCALIZATION`, `SAFE_SEARCH_DETECTION`, `LANDMARK_DETECTION`, `LOGO_DETECTION`, `PRODUCT_SEARCH`, `CROP_HINTS`, `WEB_DETECTION`
- **`min_score`** (optional, default: 0.5): Minimum confidence score (0.0-1.0) for detections

### Execution Flow

1. **Download**: Image is downloaded from storage
2. **Analysis**: Image is sent to Google Cloud Vision API for analysis
3. **Processing**: Results are filtered by confidence score and formatted
4. **Storage**: Analysis results are stored in the database metadata

### Analysis Result Structure

```json
{
  "labels": [
    {
      "description": "Person",
      "score": 0.95,
      "mid": "/m/01g317",
      "topicality": 0.95
    }
  ],
  "text": [
    {
      "description": "Extracted text from image",
      "locale": "en",
      "bounding_poly": { "vertices": [...] }
    }
  ],
  "faces": [
    {
      "bounding_poly": { "vertices": [...] },
      "detection_confidence": 0.99,
      "joy_likelihood": "VERY_LIKELY",
      "sorrow_likelihood": "VERY_UNLIKELY"
    }
  ],
  "objects": [
    {
      "name": "Clothing",
      "score": 0.92,
      "bounding_poly": { "vertices": [...] }
    }
  ],
  "safe_search": {
    "adult": "VERY_UNLIKELY",
    "spoof": "UNLIKELY",
    "medical": "UNLIKELY",
    "violence": "UNLIKELY",
    "racy": "UNLIKELY"
  },
  "detected_at": "2024-01-01T00:00:00Z"
}
```

### Use Cases

- **Content Moderation**: Use safe search detection to filter inappropriate content
- **OCR**: Extract text from images (signs, documents, screenshots)
- **Object Recognition**: Identify objects and scenes in images
- **Face Analysis**: Detect faces and analyze emotions
- **Landmark Detection**: Identify famous landmarks in images
- **Logo Detection**: Recognize brand logos

### Error Handling

Common errors:
- Invalid API key
- Quota exceeded (check Google Cloud quotas)
- Image too large (max 20MB for API key authentication)
- Unsupported image format
- Network timeouts

### Best Practices

1. **Feature Selection**: Only enable features you need to reduce API costs
2. **Confidence Threshold**: Adjust `min_score` based on your accuracy requirements
3. **Rate Limits**: Be aware of Google Cloud Vision API rate limits
4. **Cost Optimization**: Use appropriate features for your use case to minimize API calls

## Claude Vision Plugin Details

### Configuration Options

- **`api_key`** (required): Anthropic API key from [Anthropic Console](https://console.anthropic.com)
- **`model`** (optional, default: `claude-sonnet-4-20250514`): Claude model to use
  - Available: `claude-sonnet-4-20250514`, `claude-opus-4-20250514`, `claude-sonnet-3-5-20241022`
- **`max_tokens`** (optional, default: 2048): Maximum tokens for analysis response
- **`features`** (optional, default: all enabled): List of analysis features
  - Available: `objects`, `text`, `colors`, `scene`, `content_moderation`, `description`

### Configuration Example

```bash
PUT /api/v0/plugins/claude_vision/config
Content-Type: application/json

{
  "enabled": true,
  "config": {
    "api_key": "your-anthropic-api-key",
    "model": "claude-sonnet-4-20250514",
    "features": ["objects", "text", "colors", "scene"]
  }
}
```

### Use Cases

- **Natural Language Understanding**: Get comprehensive descriptions with context
- **Content Analysis**: Understand relationships between objects and the overall scene
- **Text Extraction**: Extract and understand text in context
- **Color Analysis**: Identify dominant colors and color schemes
- **Safety Assessment**: Evaluate content appropriateness with explanations

### Advantages Over Traditional Vision APIs

1. **Context Understanding**: Claude understands relationships and context, not just labels
2. **Natural Language**: Results are in natural language, easier to understand and use
3. **Flexible Output**: Can format results based on your specific needs
4. **Combined Analysis**: Single API call for multiple analysis types
5. **Better for Complex Scenes**: Excels at understanding complex scenes with multiple elements

## Replicate DeOldify Plugin Details

### Configuration Options

- **`api_token`** (required): Replicate API token from [replicate.com](https://replicate.com)
- **`model_version`** (optional): Model version to use (default: latest)
- **`render_factor`** (optional, default: 35): Quality factor 1-40. Higher = better quality, slower processing

### Execution Flow

1. **URL**: Image is made accessible to Replicate (public URL from Mindia storage)
2. **Prediction**: Replicate DeOldify model processes the image
3. **Polling**: Plugin polls until colorization completes (up to ~5 minutes)
4. **Storage**: Colorized image URL is stored in media metadata

### Use Cases

- Restoring old black and white photographs
- Colorizing historical images
- Adding color to scanned documents or sketches

### Error Handling

Common errors:
- Invalid Replicate API token
- Image URL not accessible (check CORS and storage permissions)
- Prediction timeout
- Replicate service unavailable

## Related Documentation

- [Tasks](tasks.md) - Task management and status tracking
- [Audio](audio.md) - Audio file upload and management
- [Images](images.md) - Image upload and management
- [API Reference](api-reference.md) - Complete API endpoint reference

