#!/bin/bash
set -euo pipefail

# Clear media and tasks from database (for development/testing)
# Usage: ./scripts/clear-media-tasks.sh [options]
#
# Options:
#   --tasks-only     Clear only tasks
#   --media-only     Clear only media
#   --storage        Also delete files from storage directory
#   --yes            Skip confirmation prompt
#   -h, --help       Show this help message

CLEAR_TASKS=true
CLEAR_MEDIA=true
CLEAR_STORAGE=false
SKIP_CONFIRM=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --tasks-only)
            CLEAR_MEDIA=false
            shift
            ;;
        --media-only)
            CLEAR_TASKS=false
            shift
            ;;
        --storage)
            CLEAR_STORAGE=true
            shift
            ;;
        --yes)
            SKIP_CONFIRM=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Clear media and tasks from database (for development/testing)"
            echo ""
            echo "Options:"
            echo "  --tasks-only     Clear only tasks"
            echo "  --media-only     Clear only media"
            echo "  --storage        Also delete files from storage directory"
            echo "  --yes            Skip confirmation prompt"
            echo "  -h, --help       Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Check if DATABASE_URL is set
if [ -z "${DATABASE_URL:-}" ]; then
    echo "❌ Error: DATABASE_URL environment variable is not set"
    exit 1
fi

# Show what will be cleared
echo "=================================="
echo "Clear Media and Tasks Script"
echo "=================================="
echo ""
if [ "$CLEAR_TASKS" = true ]; then
    echo "✓ Will clear tasks"
fi
if [ "$CLEAR_MEDIA" = true ]; then
    echo "✓ Will clear media (images, videos, audio, documents)"
fi
if [ "$CLEAR_STORAGE" = true ]; then
    echo "✓ Will delete files from storage/media/"
fi
echo ""

# Confirmation prompt
if [ "$SKIP_CONFIRM" = false ]; then
    echo "⚠️  WARNING: This will permanently delete data!"
    echo "   Database: $DATABASE_URL"
    echo ""
    read -p "Are you sure you want to continue? (yes/no): " -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]es$ ]]; then
        echo "Aborted."
        exit 0
    fi
fi

# Build SQL commands
SQL_COMMANDS=""

if [ "$CLEAR_TASKS" = true ]; then
    SQL_COMMANDS+="
-- Clear tasks
DELETE FROM webhook_delivery_retries;
DELETE FROM webhook_deliveries;
DELETE FROM tasks;
"
fi

if [ "$CLEAR_MEDIA" = true ]; then
    SQL_COMMANDS+="
-- Clear media-related tables
DELETE FROM plugin_executions;
DELETE FROM plugin_cost_summaries;
DELETE FROM embeddings;
DELETE FROM file_group_items;
DELETE FROM file_groups;
DELETE FROM upload_chunks;
DELETE FROM media;
"
fi

# Execute SQL
echo "Executing database cleanup..."
if psql "$DATABASE_URL" -c "$SQL_COMMANDS"; then
    echo "✓ Database cleanup completed"
else
    echo "❌ Database cleanup failed"
    exit 1
fi

# Clear storage files if requested
if [ "$CLEAR_STORAGE" = true ]; then
    echo ""
    echo "Clearing storage files..."
    STORAGE_DIR="./storage/media"
    
    if [ -d "$STORAGE_DIR" ]; then
        FILE_COUNT=$(find "$STORAGE_DIR" -type f | wc -l)
        if [ "$FILE_COUNT" -gt 0 ]; then
            rm -rf "${STORAGE_DIR:?}"/*
            echo "✓ Deleted $FILE_COUNT file(s) from $STORAGE_DIR"
        else
            echo "  No files to delete in $STORAGE_DIR"
        fi
    else
        echo "  Storage directory $STORAGE_DIR does not exist"
    fi
fi

echo ""
echo "=================================="
echo "Cleanup completed successfully!"
echo "=================================="
