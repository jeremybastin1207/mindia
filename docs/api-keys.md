# API Key Guide

This guide explains how to create and use API keys for programmatic access to the Mindia Media Management API.

---

## 🔑 Overview

API keys provide a way to authenticate your applications without requiring user login. They are ideal for:

- **Backend Services:** Server-to-server communication
- **CI/CD Pipelines:** Automated media uploads in deployment workflows
- **Long-Running Scripts:** Batch processing and automation
- **Third-Party Integrations:** Secure access for external applications

### Key Features

- ✅ **Secure:** Keys are hashed using Argon2 before storage
- ✅ **Revocable:** Can be deactivated at any time
- ✅ **Expirable:** Optional expiration dates (up to 10 years)
- ✅ **Trackable:** Last used timestamp for monitoring
- ✅ **Tenant-Scoped:** Each key is associated with a specific tenant

---

## 🚀 Quick Start

### 1. Create an API Key

Use the **master API key** (or an existing API key) to create a new API key:

```bash
# Set your master API key (from MASTER_API_KEY env or .env)
export MASTER_KEY="your-master-api-key-at-least-32-characters"

# Create an API key
curl -X POST http://localhost:3000/api/v0/api-keys \
  -H "Authorization: Bearer $MASTER_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production API Key",
    "description": "Used for automated media uploads",
    "expires_in_days": 365
  }'
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "api_key": "mk_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234yz567",
  "name": "Production API Key",
  "description": "Used for automated media uploads",
  "key_prefix": "mk_live_abc123de",
  "expires_at": "2027-01-01T12:00:00Z",
  "created_at": "2026-01-01T12:00:00Z"
}
```

⚠️ **IMPORTANT:** Save the `api_key` value securely. It will **only be shown once** and cannot be retrieved later.

### 2. Use the API Key

Use the API key instead of a JWT token for authentication:

```bash
# Save your API key
export API_KEY="mk_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234yz567"

# Upload an image
curl -X POST http://localhost:3000/api/images \
  -H "Authorization: Bearer $API_KEY" \
  -F "file=@image.jpg"

# List images
curl -X GET http://localhost:3000/api/images \
  -H "Authorization: Bearer $API_KEY"
```

---

## 📚 API Endpoints

### Create API Key

**Endpoint:** `POST /api/v0/api-keys`  
**Authentication:** Master API key or API key required  

**Request Body:**
```json
{
  "name": "string (required)",
  "description": "string (optional)",
  "expires_in_days": "integer (optional, 1-3650)"
}
```

**Response:** `201 Created`
```json
{
  "id": "uuid",
  "api_key": "string (shown only once)",
  "name": "string",
  "description": "string",
  "key_prefix": "string",
  "expires_at": "datetime (optional)",
  "created_at": "datetime"
}
```

---

### List API Keys

**Endpoint:** `GET /api/v0/api-keys`  
**Authentication:** Master API key or API key required  

**Query Parameters:**
- `limit` (optional, default: 50, max: 100) - Number of results
- `offset` (optional, default: 0) - Pagination offset

**Response:** `200 OK`
```json
[
  {
    "id": "uuid",
    "name": "string",
    "description": "string",
    "key_prefix": "string",
    "last_used_at": "datetime (optional)",
    "expires_at": "datetime (optional)",
    "is_active": "boolean",
    "created_at": "datetime"
  }
]
```

---

### Get API Key Details

**Endpoint:** `GET /api/v0/api-keys/{id}`  
**Authentication:** Master API key or API key required  

**Response:** `200 OK`
```json
{
  "id": "uuid",
  "name": "string",
  "description": "string",
  "key_prefix": "string",
  "last_used_at": "datetime (optional)",
  "expires_at": "datetime (optional)",
  "is_active": "boolean",
  "created_at": "datetime"
}
```

---

### Revoke API Key

**Endpoint:** `DELETE /api/v0/api-keys/{id}`  
**Authentication:** Master API key or API key required  

**Response:** `200 OK`
```json
{
  "message": "API key revoked successfully",
  "id": "uuid"
}
```

⚠️ **Note:** Revoked keys cannot be reactivated. You must create a new key.

---

## 🔐 Security Best Practices

### Storage

✅ **DO:**
- Store API keys in environment variables
- Use secret management systems (AWS Secrets Manager, HashiCorp Vault, etc.)
- Encrypt keys at rest in your database
- Use different keys for different environments (dev, staging, prod)

❌ **DON'T:**
- Hardcode keys in source code
- Commit keys to version control
- Share keys via email or chat
- Use the same key across multiple services

### Key Management

✅ **DO:**
- Set expiration dates on API keys
- Rotate keys regularly (every 90-365 days)
- Revoke unused keys
- Monitor the `last_used_at` field for inactive keys
- Use descriptive names and descriptions

❌ **DON'T:**
- Create keys without expiration for production use
- Keep old keys active after rotation
- Use generic names like "test" or "key1"

### Access Control

✅ **DO:**
- Create separate keys for each application/service
- Use the principle of least privilege
- Monitor API key usage in logs
- Set up alerts for suspicious activity

❌ **DON'T:**
- Share keys between multiple services
- Grant more permissions than necessary
- Ignore unusual usage patterns

---

## 🛠️ Implementation Examples

### Python Example

```python
import os
import requests

# Load API key from environment
API_KEY = os.environ.get('MINDIA_API_KEY')
BASE_URL = 'https://api.example.com'

def upload_image(file_path):
    """Upload an image using API key authentication"""
    headers = {
        'Authorization': f'Bearer {API_KEY}'
    }
    
    with open(file_path, 'rb') as f:
        files = {'file': f}
        response = requests.post(
            f'{BASE_URL}/api/images',
            headers=headers,
            files=files
        )
    
    response.raise_for_status()
    return response.json()

# Usage
result = upload_image('photo.jpg')
print(f"Image uploaded: {result['id']}")
```

