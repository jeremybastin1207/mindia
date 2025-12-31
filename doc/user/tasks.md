# Tasks

Mindia uses a task queue system to handle background processing. Tasks are used for operations like video transcoding, embedding generation, and plugin execution. This guide shows you how to monitor and manage these tasks.

## Overview

Tasks represent asynchronous operations that run in the background:
- **Video Transcoding**: Converting videos to HLS format
- **Embedding Generation**: Creating semantic search embeddings
- **Plugin Execution**: Running plugins on media files

## Task Lifecycle

```
Scheduled → Processing → Completed
                ↓
            Failed (with retries)
```

## Getting Started

### 1. List Tasks

List all tasks with optional filters:

```bash
# List all tasks
GET /api/tasks

# Filter by status
GET /api/tasks?status=completed

# Filter by type
GET /api/tasks?type=VideoTranscoding

# Filter by date range
GET /api/tasks?start_date=2024-01-01T00:00:00Z&end_date=2024-01-31T23:59:59Z

# Pagination
GET /api/tasks?limit=50&offset=0
```

**Query Parameters:**
- `status`: Filter by status (`pending`, `scheduled`, `processing`, `completed`, `failed`, `cancelled`)
- `type`: Filter by task type (`VideoTranscoding`, `EmbeddingGeneration`, `PluginExecution`)
- `start_date`: Filter tasks created after this date (ISO 8601)
- `end_date`: Filter tasks created before this date (ISO 8601)
- `limit`: Number of results (default: 50, max: 100)
- `offset`: Pagination offset (default: 0)

**Response:**
```json
{
  "tasks": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "type": "VideoTranscoding",
      "status": "processing",
      "priority": 0,
      "payload": {
        "video_id": "660e8400-e29b-41d4-a716-446655440001"
      },
      "created_at": "2024-01-01T00:00:00Z",
      "started_at": "2024-01-01T00:00:05Z",
      "completed_at": null,
      "error": null,
      "retry_count": 0,
      "max_retries": 3
    }
  ],
  "total": 42,
  "limit": 50,
  "offset": 0
}
```

### 2. Get Task Details

Get detailed information about a specific task:

```bash
GET /api/tasks/{id}
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "VideoTranscoding",
  "status": "completed",
  "priority": 0,
  "payload": {
    "video_id": "660e8400-e29b-41d4-a716-446655440001"
  },
  "created_at": "2024-01-01T00:00:00Z",
  "started_at": "2024-01-01T00:00:05Z",
  "completed_at": "2024-01-01T00:05:30Z",
  "error": null,
  "retry_count": 0,
  "max_retries": 3,
  "timeout_seconds": 3600
}
```

### 3. Get Task Statistics

Get aggregate statistics about tasks:

```bash
GET /api/tasks/stats
```

**Response:**
```json
{
  "total": 1000,
  "by_status": {
    "pending": 5,
    "scheduled": 2,
    "processing": 3,
    "completed": 950,
    "failed": 35,
    "cancelled": 5
  },
  "by_type": {
    "VideoTranscoding": 500,
    "EmbeddingGeneration": 400,
    "PluginExecution": 100
  },
  "average_duration_seconds": 45.5,
  "success_rate": 0.95
}
```

### 4. Cancel a Task

Cancel a pending or scheduled task:

```bash
POST /api/tasks/{id}/cancel
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "cancelled",
  "message": "Task cancelled successfully"
}
```

**Note:** Only tasks with status `pending` or `scheduled` can be cancelled. Processing tasks cannot be cancelled.

### 5. Retry a Failed Task

Retry a failed task:

```bash
POST /api/tasks/{id}/retry
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "scheduled",
  "message": "Task scheduled for retry"
}
```

**Note:** Tasks can only be retried if they have not exceeded `max_retries`.

## Task Types

### VideoTranscoding
Converts uploaded videos to HLS format for streaming.

