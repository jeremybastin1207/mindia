#!/bin/bash
set -e

echo "Starting ClamAV daemon..."
# Start ClamAV daemon in background
clamd &
CLAMD_PID=$!

echo "Starting freshclam (virus definition updater) in background..."
# Start freshclam daemon in background
freshclam -d &
FRESHCLAM_PID=$!

# Wait for ClamAV to be ready
echo "Waiting for ClamAV to be ready..."
for i in {1..30}; do
    if clamdscan --ping > /dev/null 2>&1; then
        echo "ClamAV is ready!"
        break
    fi
    echo "Waiting for ClamAV... ($i/30)"
    sleep 2
done

# Check if ClamAV started successfully
if ! clamdscan --ping > /dev/null 2>&1; then
    echo "WARNING: ClamAV failed to start, but continuing anyway..."
fi

echo "Starting mindia-api application..."
# Start the main application with ClamAV environment variables
export CLAMAV_ENABLED=true
export CLAMAV_HOST=127.0.0.1
export CLAMAV_PORT=3310

# OpenTelemetry configuration with defaults
export OTEL_ENABLED=${OTEL_ENABLED:-true}
export OTEL_EXPORTER_OTLP_ENDPOINT=${OTEL_EXPORTER_OTLP_ENDPOINT:-http://otel-collector:4317}
export OTEL_SERVICE_NAME=${OTEL_SERVICE_NAME:-mindia-api}
export OTEL_SERVICE_VERSION=${OTEL_SERVICE_VERSION:-0.1.0}
export OTEL_EXPORTER_OTLP_PROTOCOL=${OTEL_EXPORTER_OTLP_PROTOCOL:-grpc}

# Function to handle shutdown
shutdown() {
    echo "Shutting down..."
    kill -TERM "$CLAMD_PID" 2>/dev/null || true
    kill -TERM "$FRESHCLAM_PID" 2>/dev/null || true
    exit 0
}

# Set up signal handlers
trap shutdown SIGTERM SIGINT

# Start the application and wait for it
exec /usr/local/bin/mindia-api

