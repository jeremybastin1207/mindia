# Mindia Developer Documentation

Welcome to the Mindia developer documentation! This guide is for developers and architects working on Mindia itself - extending features, fixing bugs, or understanding the internals.

## For API Users

If you're building applications that **use** Mindia's API (not developing Mindia itself), see the [User Documentation](../user/README.md) instead.

## Quick Navigation

### Architecture & Design
- [Architecture Overview](architecture.md) - System design and component structure
- [Tech Stack](tech-stack.md) - Technologies used and rationale
- [Code Structure](code-structure.md) - Project organization and crate responsibilities
- [Database Schema](database-schema.md) - Complete schema with ER diagrams

### Getting Started
- [Development Setup](development-setup.md) - Local environment setup
- [Contributing Guide](contributing.md) - How to contribute

### Core Systems
- [Authentication System](authentication-system.md) - JWT, middleware, API keys, and tenant context
- [Job Queue](job-queue.md) - Background task queue with LISTEN/NOTIFY and worker pool
- [Webhook System](webhook-system.md) - Webhook delivery and retry logic
- [Transaction Patterns](transaction-patterns.md) - Database transaction handling

### Deployment & Operations
- [Deployment Guide](deployment.md) - Production deployment (Docker, Kubernetes, Fly.io)
- [Monitoring](monitoring.md) - OpenTelemetry, logging, and metrics
- [Production Runbook](production-runbook.md) - Operations guide
- [Backup & Recovery](backup-recovery.md) - Database backup strategies
- [Secrets Management](secrets-management.md) - Managing secrets and credentials

### Performance & Testing
- [Performance Guide](performance.md) - Optimization techniques
- [Testing](testing.md) - Testing strategies and guidelines
- [Load Testing](load-testing.md) - Load testing procedures
- [Test Coverage](test-coverage.md) - Test coverage analysis

### Security & Compliance
- [Security Audit](security-audit.md) - Security review findings
- [Security Audit Completion](security-audit-completion.md) - Audit completion report
- [Wide Events](wide-events.md) - Event tracking system

### Other
- [API Versioning](api-versioning.md) - API versioning strategy
- [Database Migrations](database-migrations.md) - Migration workflow
- [Releasing](releasing.md) - Release process
- [Comment Style Guide](comment-style-guide.md) - Code documentation standards
- [Service Boundaries](service-boundaries.md) - Service separation principles

### Historical Documentation
- [Historical Docs](../historical/README.md) - Completed refactorings and migrations (archived)

## Project Structure

```
mindia/
├── src/
│   ├── auth/              # Authentication & authorization
│   ├── db/                # Database repositories
│   ├── handlers/          # HTTP request handlers
│   ├── middleware/        # Axum middleware
│   ├── models/            # Data models
│   ├── services/          # Business logic services
│   └── main.rs            # Application entry point
├── migrations/            # SQL migrations
├── doc/                   # Documentation
└── Cargo.toml             # Rust dependencies
```

## Key Technologies

- **Axum** - Web framework
- **Tokio** - Async runtime
- **PostgreSQL + pgvector** - Database with vector search
- **AWS S3** - Object storage
- **FFmpeg** - Video transcoding
- **Anthropic (Claude)** - Embeddings and vision for semantic search
- **OpenTelemetry** - Observability

## Development Workflow

1. **Read** [Development Setup](development-setup.md) to configure your environment
2. **Explore** [Code Structure](code-structure.md) to understand the codebase
3. **Review** [Architecture](architecture.md) to understand the design
4. **Contribute** following the [Contributing Guide](contributing.md)

## Getting Help

- Check [Production Runbook](production-runbook.md) for operational guidance
- Review existing GitHub issues
- Read the [Architecture](architecture.md) docs to understand the system

