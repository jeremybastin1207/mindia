# Integration Tests

These integration tests are **not currently run** by `cargo test`. They target the former `mindia` crate and use `mindia::*` imports; the project now uses the `mindia-api` workspace crate.

To re-enable them:

1. Wire them to a package (e.g. add a `mindia-tests` dev crate or `[[test]]` entries in `mindia-api`).
2. Update imports from `mindia::*` to `mindia_api::*` (or the appropriate crate).
3. Ensure `sqlx::migrate!` points at `./migrations` or `../../migrations` relative to the crate that runs them.
