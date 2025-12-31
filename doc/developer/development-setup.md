# Development Setup

Guide to setting up Mindia for local development.

## Prerequisites

- Rust 1.75+ (install from [rustup.rs](https://rustup.rs))
- PostgreSQL 15+ with pgvector
- FFmpeg 4.0+
- (Optional) Anthropic API key for semantic search
- (Optional) ClamAV for virus scanning

## Quick Setup

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clone repository
git clone <repo-url>
cd mindia

# 3. Install PostgreSQL with pgvector
# macOS
brew install postgresql@15
brew services start postgresql@15

#  Ubuntu
sudo apt install postgresql-15 postgresql-contrib-15

# 4. Create database
createdb mindia
psql -d mindia -c "CREATE EXTENSION vector;"

# 5. Configure environment
cp .env.example .env
# Edit .env with your settings

# 6. Run migrations (automatic on startup)
cargo run

# 7. Access API
curl http://localhost:3000/health
```

## Development Workflow

### Running the Server

```bash
# Development mode (faster compilation)
cargo run

# With debug logging
RUST_LOG=debug cargo run

# Release mode (optimized)
cargo build --release
./target/release/mindia
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Run tests
cargo test

# Check compilation
cargo check
```

### Database Migrations

```bash
# Migrations run automatically on startup
# Or manually:
sqlx migrate run

# Create new migration
sqlx migrate add migration_name
```

## Project Structure

```
mindia/
├── src/                       # Multi-crate workspace
│   ├── mindia-core/           # Domain models, types, config
│   ├── mindia-db/             # Database repositories
│   ├── mindia-services/       # External service clients (S3, Anthropic, ClamAV)
│   ├── mindia-storage/        # Storage abstraction (S3, local)
│   ├── mindia-processing/     # Media processing (image, video, audio, document)
│   ├── mindia-infra/          # Infrastructure (middleware, webhooks, analytics)
│   ├── mindia-worker/         # Background task queue
│   ├── mindia-plugins/        # Plugin system
│   ├── mindia-api/            # Main API service
│   ├── mindia-cli/            # Command-line tools
│   └── mindia-mcp/            # Model Context Protocol server
├── migrations/                # SQL migrations
├── doc/                       # Documentation
└── Cargo.toml                 # Workspace definition
```

## IDE Setup

### VS Code

Recommended extensions:
- rust-analyzer
- CodeLLDB (debugging)
- Even Better TOML
- Error Lens

### IntelliJ IDEA/CLion

Install Rust plugin from JetBrains.

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Integration tests
cargo test --test integration
```

## Debugging

### VS Code launch.json

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Mindia",
      "cargo": {
        "args": ["build", "--bin=mindia"]
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug"
      }
    }
  ]
}
```

## Common Issues

### Linker Errors (Windows)

Install Visual Studio Build Tools with C++ development workload.

### Linker SIGBUS on Linux (`ld terminated with signal 7 [Bus error]`)

On some systems, linking the `mindia-api` test binary can crash with a Bus error when using the default LLVM linker (lld). The project includes a workaround in [`.cargo/config.toml`](../../.cargo/config.toml) that forces the GNU BFD linker (`-fuse-ld=bfd`). If you still see the error, ensure the `binutils` package is installed (e.g. `apt install binutils`). To try lld again, comment out the `[target.x86_64-unknown-linux-gnu]` section in `.cargo/config.toml`.

### No space left on device

A full build and tests use several GB of disk (debug artifacts, dependencies). If you see `No space left on device (os error 28)`, free space or run `cargo clean` and then a smaller build (e.g. `cargo test -p mindia-core`).

### Database Connection

Ensure PostgreSQL is running and DATABASE_URL is correct.

### S3 Errors

Check AWS credentials and bucket permissions.

## Next Steps

- [Code Structure](code-structure.md) - Project organization
- [Contributing](contributing.md) - Contribution guidelines
- [Architecture](architecture.md) - System design

