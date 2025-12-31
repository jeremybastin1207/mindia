# Mindia

> **Warning: This repository is under active development and should not be used in production.**

> **Note:** This codebase was built with the assistance of AI tools.

A high-performance Rust-based media management service. Upload and manage images, videos, documents, and audio files with S3 storage, on-the-fly transformations, HLS streaming, and semantic search.

## Quick Start

```bash
cp .env.example .env  # Configure your environment
cargo run -p mindia-api
```

## Features

- Image upload with on-the-fly resizing and transformations
- Video upload with HLS adaptive streaming
- Audio and document management
- Semantic search with AI embeddings
- Plugin system for AI/ML integrations (Claude Vision, AWS Rekognition, AssemblyAI, etc.)
- Multi-tenant architecture with API key authentication
- Webhooks for event notifications
- OpenTelemetry observability
- Optional ClamAV virus scanning

## Documentation

| Audience | Link |
|----------|------|
| API Users | [User Documentation](doc/user/README.md) |
| Developers | [Developer Documentation](doc/developer/README.md) |

## Tech Stack

Rust, Axum, Tokio, PostgreSQL, AWS S3, FFmpeg

## License

MIT License. See [LICENSE](LICENSE) for details.
