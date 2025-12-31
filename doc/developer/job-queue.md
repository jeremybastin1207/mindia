# Job Queue System

Mindia uses a task queue system for background processing of long-running operations like video transcoding, embedding generation, and plugin execution. This document explains the architecture and implementation.

## Overview

The job queue system provides:
- **Asynchronous Processing**: Long-running tasks don't block API requests
- **Priority Support**: High-priority tasks are processed first
- **Retry Logic**: Automatic retries with exponential backoff
- **Rate Limiting**: Per-task-type rate limiting to prevent overload
- **Task Dependencies**: Support for dependent tasks
- **Scheduled Tasks**: Future execution support

## Architecture

```
API Request → Create Task → Task Queue → Worker Pool → Task Handler → Completion
                                      ↓
                                 Rate Limiter
                                      ↓
                                 Retry Logic
```

## Components

### 1. Task Queue (`src/services/task_queue.rs`)

Main queue service that manages task submission and worker pool.

**Key Features:**
- Task submission with priority
- Worker pool management
- Task polling and distribution
- Shutdown handling

**Configuration:**
```rust
pub struct TaskQueueConfig {
    pub max_workers: usize,           // Default: 4
    pub poll_interval_ms: u64,         // Default: 1000ms
    pub default_timeout_seconds: i32,   // Default: 3600s
    pub max_retries: i32,              // Default: 3
}
```

### 2. Task Repository (`src/db/task.rs`)

Database operations for task management.

**Key Operations:**
- Create task
- Get task by ID
- List tasks with filters
- Update task status
- Get task statistics

**Task States:**
- `pending`: Queued, waiting to be processed
- `scheduled`: Scheduled for future execution
- `processing`: Currently being executed
- `completed`: Successfully completed
- `failed`: Failed (may be retried)
- `cancelled`: Cancelled before completion

### 3. Task Handlers (`src/services/task_handlers/`)

Handlers for different task types.

**Available Handlers:**
- `VideoTaskHandler`: Video transcoding to HLS
- `EmbeddingTaskHandler`: Semantic search embedding generation
- `PluginTaskHandler`: Plugin execution

**Handler Trait:**
```rust
#[async_trait]
pub trait TaskHandler {
    async fn process(
        &self,
        task: &Task,
        state: Arc<AppState>
    ) -> Result<serde_json::Value>;
}
```

### 4. Rate Limiter (`src/services/rate_limiter.rs`)

Per-task-type rate limiting to prevent system overload.

**Features:**
- Token bucket algorithm
- Per-task-type limits
- Configurable rates

## Task Types

### VideoTranscoding

Converts uploaded videos to HLS format for streaming.

