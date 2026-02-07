# Integration Tests

## Active integration tests (mindia-api)

Integration tests that run with `cargo test` live under **`crates/mindia-api/tests/`**. They use the real `AppState`, `mindia_core::Config`, and `setup_routes` from the API.

**Run from workspace root:**

```bash
cargo test -p mindia-api --test images_test
# or all integration tests:
cargo test -p mindia-api --tests
```

**Requirements:**

- Docker (for testcontainers Postgres)
- Same feature set as the API (default features include image, video, audio, document, storage-local, clamav, plugin, workflow)

**Layout:**

- `crates/mindia-api/tests/images_test.rs` – image upload, list, workflow
- `crates/mindia-api/tests/helpers/` – shared setup: `setup_test_app()`, `create_test_config()`, auth/fixtures/workflows

Migrations are loaded from `../../migrations` (workspace root) when the test runs.

## Legacy tests in `tests/` (root)

The files in the repo-root **`tests/`** directory are **not** run by `cargo test`. They still use the old `mindia::*` imports and the previous flat `AppState`. They are kept for reference; the canonical integration tests are in `crates/mindia-api/tests/`.
