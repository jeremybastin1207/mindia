# Documentation Sync

**Documentation must always be updated after code changes.**

When modifying code—whether adding features, changing behavior, refactoring, or fixing bugs—you must also update the relevant documentation to stay in sync.

## What to Update

- **README.md** – Installation steps, usage, or project overview changes
- **docs/** – API docs, guides, and configuration references
- **Inline comments / doc comments** – Rust `///` and `//!` doc comments when public APIs change
- **CHANGELOG.md** – User-facing or notable changes

## Checklist

After any code change, ask:

1. Did I add, remove, or change any public API? → Update API docs and doc comments.
2. Did I change behavior users rely on? → Update README, guides, or CHANGELOG.
3. Did I change configuration or env vars? → Update `docs/configuration.md` and `.env.example`.

Do not consider a change complete until documentation is updated.
