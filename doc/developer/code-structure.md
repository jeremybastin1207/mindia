# Code Structure

Organization and conventions of the Mindia codebase.

## Crate Structure

Mindia is organized as a multi-crate workspace. Each crate has a specific responsibility:

```
mindia/
├── mindia-core/               # Core domain models, types, and traits
│   ├── src/
│   │   ├── config.rs          # Configuration types
│   │   ├── error.rs           # Error types
│   │   ├── models/            # Domain models (media, user, tenant, etc.)
│   │   ├── messaging.rs       # Messaging types
│   │   └── validation/        # Validation logic
│   └── Cargo.toml
│
├── mindia-db/                 # Database repositories
│   ├── src/
│   │   ├── db/                # Repository implementations
│   │   │   ├── media/         # Media repositories
│   │   │   ├── control/       # Control plane repositories
│   │   │   └── ...
│   │   └── lib.rs
│   └── Cargo.toml
│
├── mindia-services/           # External service integrations
│   ├── src/
│   │   ├── services/
│   │   │   ├── s3.rs          # S3 client
│   │   │   ├── anthropic.rs   # Anthropic (Claude) client
│   │   │   └── clamav.rs      # ClamAV client
│   │   └── lib.rs
│   └── Cargo.toml
│
├── mindia-storage/            # Storage abstraction layer
│   ├── src/
│   │   ├── traits.rs          # Storage trait definitions
│   │   ├── factory.rs         # Storage factory
│   │   ├── s3.rs              # S3 implementation
│   │   └── local.rs           # Local filesystem implementation
│   └── Cargo.toml
│
├── mindia-processing/         # Media processing logic
│   ├── src/
│   │   ├── image/             # Image processing
│   │   ├── video/             # Video processing
│   │   ├── audio/             # Audio processing
│   │   ├── document/          # Document processing
│   │   ├── upload/            # Upload pipeline
│   │   └── validator.rs       # File validation
│   └── Cargo.toml
│
├── mindia-infra/              # Infrastructure components
│   ├── src/
│   │   ├── middleware/        # HTTP middleware (request ID, security headers, CSRF)
│   │   ├── telemetry/         # OpenTelemetry setup
│   │   ├── webhook/           # Webhook delivery and retry
│   │   ├── analytics/         # Analytics collection
│   │   ├── rate_limit/        # Rate limiting (sharded, sliding window)
│   │   ├── cleanup/           # Cleanup services (files, tasks)
│   │   ├── capacity/          # Capacity checking (disk, memory, CPU)
│   │   └── archive/           # Archive creation (ZIP, TAR)
│   └── Cargo.toml
│
├── mindia-worker/             # Background task queue
│   ├── src/
│   │   ├── queue.rs           # Task queue with worker pool
│   │   ├── context.rs         # Task handler context trait
│   │   └── lib.rs             # Public API
│   └── Cargo.toml
│
├── mindia-plugins/            # Plugin system
│   ├── src/
│   │   ├── registry.rs        # Plugin registry
│   │   ├── assembly_ai.rs     # AssemblyAI plugin
│   │   ├── aws_rekognition.rs # AWS Rekognition plugin
│   │   └── ...                # Other plugin implementations
│   └── Cargo.toml
│
├── mindia-api/                # Main API service
│   ├── src/
│   │   ├── main.rs            # Application entry point
│   │   ├── handlers/          # HTTP request handlers
│   │   ├── middleware/        # Service-specific middleware
│   │   ├── services/          # Business logic services
│   │   ├── setup/             # Server setup and initialization
│   │   ├── task_handlers/     # Background job handlers
│   │   ├── task_dispatch.rs   # Task dispatcher (TaskHandlerContext impl)
│   │   └── state.rs           # Application state
│   └── Cargo.toml
│
├── mindia-cli/                # CLI tools
│   ├── src/
│   │   ├── bin/               # CLI binaries
│   │   │   ├── list_media.rs  # List media command
│   │   │   └── media_stats.rs # Media stats command
│   │   └── lib.rs             # Shared CLI utilities
│   └── Cargo.toml
│
└── mindia-mcp/                # MCP (Model Context Protocol) server
    ├── src/
    │   ├── server.rs          # MCP server implementation
    │   ├── tools.rs           # MCP tools
    │   └── main.rs            # Entry point
    └── Cargo.toml
```

## Service Structure (mindia-api example)

Within the main service crate (`mindia-api`), the structure follows a layered architecture:

