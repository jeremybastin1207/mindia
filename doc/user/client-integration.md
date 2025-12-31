# Client Integration

Examples and patterns for integrating Mindia into your applications.

## JavaScript/TypeScript SDK

### Basic API Client

```typescript
class MindiaClient {
  constructor(private baseUrl: string, private token: string) {}

  private async request(endpoint: string, options: RequestInit = {}) {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      headers: {
        'Authorization': `Bearer ${this.token}`,
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    return response.json();
  }

  // Images
  async uploadImage(file: File) {
    const formData = new FormData();
    formData.append('file', file);

    const response = await fetch(`${this.baseUrl}/api/images`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${this.token}` },
      body: formData,
    });

    return response.json();
  }

  async listImages(limit = 50, offset = 0) {
    return this.request(`/api/images?limit=${limit}&offset=${offset}`);
  }

  async getImage(id: string) {
    return this.request(`/api/images/${id}`);
  }

  async deleteImage(id: string) {
    return this.request(`/api/images/${id}`, { method: 'DELETE' });
  }

  // Videos
  async uploadVideo(file: File) {
    const formData = new FormData();
    formData.append('file', file);

    const response = await fetch(`${this.baseUrl}/api/videos`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${this.token}` },
      body: formData,
    });

    return response.json();
  }

  async getVideo(id: string) {
    return this.request(`/api/videos/${id}`);
  }

  // Search
  async search(query: string, type?: string, limit = 20) {
    const params = new URLSearchParams({ q: query, limit: limit.toString() });
    if (type) params.append('type', type);
    
    return this.request(`/api/search?${params}`);
  }
}

// Usage
const client = new MindiaClient('https://api.example.com', 'your-token');

const image = await client.uploadImage(file);
const images = await client.listImages();
const results = await client.search('sunset beach');
```

## React Integration

### Context Provider

```tsx
import React, { createContext, useContext, useState, useEffect } from 'react';

interface MindiaContextType {
  client: MindiaClient | null;
  isAuthenticated: boolean;
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
}

const MindiaContext = createContext<MindiaContextType | null>(null);

export function MindiaProvider({ children, baseUrl }: { children: React.ReactNode; baseUrl: string }) {
  const [client, setClient] = useState<MindiaClient | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  useEffect(() => {
    // Use API key from env or config (never expose master key in browser; use a tenant API key)
    const apiKey = process.env.MINDIA_API_KEY || '';
    if (apiKey) {
      setClient(new MindiaClient(baseUrl, apiKey));
      setIsAuthenticated(true);
    }
  }, [baseUrl]);

  function setApiKey(apiKey: string) {
    setClient(new MindiaClient(baseUrl, apiKey));
    setIsAuthenticated(true);
  }

  function logout() {
    setClient(null);
    setIsAuthenticated(false);
  }

  return (
    <MindiaContext.Provider value={{ client, isAuthenticated, setApiKey, logout }}>
      {children}
    </MindiaContext.Provider>
  );
}

export function useMindia() {
  const context = useContext(MindiaContext);
  if (!context) throw new Error('useMindia must be used within MindiaProvider');
  return context;
}
```

### Hooks

```tsx
// Upload Hook
function useImageUpload() {
  const { client } = useMindia();
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function upload(file: File) {
    setUploading(true);
    setError(null);

    try {
      const result = await client.uploadImage(file);
      return result;
    } catch (err) {
      setError(err.message);
      throw err;
    } finally {
      setUploading(false);
    }
  }

  return { upload, uploading, error };
}

// Images List Hook
function useImages() {
  const { client } = useMindia();
  const [images, setImages] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    client.listImages().then(setImages).finally(() => setLoading(false));
  }, [client]);

  return { images, loading, refresh: () => client.listImages().then(setImages) };
}
```

## Vue Integration

```vue
<!-- Composition API -->
<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { MindiaClient } from './mindia-client';

const client = new MindiaClient('https://api.example.com', localStorage.getItem('token'));
const images = ref([]);
const loading = ref(true);

onMounted(async () => {
  images.value = await client.listImages();
  loading.value = false;
});

async function uploadImage(event) {
  const file = event.target.files[0];
  const result = await client.uploadImage(file);
  images.value.unshift(result);
}
</script>

<template>
  <div>
    <input type="file" @change="uploadImage" />
    
    <div v-if="loading">Loading...</div>
    
    <div v-else class="image-grid">
      <img v-for="image in images" :key="image.id" :src="image.url" />
    </div>
  </div>
</template>
```

## Next.js Integration

```tsx
// pages/api/upload.ts
import type { NextApiRequest, NextApiResponse } from 'next';
import formidable from 'formidable';

export const config = {
  api: { bodyParser: false },
};

export default async function handler(req: NextApiRequest, res: NextApiResponse) {
  if (req.method !== 'POST') {
    return res.status(405).json({ error: 'Method not allowed' });
  }

  const form = formidable();
  const [fields, files] = await form.parse(req);

  const file = files.file[0];
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch('https://api.example.com/api/images', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${process.env.MINDIA_TOKEN}`,
    },
    body: formData,
  });

  const data = await response.json();
  res.status(200).json(data);
}
```

## Mobile Integration

### React Native

```tsx
import { useState } from 'react';
import { launchImageLibrary } from 'react-native-image-picker';

