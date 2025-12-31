# Releasing Mindia

This document describes the standard release process for the Mindia project.

Releases are:
- **Versioned** using semantic versioning (SemVer)
- **Tagged** in git as `vMAJOR.MINOR.PATCH`
- **Built and published** via GitHub Actions, which creates a GitHub Release and attaches Linux binaries

For details on SemVer, see <https://semver.org/>.

## Quick Start

To cut a new release:

```bash
# 1. Make sure your branch is up to date
git checkout main
git pull origin main

# 2. Run tests and checks locally (optional but recommended)
make ci

# 3. Bump the version and create a tag
scripts/bump-version.sh 0.2.0

# 4. Push the new tag (triggers the release workflow)
git push origin v0.2.0
```

GitHub Actions will:
- Run CI checks (`make ci-full`)
- Build a release binary
- Compute checksums
- Create a GitHub Release with auto-generated release notes
- Attach the binary and checksum files

## Versioning Strategy

Mindia uses a **single version** for the entire workspace, defined in the root `Cargo.toml` under `[workspace.package]`.

- All crates use `version.workspace = true`
- Bumping the workspace version updates the effective version for every crate

### Semantic Versioning

We follow SemVer:

- **MAJOR**: breaking API or behavior changes
- **MINOR**: new features that are backwards compatible
- **PATCH**: bug fixes and small, backwards compatible improvements

Pre-release versions are supported:

- `1.2.3-alpha.1`
- `1.2.3-beta.2`
- `1.2.3-rc.1`

These can be used for testing before a stable release.

## Version Bump Script

Use `scripts/bump-version.sh` to change the workspace version, commit the change, and create a tag.

### Usage

```bash
scripts/bump-version.sh [OPTIONS] <new-version>
```

Examples:

```bash
scripts/bump-version.sh 0.2.0
scripts/bump-version.sh v0.2.0
scripts/bump-version.sh --dry-run 0.2.0
```

### Options

- `--dry-run` – Show what would change, but do not modify files or git history
- `--no-commit` – Update files only, do not create a git commit
- `--no-tag` – Do not create a git tag
- `-h, --help` – Show usage

### What It Does

The script:

1. Validates that `<new-version>` is a semantic version (`MAJOR.MINOR.PATCH` with optional `-prerelease`).
2. Updates the workspace version in the root `Cargo.toml`.
3. Updates `fly.toml` `version` field if present.
4. Creates a commit: `chore(release): bump version to <new-version>`.
5. Creates an annotated git tag: `v<new-version>`.
6. Optionally prompts to push the tag to `origin`.

> **Tip:** Use `--dry-run` to check what would change before actually bumping the version.

## GitHub Release Workflow

Releases are automated via `.github/workflows/release.yml`.

### Trigger

The workflow runs on:

```yaml
on:
  push:
    tags:
      - "v*.*.*"
```

Any push of a tag like `v0.2.0` will:

1. Run CI checks (formatting, linting, tests, build).
2. Build the `mindia-api` release binary with `RUSTFLAGS="-C target-cpu=native"`.
3. Create a GitHub Release with:
   - Title: the tag name (e.g. `v0.2.0`)
   - Auto-generated release notes (based on commits and PRs)
   - Attached artifacts:
     - `mindia-api-linux-amd64`
     - `mindia-api-linux-amd64.sha256`

### Requirements

- The repository must be hosted on GitHub.
- The default `GITHUB_TOKEN` secret is used by the workflow (no extra secrets required for release itself).

## Release Checklist

Before creating a release:

1. **Ensure main is green**
   - All CI checks pass on `main`.
2. **Review CHANGELOG**
   - Update `CHANGELOG.md` `[Unreleased]` section as needed.
   - Optionally move entries into a new `## [x.y.z]` section.
3. **Confirm docs**
   - Update user and developer docs relevant to the changes.
4. **Check breaking changes**
   - If there are any, bump **MAJOR** version and document them clearly.

Then:

```bash
git checkout main
git pull origin main
scripts/bump-version.sh 0.2.0
git push origin v0.2.0
```

GitHub Actions will handle the rest.

## Pre-releases

You can create pre-releases using a pre-release suffix:

```bash
scripts/bump-version.sh 1.0.0-rc.1
git push origin v1.0.0-rc.1
```

Notes:

- The workflow will run just like for stable releases.
- Mark the release as a pre-release manually in the GitHub UI if desired.

## Troubleshooting

### Workflow Fails on CI Checks

- Inspect the failing job in GitHub Actions.
- Fix issues (formatting, clippy, tests, etc.).
- Merge the fixes into `main`.
- You can either:
  - Delete the failed tag and re-tag, or
  - Create a new patch release with the fixes.

### Binary Not Attached

If the GitHub Release is created without a binary:

1. Check the `Build release binary` and `Compute checksums` steps in the workflow logs.
2. Ensure:
   - `cargo build --release -p mindia-api` succeeds locally.
   - The expected binary exists at `target/release/mindia-api`.

### Tag Pushed by Mistake

If you accidentally push a tag:

1. Delete the tag locally and remotely:

   ```bash
   git tag -d vX.Y.Z
   git push origin :refs/tags/vX.Y.Z
   ```

2. Optionally delete the corresponding GitHub Release in the UI.

## How This Integrates with Contributing

For contributors:

- Keep `CHANGELOG.md` up to date when you add features or fix bugs.
- Follow conventional commit messages and clear PR titles to improve auto-generated release notes.
- Major or minor release decisions are typically made by maintainers.

For maintainers:

- Use this document as the authoritative guide for cutting new releases.
- Link back here from the contributing guide so the process is discoverable.

