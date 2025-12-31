#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<EOF
Usage: scripts/bump-version.sh [OPTIONS] <new-version>

Bump the Mindia workspace version, commit the change, and create a git tag.

Examples:
  scripts/bump-version.sh 0.2.0
  scripts/bump-version.sh v0.2.0
  scripts/bump-version.sh --dry-run 0.2.0

Options:
  --dry-run     Show what would change but do not modify files or git history.
  --no-commit   Do not create a git commit (only update files).
  --no-tag      Do not create a git tag.
  -h, --help    Show this help message.

Notes:
  - Version must follow semantic versioning: MAJOR.MINOR.PATCH
    Optional pre-release suffixes are supported (e.g. 1.2.3-alpha.1).
  - All crates use the workspace version defined in Cargo.toml.
EOF
}

DRY_RUN=0
DO_COMMIT=1
DO_TAG=1

ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --no-commit)
      DO_COMMIT=0
      shift
      ;;
    --no-tag)
      DO_TAG=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ ${#ARGS[@]} -ne 1 ]]; then
  echo "Error: expected exactly one <new-version> argument." >&2
  echo >&2
  usage
  exit 1
fi

RAW_VERSION="${ARGS[0]}"

# Strip leading 'v' if present (e.g. v0.2.0 -> 0.2.0)
NEW_VERSION="${RAW_VERSION#v}"

# Basic semver validation: MAJOR.MINOR.PATCH(-PRERELEASE)?
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z\.-]+)?$ ]]; then
  echo "Error: version '$NEW_VERSION' is not a valid semantic version (expected MAJOR.MINOR.PATCH or with -prerelease)." >&2
  exit 1
fi

TAG="v$NEW_VERSION"

echo "Bumping workspace version to $NEW_VERSION (tag: $TAG)"

ROOT_CARGO_TOML="$ROOT_DIR/Cargo.toml"
if [[ ! -f "$ROOT_CARGO_TOML" ]]; then
  echo "Error: Cargo.toml not found at $ROOT_CARGO_TOML" >&2
  exit 1
fi

CURRENT_VERSION_LINE
CURRENT_VERSION_LINE="$(grep -E '^[[:space:]]*version[[:space:]]*=' "$ROOT_CARGO_TOML" || true)"
if [[ -z "$CURRENT_VERSION_LINE" ]]; then
  echo "Error: could not find workspace version in Cargo.toml" >&2
  exit 1
fi

echo "Current version line: $CURRENT_VERSION_LINE"

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo
  echo "[DRY RUN] Would update Cargo.toml workspace version to $NEW_VERSION"
else
  # Update the workspace.package version in Cargo.toml
  # This replaces the first occurrence of a 'version = "...\"' line.
  perl -0pi -e "s/(\\[workspace.package\\][^\\[]*?version\\s*=\\s*\")([^\"]+)(\"[^\[]*)/\${1}$NEW_VERSION\${3}/s" "$ROOT_CARGO_TOML"

  echo "Updated Cargo.toml workspace version to $NEW_VERSION"

  # Optionally update fly.toml if it exists and contains an app version field
  FLY_TOML="$ROOT_DIR/fly.toml"
  if [[ -f "$FLY_TOML" ]]; then
    if grep -qE '^\s*version\s*=' "$FLY_TOML"; then
      perl -pi -e "s/^(\\s*version\\s*=\\s*\")([^\"]+)(\")/\${1}$NEW_VERSION\${3}/" "$FLY_TOML"
      echo "Updated fly.toml version to $NEW_VERSION"
    fi
  fi
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo
  echo "[DRY RUN] Skipping git commit and tag."
  exit 0
fi

if [[ "$DO_COMMIT" -eq 1 ]]; then
  git add Cargo.toml fly.toml 2>/dev/null || git add Cargo.toml
  git commit -m "chore(release): bump version to $NEW_VERSION"
  echo "Created git commit for version $NEW_VERSION"
else
  echo "Skipping git commit (--no-commit). Remember to commit your changes."
fi

if [[ "$DO_TAG" -eq 1 ]]; then
  git tag -a "$TAG" -m "Release $TAG"
  echo "Created git tag $TAG"

  echo
  read -r -p "Push tag '$TAG' to origin now? [y/N] " REPLY
  if [[ "$REPLY" =~ ^[Yy]$ ]]; then
    git push origin "$TAG"
    echo "Pushed tag $TAG to origin"
  else
    echo "Skipping tag push. You can push later with: git push origin $TAG"
  fi
else
  echo "Skipping git tag creation (--no-tag)."
fi

echo "Done."