function useImagePicker() {
  const [uploading, setUploading] = useState(false);

  async function pickAndUpload() {
    const result = await launchImageLibrary({ mediaType: 'photo' });
    
    if (result.assets && result.assets[0]) {
      setUploading(true);

      const formData = new FormData();
      formData.append('file', {
        uri: result.assets[0].uri,
        type: result.assets[0].type,
        name: result.assets[0].fileName,
      });

      const response = await fetch('https://api.example.com/api/images', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
        },
        body: formData,
      });

      const data = await response.json();
      setUploading(false);

      return data;
    }
  }

  return { pickAndUpload, uploading };
}
```

## Python Integration

```python
import requests
from typing import Optional

class MindiaClient:
    def __init__(self, base_url: str, token: str):
        self.base_url = base_url
        self.token = token
        self.headers = {
            'Authorization': f'Bearer {token}'
        }

    def upload_image(self, file_path: str):
        with open(file_path, 'rb') as f:
            files = {'file': f}
            response = requests.post(
                f'{self.base_url}/api/images',
                headers=self.headers,
                files=files
            )
            response.raise_for_status()
            return response.json()

    def list_images(self, limit: int = 50, offset: int = 0):
        response = requests.get(
            f'{self.base_url}/api/images',
            headers=self.headers,
            params={'limit': limit, 'offset': offset}
        )
        response.raise_for_status()
        return response.json()

    def search(self, query: str, media_type: Optional[str] = None, limit: int = 20):
        params = {'q': query, 'limit': limit}
        if media_type:
            params['type'] = media_type

        response = requests.get(
            f'{self.base_url}/api/search',
            headers=self.headers,
            params=params
        )
        response.raise_for_status()
        return response.json()

# Usage
client = MindiaClient('https://api.example.com', 'your-token')

# Upload
image = client.upload_image('photo.jpg')

# Search
results = client.search('sunset beach', media_type='image')
```

## Best Practices

### 1. Token Management

```typescript
class TokenManager {
  private token: string | null = null;
  private expiresAt: number | null = null;

  setToken(token: string, expiresIn: number) {
    this.token = token;
    this.expiresAt = Date.now() + (expiresIn * 1000);
    localStorage.setItem('mindia_token', token);
    localStorage.setItem('mindia_expires', this.expiresAt.toString());
  }

  getToken(): string | null {
    if (!this.token) {
      this.token = localStorage.getItem('mindia_token');
      this.expiresAt = parseInt(localStorage.getItem('mindia_expires') || '0');
    }

    if (this.isExpired()) {
      this.clearToken();
      return null;
    }

    return this.token;
  }

  isExpired(): boolean {
    return this.expiresAt ? Date.now() >= this.expiresAt : true;
  }

  clearToken() {
    this.token = null;
    this.expiresAt = null;
    localStorage.removeItem('mindia_token');
    localStorage.removeItem('mindia_expires');
  }
}
```

### 2. File Validation

```typescript
function validateImageFile(file: File): void {
  const maxSize = 10 * 1024 * 1024; // 10MB
  const allowedTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp'];

  if (!allowedTypes.includes(file.type)) {
    throw new Error(`Invalid file type: ${file.type}`);
  }

  if (file.size > maxSize) {
    throw new Error(`File too large: ${(file.size / 1024 / 1024).toFixed(1)}MB`);
  }
}
```

### 3. Upload Progress

```typescript
async function uploadWithProgress(file: File, onProgress: (percent: number) => void) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();

    xhr.upload.addEventListener('progress', (e) => {
      if (e.lengthComputable) {
        onProgress((e.loaded / e.total) * 100);
      }
    });

    xhr.addEventListener('load', () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        resolve(JSON.parse(xhr.responseText));
      } else {
        reject(new Error(`Upload failed: ${xhr.status}`));
      }
    });

    xhr.addEventListener('error', () => reject(new Error('Upload failed')));

    const formData = new FormData();
    formData.append('file', file);

    xhr.open('POST', 'https://api.example.com/api/images');
    xhr.setRequestHeader('Authorization', `Bearer ${token}`);
    xhr.send(formData);
  });
}
```

## Next Steps

- [Error Handling](error-handling.md) - Handle API errors
- [Best Practices](best-practices.md) - Production tips
- [API Reference](api-reference.md) - Complete API docs