```
mindia-api/src/
├── main.rs                    # Application entry point
├── setup/                     # Server setup and configuration
│   ├── server.rs              # Server initialization
│   ├── routes.rs              # Route definitions
│   ├── services.rs            # Service initialization
│   └── database.rs            # Database setup
├── handlers/                  # HTTP request handlers
│   ├── image_upload.rs        # Image upload endpoint
│   ├── video_upload.rs        # Video upload endpoint
│   ├── audio_upload.rs        # Audio upload endpoint
│   ├── media_get.rs           # Media retrieval
│   ├── media_delete.rs        # Media deletion
│   ├── folders.rs             # Folder management
│   ├── plugins.rs             # Plugin endpoints
│   └── ...
├── middleware/                # HTTP middleware
│   ├── analytics.rs           # Analytics tracking
│   ├── audit.rs               # Audit logging
│   ├── idempotency.rs         # Idempotency handling
│   ├── rate_limit.rs          # Rate limiting
│   └── ...
├── services/                  # Business logic services
│   ├── upload/                # Upload processing pipeline
│   │   ├── service.rs         # Main upload service
│   │   ├── processor_adapter.rs # Media processor adapter
│   │   └── ...
│   └── usage.rs               # Usage tracking
├── task_handlers/             # Background job handlers
│   ├── video_handler.rs       # Video transcoding
│   ├── embedding_handler.rs   # Embedding generation
│   ├── plugin_handler.rs      # Plugin execution
│   └── ...
├── task_dispatch.rs           # Task dispatcher (implements TaskHandlerContext)
├── state.rs                   # Application state (AppState)
└── lib.rs                     # Public library API
```

## Crate Responsibilities

### mindia-core
- **Purpose**: Foundation crate with domain models, types, and traits
- **Dependencies**: Minimal (only external crates)
- **Contains**: Models, error types, configuration types, validation logic
- **Used by**: All other crates

### mindia-db
- **Purpose**: Database access layer
- **Dependencies**: `mindia-core`, `sqlx`
- **Contains**: Repository implementations for all database operations
- **Pattern**: Repository pattern with tenant isolation

### mindia-services
- **Purpose**: External service integrations
- **Dependencies**: `mindia-core`
- **Contains**: S3, Anthropic/Claude, ClamAV

### mindia-storage
- **Purpose**: Storage abstraction
- **Dependencies**: `mindia-core`
- **Contains**: Storage trait definitions and implementations (S3, local)

### mindia-processing
- **Purpose**: Media processing logic
- **Dependencies**: `mindia-core`
- **Contains**: Image, video, audio, document processing, transformations, validation

### mindia-infra
- **Purpose**: Shared infrastructure components
- **Dependencies**: `mindia-core`, `mindia-db` (optional), `mindia-storage` (optional)
- **Contains**: Middleware, telemetry, webhooks, analytics, rate limiting, cleanup, capacity checking

### mindia-worker
- **Purpose**: Background task queue infrastructure
- **Dependencies**: `mindia-core`, `mindia-db`, `mindia-infra`
- **Contains**: Task queue with worker pool, LISTEN/NOTIFY, retry logic, rate limiting
- **Used by**: `mindia-api` (implements `TaskHandlerContext` trait)

### mindia-api
- **Purpose**: Main API service (formerly mindia-media-api)
- **Dependencies**: All other crates
- **Contains**: HTTP handlers, business logic, task handlers, authentication

## Layer Architecture (within services)

### 1. Handlers (handlers/)

HTTP request handlers:
- Extract request data
- Call services
- Return responses
- Handle errors

**Pattern**:
```rust
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(payload): Json<RequestType>,
) -> Result<Json<ResponseType>, AppError> {
    // Business logic
    Ok(Json(response))
}
```

### 2. Services (services/)

Business logic:
- No HTTP concerns
- Reusable
- Testable
- Pure functions where possible

### 3. Repositories (mindia-db)

Data access:
- SQL queries
- Transaction handling
- Connection pooling

**Pattern**:
```rust
impl ImageRepository {
    pub async fn create(&self, ...) -> Result<Image, sqlx::Error> {
        sqlx::query_as!(...)
            .fetch_one(&self.pool)
            .await
    }
}
```

### 4. Models (mindia-core)

Data structures:
- Request/response types
- Database entities
- Business domain models

## Naming Conventions

### Files

- Snake_case: `image_processor.rs`
- One concept per file
- Group related items in modules

### Types

- PascalCase: `ImageProcessor`
- Descriptive names
- Suffix with type: `ImageResponse`, `UploadError`

### Functions

- Snake_case: `process_image()`
- Verb-first: `create_user()`, `get_image()`
- Async functions: `async fn fetch_data()`

### Constants

- UPPER_SNAKE_CASE: `MAX_FILE_SIZE`

## Error Handling

### AppError Type

```rust
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    Internal(String),
    // ...
}
```

### Usage

```rust
// Return specific errors
if user.is_none() {
    return Err(AppError::NotFound("User not found".to_string()));
}

// Convert from other errors
let image = repo.get_image(id)
    .await
    .map_err(AppError::Database)?;
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function() {
        assert_eq!(result, expected);
    }
}
```

### Integration Tests

Located in `tests/` directory.

## Async Patterns

### Spawn Blocking

For CPU-intensive work:

```rust
let result = tokio::task::spawn_blocking(move || {
    // CPU-intensive work
    process_image(data)
}).await?;
```

### Concurrent Operations

```rust
let (result1, result2) = tokio::join!(
    async_operation1(),
    async_operation2(),
);
```

## Next Steps

- [Development Setup](development-setup.md) - Get started
- [Contributing](contributing.md) - Contribution guidelines
- [Testing Guide](testing-guide.md) - Writing tests

