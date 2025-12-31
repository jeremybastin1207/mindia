# Unrecoverable Task Error Handling

## Overview

The task system now supports distinguishing between **recoverable** and **unrecoverable** errors. This prevents tasks from retrying indefinitely when they encounter errors that won't resolve with retries, such as missing API keys or invalid configuration.

## What are Unrecoverable Errors?

Unrecoverable errors are failures that won't be fixed by retrying the task. Common examples include:

- **Missing or invalid API keys/credentials**: If a plugin requires an API key that hasn't been configured, retrying won't help until the configuration is fixed.
- **Invalid configuration**: Malformed configuration values that need manual correction.
- **Authorization/permission errors**: Access denied errors that require manual intervention.
- **Invalid input data**: Data validation failures where the input data itself is incorrect.

## How It Works

### TaskError Type

A new `TaskError` type was introduced in `mindia-core` that wraps `anyhow::Error` and adds a `recoverable` flag:

```rust
use mindia_core::{TaskError, TaskResultExt};

// Create an unrecoverable error
let err = TaskError::unrecoverable(anyhow::anyhow!("Missing API key"));

// Create a recoverable error (default)
let err = TaskError::recoverable(anyhow::anyhow!("Network timeout"));

// Check if an error is recoverable
if err.is_recoverable() {
    // Will retry
} else {
    // Will fail immediately
}
```

### Using in Task Handlers

Task handlers can return `TaskError` converted to `anyhow::Error` (via automatic `From` implementation):

```rust
async fn process(&self, task: &Task, state: Arc<AppState>) -> Result<serde_json::Value> {
    // Configuration errors should not be retried
    let config = get_config()
        .ok_or_else(|| {
            TaskError::unrecoverable(anyhow::anyhow!(
                "Plugin not configured. Please add API key."
            ))
        })?
        .into(); // Convert TaskError to anyhow::Error

    // Transient errors can be retried (default)
    let data = download_file().await?;
    
    Ok(json!({ "result": data }))
}
```

### Plugin Configuration Validation

Plugins now validate their configuration in the `validate_config` method, checking for:

1. **Missing required fields**: API keys, credentials, endpoints
2. **Invalid placeholder values**: Default values like `"your-api-key"`
3. **Malformed values**: Empty strings, values that are too short

Example from `assembly_ai.rs`:

```rust
fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
    let config: AssemblyAiConfig = serde_json::from_value(config.clone())
        .context("Invalid Assembly AI configuration: missing or invalid fields")?;

    // Validate API key is present and not a placeholder
    if config.api_key.is_empty() {
        anyhow::bail!("Assembly AI API key is required but not provided");
    }
    
    if config.api_key == "your-api-key" || config.api_key.len() < 10 {
        anyhow::bail!("Assembly AI API key appears to be invalid or a placeholder");
    }

    Ok(())
}
```

### Task Processing Flow

When a task fails:

1. The task queue checks if the error is a `TaskError` with `is_recoverable() == false`
2. If **unrecoverable**: Mark task as failed immediately, skip all retries
3. If **recoverable**: Retry according to the task's retry policy (default: 3 retries with exponential backoff)

Example from `queue.rs`:

```rust
Ok(Err(e)) => {
    // Check if this is a TaskError with unrecoverable flag
    let is_unrecoverable = e.downcast_ref::<TaskError>()
        .map(|te| !te.is_recoverable())
        .unwrap_or(false);

    if is_unrecoverable {
        // Fail immediately without retrying
        repository.mark_failed(task.id, error_result).await?;
        return Err(e);
    }

    // Otherwise, retry if within retry limit
    if task.can_retry() {
        repository.increment_retry(task.id).await?;
    }
}
```

## Error Result Format

When a task fails with an unrecoverable error, the result JSON includes:

```json
{
  "error": "Plugin 'claude_vision' configuration is invalid: API key is required but not provided",
  "retry_count": 0,
  "unrecoverable": true,
  "reason": "Task failed with unrecoverable error (e.g., missing configuration, invalid credentials)"
}
```

This helps users understand:
- Why the task failed
- Why it wasn't retried
- What action to take (configure the plugin, add API key, etc.)

## Best Practices

### When to Use Unrecoverable Errors

Use `TaskError::unrecoverable()` for:

- ✅ Missing API keys, tokens, or credentials
- ✅ Invalid configuration that requires user intervention
- ✅ Authorization/permission denied errors
- ✅ Invalid input data that won't change
- ✅ Plugin not configured/enabled
- ✅ Validation errors for configuration

### When to Use Recoverable Errors (Default)

Use regular `anyhow::Error` or `TaskError::recoverable()` for:

- ✅ Network timeouts and connection errors
- ✅ Temporary API rate limiting
- ✅ Transient service unavailability
- ✅ Database connection issues
- ✅ Temporary resource constraints

### Example: Updating a Plugin

```rust
use mindia_core::TaskError;

async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
    // Parse and validate configuration
    let config: MyPluginConfig = serde_json::from_value(context.config.clone())
        .context("Failed to parse plugin configuration")?;
    
    // Check for configuration issues (unrecoverable)
    if config.api_key.is_empty() {
        return Ok(PluginResult {
            status: PluginExecutionStatus::Failed,
            error: Some("API key not configured. Please configure the plugin.".to_string()),
            // ... other fields
        });
    }
    
    // API call (recoverable if it fails due to network issues)
    let response = self.http_client
        .post(&format!("{}/analyze", config.api_endpoint))
        .json(&payload)
        .send()
        .await
        .context("Failed to call API")?;
    
    // Process response...
    Ok(PluginResult {
        status: PluginExecutionStatus::Success,
        data: result,
        // ... other fields
    })
}
```

## Testing

Test cases for `TaskError`:

```rust
#[test]
fn test_unrecoverable_error() {
    let err = TaskError::unrecoverable(anyhow::anyhow!("Missing API key"));
    assert!(!err.is_recoverable());
}

#[test]
fn test_recoverable_error() {
    let err = TaskError::recoverable(anyhow::anyhow!("Network timeout"));
    assert!(err.is_recoverable());
}

#[test]
fn test_from_anyhow() {
    let err: TaskError = anyhow::anyhow!("Some error").into();
    assert!(err.is_recoverable(), "Default should be recoverable");
}
```

## Updated Components

The following components were updated to support unrecoverable errors:

1. **mindia-core**: New `TaskError` type and `TaskResultExt` trait
2. **mindia-worker**: Task queue processing logic to detect and handle unrecoverable errors
3. **mindia-api**: Plugin handler to wrap configuration errors as unrecoverable
4. **mindia-plugins**: Updated `validate_config` methods to check for missing/invalid API keys:
   - `assembly_ai.rs`
   - `claude_vision.rs`
   - `replicate_deoldify.rs`

## Migration Guide

If you have custom task handlers or plugins:

1. **Update plugin validation**: Add checks for missing/invalid API keys in `validate_config()`
2. **Use TaskError for config issues**: Wrap configuration errors with `TaskError::unrecoverable()`
3. **Keep transient errors recoverable**: Let network/service errors use default retry behavior

No changes are required for existing code - all errors are recoverable by default.
