#!/usr/bin/env bash
# Deploy Mindia to Fly.io with a basic config.
#
# Prerequisites:
#   - flyctl: https://fly.io/docs/hands-on/install-flyctl/
#   - Logged in: fly auth login
#
# Basic config uses:
#   - Fly Postgres (created and attached if DATABASE_URL is not set)
#   - Local storage on a Fly volume (or S3 if S3_* env vars are set)
#   - Generated JWT_SECRET if not set
#
# Usage:
#   ./scripts/deploy-to-fly.sh              # interactive: create DB if needed, set secrets, deploy
#   DATABASE_URL=... JWT_SECRET=... ./scripts/deploy-to-fly.sh   # non-interactive
#   USE_S3=1 ./scripts/deploy-to-fly.sh     # use S3 (set S3_BUCKET, S3_REGION, AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FLY_TOML="${REPO_ROOT}/fly.toml"
APP_NAME="mindia"
REGION="ams"
VOLUME_NAME="mindia_data"
VOLUME_MOUNT_PATH="/data"
LOCAL_STORAGE_SUBPATH="storage"

# Read app name from fly.toml if present
if [[ -f "$FLY_TOML" ]]; then
  if grep -q "^app = " "$FLY_TOML" 2>/dev/null; then
    APP_NAME=$(sed -n 's/^app = *["'\'']\([^"'\'']*\)["'\''] *$/\1/p' "$FLY_TOML" | head -1)
  fi
fi

usage() {
  echo "Usage: $0 [OPTIONS]"
  echo ""
  echo "Deploy Mindia to Fly.io with basic config."
  echo ""
  echo "Options:"
  echo "  -n, --no-deploy    Set secrets only, do not run fly deploy"
  echo "  -h, --help         Show this help"
  echo ""
  echo "Environment (optional):"
  echo "  DATABASE_URL       Postgres URL. If unset, script can create Fly Postgres and attach."
  echo "  JWT_SECRET         Min 32 chars. If unset, one is generated."
  echo "  USE_S3             Set to 1 to use S3; requires S3_BUCKET, S3_REGION, AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY."
  echo "  FLY_APP_NAME       Override app name (default: from fly.toml or 'mindia')."
  exit 0
}

NO_DEPLOY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    -n|--no-deploy) NO_DEPLOY=true; shift ;;
    -h|--help) usage ;;
    *) echo "Unknown option: $1"; usage ;;
  esac
done

if [[ -n "${FLY_APP_NAME:-}" ]]; then
  APP_NAME="$FLY_APP_NAME"
fi

echo "==> Mindia Fly.io deploy (app: $APP_NAME, region: $REGION)"
echo ""

# --- Check flyctl ---
if ! command -v flyctl &>/dev/null && ! command -v fly &>/dev/null; then
  echo "flyctl is not installed. Install it: https://fly.io/docs/hands-on/install-flyctl/"
  exit 1
fi
FLY_CMD="flyctl"
if command -v fly &>/dev/null; then
  FLY_CMD="fly"
fi

if ! $FLY_CMD auth whoami &>/dev/null; then
  echo "Not logged in to Fly. Run: $FLY_CMD auth login"
  exit 1
fi

# --- Ensure app exists (optional; fly deploy can create from fly.toml) ---
if ! $FLY_CMD apps list 2>/dev/null | grep -q "$APP_NAME"; then
  echo "Creating app $APP_NAME..."
  $FLY_CMD apps create "$APP_NAME" 2>/dev/null || true
fi

# --- Database ---
if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is not set."
  read -r -p "Create and attach a Fly Postgres cluster? [y/N] " CREATE_DB
  if [[ "${CREATE_DB,,}" =~ ^y(es)?$ ]]; then
    PG_APP="${APP_NAME}-db"
    echo "Creating Postgres app: $PG_APP (region: $REGION)..."
    $FLY_CMD postgres create --name "$PG_APP" --region "$REGION" --initial-cluster-size 1
    echo "Attaching Postgres to $APP_NAME..."
    $FLY_CMD postgres attach "$PG_APP" --app "$APP_NAME"
    echo "DATABASE_URL is now set on the app via attach."
  else
    echo "Set DATABASE_URL and run again, or choose 'y' to create Fly Postgres."
    exit 1
  fi
else
  echo "Using existing DATABASE_URL (will set as secret)."
fi

