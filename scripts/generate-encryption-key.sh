#!/usr/bin/env bash
# Generate a secure encryption key for ENCRYPTION_KEY environment variable
# This key is used to encrypt sensitive plugin configuration data (API keys, secrets, tokens)

set -e

echo "Generating secure 32-byte encryption key..."
key=$(openssl rand -base64 32)

echo ""
echo "Add this to your .env file:"
echo ""
echo "ENCRYPTION_KEY=$key"
echo ""
echo "⚠️  IMPORTANT: Keep this key secure and never commit it to version control!"
echo "⚠️  If you lose this key, encrypted plugin configs cannot be decrypted."