**Payload:**
```json
{
  "video_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Handler:** `VideoTaskHandler`
**Typical Duration:** 1-10 minutes
**Rate Limit:** Configurable (default: 2 concurrent)

### EmbeddingGeneration

Generates semantic search embeddings for media files.

**Payload:**
```json
{
  "entity_type": "image",
  "entity_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Handler:** `EmbeddingTaskHandler`
**Typical Duration:** 5-30 seconds
**Rate Limit:** Configurable (default: 10 concurrent)

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

**Handler:** `PluginTaskHandler`
**Typical Duration:** Varies by plugin
**Rate Limit:** Configurable (default: 5 concurrent)

## Task Lifecycle

```
1. Create Task
   ↓
2. Status: pending (or scheduled if future)
   ↓
3. Worker picks up task
   ↓
4. Status: processing
   ↓
5. Task handler executes
   ↓
6a. Success → Status: completed
6b. Failure → Retry logic
   ↓
7. If retries exhausted → Status: failed
```

## Retry Logic

### Exponential Backoff

Failed tasks are retried with exponential backoff:
- Initial delay: 1 minute
- Backoff multiplier: 2x
- Max delay: 1 hour
- Max retries: Configurable (default: 3)

### Retry Calculation

```rust
let backoff_seconds = (2_u64.pow(retry_count as u32) * 60).min(3600);
let scheduled_at = Utc::now() + Duration::seconds(backoff_seconds);
```

### Retry Conditions

Tasks are retried if:
- Task status is `failed`
- `retry_count < max_retries`
- Task hasn't exceeded timeout

## Worker Pool

### Worker Loop

```rust
async fn worker_pool(
    repository: TaskRepository,
    rate_limiter: RateLimiter,
    config: TaskQueueConfig,
    state: Arc<AppState>,
    shutdown_rx: mpsc::Receiver<()>,
) {
    let semaphore = Semaphore::new(config.max_workers);
    
    loop {
        // Poll for available tasks
        let tasks = repository.get_pending_tasks().await?;
        
        for task in tasks {
            // Check rate limit
            if !rate_limiter.acquire(&task.task_type).await {
                continue; // Skip if rate limited
            }
            
            // Acquire worker slot
            let permit = semaphore.acquire().await?;
            
            // Spawn task handler
            tokio::spawn(async move {
                let _permit = permit;
                process_task(task, state).await;
            });
        }
        
        // Wait before next poll
        sleep(Duration::from_millis(config.poll_interval_ms)).await;
    }
}
```

### Concurrency Control

- **Semaphore**: Limits concurrent workers
- **Rate Limiter**: Limits per-task-type concurrency
- **Priority Queue**: High-priority tasks processed first

## Task Submission

### Basic Submission

```rust
let task_id = task_queue
    .submit_task(
        TaskType::VideoTranscoding,
        serde_json::json!({ "video_id": video_id }),
        Priority::Normal,
        None,  // scheduled_at
        None,  // depends_on
    )
    .await?;
```

### With Scheduling

```rust
let scheduled_at = Utc::now() + Duration::hours(1);
let task_id = task_queue
    .submit_task(
        TaskType::EmbeddingGeneration,
        payload,
        Priority::High,
        Some(scheduled_at),
        None,
    )
    .await?;
```

### With Dependencies

```rust
let task_id = task_queue
    .submit_task(
        TaskType::PluginExecution,
        payload,
        Priority::Normal,
        None,
        Some(vec![dependency_task_id]),
    )
    .await?;
```

## Priority Levels

- **High (0)**: Critical tasks, processed first
- **Normal (1)**: Standard tasks (default)
- **Low (2)**: Background tasks, processed last

Tasks are processed in priority order within each task type.

## Error Handling

### Task Failures

When a task handler fails:
1. Task status set to `failed`
2. Error message stored in task record
3. Retry scheduled if `retry_count < max_retries`
4. Task marked as permanently failed if retries exhausted

### Timeout Handling

Tasks that exceed `timeout_seconds`:
1. Task status set to `failed`
2. Error: "Task timeout"
3. Retry logic applies

### Handler Errors

Handlers should return `Result<serde_json::Value>`:
- `Ok(value)`: Task completed successfully, value stored in `result` field
- `Err(e)`: Task failed, error message stored, retry scheduled

## Monitoring

### Task Statistics

```rust
let stats = task_repo.get_task_stats(tenant_id).await?;
// Returns:
// - Total tasks
// - Tasks by status
// - Tasks by type
// - Average duration
// - Success rate
```

### Logging

All task operations are logged with:
- Task ID
- Task type
- Status changes
- Errors
- Duration

## Database Schema

**Tasks Table:**
```sql
CREATE TABLE tasks (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    task_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL,
    priority INTEGER NOT NULL,
    payload JSONB NOT NULL,
    result JSONB,
    scheduled_at TIMESTAMPTZ NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    timeout_seconds INTEGER,
    depends_on UUID[],
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_tasks_status ON tasks(status, scheduled_at);
CREATE INDEX idx_tasks_tenant ON tasks(tenant_id);
CREATE INDEX idx_tasks_type ON tasks(task_type);
```

## Configuration

### Environment Variables

```env
# Task queue configuration
TASK_QUEUE_MAX_WORKERS=4
TASK_QUEUE_POLL_INTERVAL_MS=1000
TASK_QUEUE_DEFAULT_TIMEOUT_SECONDS=3600
TASK_QUEUE_MAX_RETRIES=3

# Rate limits (per task type)
TASK_RATE_LIMIT_VIDEO_TRANSCODING=2
TASK_RATE_LIMIT_EMBEDDING_GENERATION=10
TASK_RATE_LIMIT_PLUGIN_EXECUTION=5

# Task cleanup: delete finished tasks older than N days (0 = disabled)
TASK_RETENTION_DAYS=30
```

## LISTEN/NOTIFY

When the task queue is created with a database pool, workers use PostgreSQL LISTEN/NOTIFY on the `mindia_new_task` channel. When a task is created, `pg_notify` is sent so workers wake immediately instead of waiting for the next poll interval. Polling still runs at `poll_interval_ms` for scheduled tasks and as a fallback.

## Task Cleanup

Finished tasks (completed, failed, cancelled) older than `TASK_RETENTION_DAYS` are automatically deleted by the cleanup service (runs hourly). Set `TASK_RETENTION_DAYS=0` to disable task cleanup. Default: 30 days.

## CPU-Bound Work in Handlers

Handlers that do CPU-intensive work (e.g. document parsing, image processing) should run that work inside `tokio::task::spawn_blocking` so it does not block the async runtime. See the `TaskHandler` trait documentation in the API crate.

## Best Practices

1. **Task Payloads**: Keep payloads small, reference data by ID
2. **Idempotency**: Make handlers idempotent for safe retries
3. **Timeouts**: Set appropriate timeouts for each task type
4. **Monitoring**: Monitor task success rates and durations
5. **Rate Limits**: Configure rate limits to prevent system overload
6. **Error Handling**: Provide clear error messages for debugging
7. **Logging**: Log all task state changes for observability

## Adding New Task Types

1. **Define Task Type:**
```rust
// In src/models/task.rs
#[derive(Debug, Clone, ...)]
pub enum TaskType {
    // ... existing types
    MyNewTaskType,
}
```

2. **Create Handler:**
```rust
// In src/services/task_handlers/my_handler.rs
pub struct MyTaskHandler;

#[async_trait]
impl TaskHandler for MyTaskHandler {
    async fn process(&self, task: &Task, state: Arc<AppState>) -> Result<serde_json::Value> {
        // Implementation
    }
}
```

3. **Register Handler:**
```rust
// In task queue setup
let my_handler = Arc::new(MyTaskHandler);
// Add to handler registry
```

4. **Update Database:**
```sql
-- Add new task type to enum if using PostgreSQL enum type
-- Or ensure VARCHAR column accepts new type
```

## Related Documentation

- [Tasks](../user/tasks.md) - User guide for task management
- [Video Processing](media-processing.md) - Video transcoding details
- [Embeddings System](embeddings-system.md) - Embedding generation
- [Plugins](plugins.md) - Plugin execution system