# --- JWT_SECRET ---
if [[ -z "${JWT_SECRET:-}" ]]; then
  JWT_SECRET=$(openssl rand -hex 32)
  echo "Generated JWT_SECRET (saving to app secrets)."
else
  if [[ ${#JWT_SECRET} -lt 32 ]]; then
    echo "JWT_SECRET must be at least 32 characters."
    exit 1
  fi
  echo "Using provided JWT_SECRET."
fi

# --- Storage: S3 vs local (volume) ---
USE_S3="${USE_S3:-0}"
if [[ "$USE_S3" = "1" ]] && [[ -n "${S3_BUCKET:-}" ]] && [[ -n "${S3_REGION:-}" ]] && [[ -n "${AWS_ACCESS_KEY_ID:-}" ]] && [[ -n "${AWS_SECRET_ACCESS_KEY:-}" ]]; then
  STORAGE_BACKEND="s3"
  echo "Using S3 storage (bucket: $S3_BUCKET)."
else
  STORAGE_BACKEND="local"
  APP_URL="https://${APP_NAME}.fly.dev"
  LOCAL_STORAGE_PATH="${VOLUME_MOUNT_PATH}/${LOCAL_STORAGE_SUBPATH}"
  LOCAL_STORAGE_BASE_URL="${APP_URL}"
  echo "Using local storage (volume, path: $LOCAL_STORAGE_PATH)."
fi

# --- Add volume mount to fly.toml for local storage ---
if [[ "$STORAGE_BACKEND" = "local" ]] && [[ -f "$FLY_TOML" ]]; then
  if ! grep -q "\[\[mounts\]\]" "$FLY_TOML"; then
    echo "Adding volume mount to fly.toml..."
    cat >> "$FLY_TOML" << EOF

# Persistent volume for local storage (added by deploy-to-fly.sh)
[[mounts]]
  source = "$VOLUME_NAME"
  destination = "$VOLUME_MOUNT_PATH"
EOF
  fi
  echo "Ensuring volume $VOLUME_NAME exists in $REGION..."
  if ! $FLY_CMD volumes list -a "$APP_NAME" 2>/dev/null | grep -q "$VOLUME_NAME"; then
    $FLY_CMD volumes create "$VOLUME_NAME" --size 1 --region "$REGION" -a "$APP_NAME"
  else
    echo "Volume $VOLUME_NAME already exists."
  fi
fi

# --- Build secrets for fly (KEY=value pairs; values with special chars are safe in array elements) ---
declare -a SECRET_ARGS=(
  "JWT_SECRET=$JWT_SECRET"
  "STORAGE_BACKEND=$STORAGE_BACKEND"
  "ENVIRONMENT=production"
)

if [[ -n "${DATABASE_URL:-}" ]]; then
  SECRET_ARGS+=("DATABASE_URL=$DATABASE_URL")
fi

if [[ "$STORAGE_BACKEND" = "local" ]]; then
  SECRET_ARGS+=("LOCAL_STORAGE_PATH=$LOCAL_STORAGE_PATH")
  SECRET_ARGS+=("LOCAL_STORAGE_BASE_URL=$LOCAL_STORAGE_BASE_URL")
fi

if [[ "$STORAGE_BACKEND" = "s3" ]]; then
  SECRET_ARGS+=("S3_BUCKET=$S3_BUCKET")
  SECRET_ARGS+=("S3_REGION=$S3_REGION")
  SECRET_ARGS+=("AWS_ACCESS_KEY_ID=$AWS_ACCESS_KEY_ID")
  SECRET_ARGS+=("AWS_SECRET_ACCESS_KEY=$AWS_SECRET_ACCESS_KEY")
fi

# --- Set secrets ---
echo ""
echo "Setting secrets on $APP_NAME..."
for s in "${SECRET_ARGS[@]}"; do
  key="${s%%=*}"
  echo "  $key=***"
done
$FLY_CMD secrets set -a "$APP_NAME" "${SECRET_ARGS[@]}"

echo ""
echo "Secrets updated."

if [[ "$NO_DEPLOY" = true ]]; then
  echo "Skipping deploy (--no-deploy). Run: $FLY_CMD deploy -a $APP_NAME"
  exit 0
fi

echo ""
echo "Deploying (this may take several minutes)..."
cd "$REPO_ROOT"
$FLY_CMD deploy -a "$APP_NAME"

echo ""
echo "Done. App URL: https://${APP_NAME}.fly.dev"
echo "Health: https://${APP_NAME}.fly.dev/health"
