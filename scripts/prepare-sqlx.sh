#!/bin/bash
# Script to prepare SQLx query cache for offline builds
# This allows cargo check/build to work without a database connection

set -e

echo "🔧 Preparing SQLx query cache..."

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "❌ Error: DATABASE_URL environment variable is not set"
    echo ""
    echo "Please set DATABASE_URL to your PostgreSQL connection string."
    echo "Example:"
    echo "  export DATABASE_URL='postgresql://user:password@localhost:5432/mindia'"
    echo ""
    echo "Or create a .env file with:"
    echo "  DATABASE_URL=postgresql://user:password@localhost:5432/mindia"
    echo ""
    echo "Then run:"
    echo "  source .env  # or: export \$(cat .env | xargs)"
    echo "  ./scripts/prepare-sqlx.sh"
    exit 1
fi

# Check if sqlx-cli is installed
if ! command -v sqlx &> /dev/null; then
    echo "📦 Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features rustls,postgres
fi

# Prepare the query cache
echo "📝 Generating SQLx query cache..."
cargo sqlx prepare --workspace

echo "✅ SQLx query cache prepared successfully!"
echo ""
echo "You can now build the project without a database connection:"
echo "  cargo check"
echo "  cargo build"