**Payload:**
```json
{
  "video_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Typical Duration:** 1-10 minutes depending on video size

### EmbeddingGeneration
Generates semantic search embeddings for media files.

**Payload:**
```json
{
  "entity_type": "image",
  "entity_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Typical Duration:** 5-30 seconds

### PluginExecution
Executes a plugin on a media file.

**Payload:**
```json
{
  "plugin_name": "assembly_ai",
  "media_id": "550e8400-e29b-41d4-a716-446655440000",
  "tenant_id": "660e8400-e29b-41d4-a716-446655440001"
}
```

**Typical Duration:** Varies by plugin (Assembly AI: 1-5 minutes)

## Task Statuses

- **`pending`**: Task is queued and waiting to be processed
- **`scheduled`**: Task is scheduled for future execution
- **`processing`**: Task is currently being executed
- **`completed`**: Task finished successfully
- **`failed`**: Task failed (may be retried)
- **`cancelled`**: Task was cancelled before completion

## Monitoring Tasks

### Polling for Completion

```javascript
async function waitForTaskCompletion(taskId, pollInterval = 2000) {
  while (true) {
    const response = await fetch(`/api/tasks/${taskId}`, {
      headers: { 'Authorization': `Bearer ${apiKey}` }
    });
    const task = await response.json();
    
    if (task.status === 'completed') {
      return task;
    } else if (task.status === 'failed') {
      throw new Error(`Task failed: ${task.error}`);
    } else if (task.status === 'cancelled') {
      throw new Error('Task was cancelled');
    }
    
    // Wait before next poll
    await new Promise(resolve => setTimeout(resolve, pollInterval));
  }
}

// Usage
try {
  const completedTask = await waitForTaskCompletion(taskId);
  console.log('Task completed!', completedTask);
} catch (error) {
  console.error('Task error:', error);
}
```

### Webhook Notifications

For better efficiency, use [webhooks](webhooks.md) to receive notifications when tasks complete instead of polling.

## Error Handling

Tasks may fail for various reasons:
- **Network errors**: Temporary connectivity issues
- **Processing errors**: Invalid input or processing failures
- **Timeout**: Task exceeded maximum execution time
- **Resource limits**: System resource constraints

Failed tasks are automatically retried with exponential backoff up to `max_retries`.

## Best Practices

1. **Monitor Progress**: Poll task status or use webhooks to track completion
2. **Handle Failures**: Always check task status and handle errors appropriately
3. **Set Timeouts**: Be aware of task timeout limits
4. **Retry Logic**: Use the retry endpoint for transient failures
5. **Cancel Unneeded Tasks**: Cancel tasks that are no longer needed

## Rate Limiting

Tasks are rate-limited by type to prevent system overload:
- Each task type has its own rate limit
- Tasks are queued and processed according to priority
- High-priority tasks are processed first

## Example: Complete Workflow

```javascript
// 1. Upload a video (creates transcoding task)
const videoResponse = await fetch('/api/videos', {
  method: 'POST',
  headers: { 'Authorization': `Bearer ${apiKey}` },
  body: formData
});
const { id: videoId } = await videoResponse.json();

// 2. Find the transcoding task
const tasksResponse = await fetch(
  `/api/tasks?type=VideoTranscoding&status=processing`,
  { headers: { 'Authorization': `Bearer ${apiKey}` } }
);
const { tasks } = await tasksResponse.json();
const transcodingTask = tasks.find(t => 
  t.payload.video_id === videoId
);

// 3. Monitor task completion
if (transcodingTask) {
  const completedTask = await waitForTaskCompletion(transcodingTask.id);
  
  if (completedTask.status === 'completed') {
    // Video is ready for streaming
    console.log('Video transcoding complete!');
  } else if (completedTask.status === 'failed') {
    // Handle failure
    console.error('Transcoding failed:', completedTask.error);
    
    // Retry if appropriate
    if (completedTask.retry_count < completedTask.max_retries) {
      await fetch(`/api/tasks/${completedTask.id}/retry`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${apiKey}` }
      });
    }
  }
}
```

## API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/tasks` | List tasks with filters |
| GET | `/api/tasks/{id}` | Get task details |
| GET | `/api/tasks/stats` | Get task statistics |
| POST | `/api/tasks/{id}/cancel` | Cancel a task |
| POST | `/api/tasks/{id}/retry` | Retry a failed task |

## Related Documentation

- [Videos](videos.md) - Video upload and transcoding
- [Semantic Search](semantic-search.md) - Embedding generation
- [Plugins](plugins.md) - Plugin execution
- [Webhooks](webhooks.md) - Task completion notifications
- [API Reference](api-reference.md) - Complete API documentation