### Node.js Example

```javascript
const axios = require('axios');
const FormData = require('form-data');
const fs = require('fs');

const API_KEY = process.env.MINDIA_API_KEY;
const BASE_URL = 'https://api.example.com';

async function uploadImage(filePath) {
  const form = new FormData();
  form.append('file', fs.createReadStream(filePath));

  const response = await axios.post(
    `${BASE_URL}/api/images`,
    form,
    {
      headers: {
        ...form.getHeaders(),
        'Authorization': `Bearer ${API_KEY}`
      }
    }
  );

  return response.data;
}

// Usage
uploadImage('photo.jpg')
  .then(result => console.log(`Image uploaded: ${result.id}`))
  .catch(error => console.error('Upload failed:', error));
```

### Bash/cURL Example

```bash
#!/bin/bash

# Load API key from environment
API_KEY="${MINDIA_API_KEY}"
BASE_URL="https://api.example.com"

# Upload image
upload_image() {
    local file_path="$1"
    
    curl -X POST "${BASE_URL}/api/images" \
        -H "Authorization: Bearer ${API_KEY}" \
        -F "file=@${file_path}" \
        -s | jq -r '.id'
}

# Usage
IMAGE_ID=$(upload_image "photo.jpg")
echo "Image uploaded: ${IMAGE_ID}"
```

---

## 🔄 Key Rotation Workflow

### Step 1: Create New Key

```bash
curl -X POST http://localhost:3000/api/v0/api-keys \
  -H "Authorization: Bearer $MASTER_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production API Key v2",
    "description": "Rotation on 2026-01-01",
    "expires_in_days": 365
  }'
```

### Step 2: Update Applications

Update your applications to use the new key:

```bash
# Update environment variable
export MINDIA_API_KEY="new_key_here"

# Or update in your deployment system
kubectl set env deployment/myapp MINDIA_API_KEY=new_key_here
```

### Step 3: Verify New Key Works

```bash
curl -X GET http://localhost:3000/api/images \
  -H "Authorization: Bearer $NEW_API_KEY"
```

### Step 4: Revoke Old Key

```bash
curl -X DELETE http://localhost:3000/api/v0/api-keys/{old_key_id} \
  -H "Authorization: Bearer $MASTER_KEY"
```

---

## 🐛 Troubleshooting

### Error: "Invalid API key"

**Cause:** API key is incorrect or doesn't exist  
**Solution:** 
- Double-check the key value
- Ensure you copied the entire key (including `mk_live_` prefix)
- Create a new key if the original was lost

### Error: "API key has been revoked"

**Cause:** The API key was deactivated  
**Solution:** Create a new API key

### Error: "API key has expired"

**Cause:** The key's expiration date has passed  
**Solution:** Create a new API key with a new expiration date

### Error: "Missing authorization header"

**Cause:** No `Authorization` header in request  
**Solution:** Add header: `Authorization: Bearer YOUR_API_KEY`

---

## 📊 Monitoring

### Check Key Usage

```bash
# List all keys with last used timestamps
curl -X GET http://localhost:3000/api/v0/api-keys \
  -H "Authorization: Bearer $MASTER_KEY" \
  | jq '.[] | {name, last_used_at, is_active}'
```

### Identify Unused Keys

Keys with `last_used_at` as `null` or very old dates should be reviewed and potentially revoked.

---

## ⚡ Performance Considerations

- **Caching:** API key validation results can be cached for a short period (1-5 minutes)
- **Rate Limiting:** API keys are subject to the same rate limits as JWT tokens
- **Last Used Update:** The `last_used_at` timestamp is updated asynchronously to avoid performance impact

---

## 🔒 Technical Details

### Key Format

- **Prefix:** `mk_live_` (8 characters)
- **Random Part:** 40 hex characters
- **Total Length:** 48 characters
- **Example:** `mk_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234yz567`

### Storage

- Keys are hashed using **Argon2** before storage
- Only the hash is stored in the database
- The original key is only shown once at creation

### Authentication Flow

1. Extract token from `Authorization: Bearer <token>` header
2. If token matches master API key → use default tenant context
3. Else if token starts with `mk_live_` (generated API key):
   - Extract key prefix for efficient lookup
   - Find matching keys in database
   - Verify key against stored hash
   - Check if key is active and not expired
   - Load tenant information
   - Update `last_used_at` timestamp (async)
4. Else → return 401 Unauthorized
5. Create tenant context and proceed with request

---

## 📝 FAQ

### Can I use an API key to create other API keys?

Yes. You can use the master API key or an existing API key to create additional API keys.

### How many API keys can I create?

There's no hard limit, but we recommend keeping the number reasonable (e.g. one per application or environment).

### Can I regenerate a lost API key?

No. If you lose an API key, you must revoke it and create a new one. This is by design for security reasons.

### Do API keys expire automatically?

Yes, if you set an `expires_in_days` value when creating the key. Otherwise, they remain valid indefinitely (until revoked).

### Can I see which API key was used for a request?

The API key ID is logged in the application logs when authentication occurs. Check your logs for `api_key_id` field.

### What happens if I revoke an API key?

The key is deactivated immediately. Requests using that key will receive 401 Unauthorized.

---

## 🔗 Related Documentation

- [Authentication Testing Guide](./AUTHENTICATION_TESTING_GUIDE.md)
- [API Documentation](/docs)
- [Security Best Practices](./doc/SECURITY.md)

---

**Last Updated:** January 1, 2026  
**Version:** 1.0.0

