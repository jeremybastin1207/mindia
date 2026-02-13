# Mindia

[Documentation](https://jeremybastin1207.github.io/mindia/) · [License](LICENSE)

[![dependency status](https://deps.rs/repo/github/jeremybastin1207/mindia/status.svg)](https://deps.rs/repo/github/jeremybastin1207/mindia) [![License](https://img.shields.io/github/license/jeremybastin1207/mindia)](LICENSE) [![Rust](https://img.shields.io/badge/rust-lang-orange)](https://www.rust-lang.org/)

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
- Workflows: auto-run plugin pipelines on upload (e.g. moderation → object detection → colorization) with filters (media type, folder, content type)
- Multi-tenant architecture with API key authentication
- Webhooks for event notifications
- OpenTelemetry observability
- Optional ClamAV virus scanning

## Documentation

Full documentation is available at [https://jeremybastin1207.github.io/mindia/](https://jeremybastin1207.github.io/mindia/) including quick start, API reference, configuration, and feature guides.

## Contributing

Contributions are welcome! Please open an [issue](https://github.com/jeremybastin1207/mindia/issues) for bugs and feature requests, or submit a pull request.

## Versioning

Releases are published as [GitHub releases](https://github.com/jeremybastin1207/mindia/releases) and tagged following [SemVer](https://semver.org/). See [CHANGELOG.md](CHANGELOG.md) for details.

## License

MIT License. See [LICENSE](LICENSE) for details.
