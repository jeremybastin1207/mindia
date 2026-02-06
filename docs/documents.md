# Documents

Complete guide to uploading, managing, and serving PDF documents with Mindia.

## Table of Contents

- [Overview](#overview)
- [Upload Document](#upload-document)
- [List Documents](#list-documents)
- [Get Document Metadata](#get-document-metadata)
- [Download Document](#download-document)
- [Delete Document](#delete-document)
- [Best Practices](#best-practices)

## Overview

Mindia provides PDF document management with semantic search indexing.

**Supported Formats**:
- PDF (`.pdf`) - Portable Document Format

**Features**:
- ✅ Automatic text extraction
- ✅ Semantic search indexing
- ✅ Storage options (temporary or permanent)
- ✅ S3 or local storage
- ✅ UUID-based naming

**Limits** (configurable):
- Max file size: 50MB (default)
- Format: PDF only (for now)

## Upload Document

Upload a PDF document to Mindia.

### Endpoint

```
POST /api/documents
```

### Headers

```
Authorization: Bearer <token>
Content-Type: multipart/form-data
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `store` | string | `auto` | Storage behavior: `0` (24h), `1` (permanent), `auto` |

### Request Body

Multipart form data with a single `file` field.

### Response

**Status**: `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "550e8400-e29b-41d4-a716-446655440000.pdf",
  "original_filename": "report-q4-2024.pdf",
  "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.pdf",
  "content_type": "application/pdf",
  "file_size": 2097152,
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

### Examples

```bash
TOKEN="your-token"

# Upload document
curl -X POST https://api.example.com/api/documents \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@report.pdf"

# Upload with permanent storage
curl -X POST "https://api.example.com/api/documents?store=1" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@contract.pdf"
```

```javascript
async function uploadDocument(file, permanent = true) {
  const token = localStorage.getItem('token');
  const formData = new FormData();
  formData.append('file', file);

  const storeParam = permanent ? '1' : '0';
  const response = await fetch(
    `https://api.example.com/api/documents?store=${storeParam}`,
    {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
      },
      body: formData,
    }
  );

  if (!response.ok) {
    throw new Error('Upload failed');
  }

  return await response.json();
}

// Usage
const fileInput = document.querySelector('input[type="file"]');
fileInput.addEventListener('change', async (e) => {
  const file = e.target.files[0];
  
  if (file.type !== 'application/pdf') {
    alert('Please select a PDF file');
    return;
  }

  const document = await uploadDocument(file);
  console.log('Uploaded:', document.original_filename);
});
```

## List Documents

Retrieve a paginated list of documents.

### Endpoint

```
GET /api/documents
```

### Headers

```
Authorization: Bearer <token>
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | `50` | Number of results (1-100) |
| `offset` | integer | `0` | Number to skip |

### Response

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "filename": "550e8400-e29b-41d4-a716-446655440000.pdf",
    "original_filename": "report-q4-2024.pdf",
    "url": "https://bucket.s3.amazonaws.com/uploads/...",
    "content_type": "application/pdf",
    "file_size": 2097152,
    "uploaded_at": "2024-01-01T00:00:00Z"
  }
]
```

### Examples

```javascript
async function fetchDocuments(page = 1, perPage = 50) {
  const token = localStorage.getItem('token');
  const offset = (page - 1) * perPage;

  const response = await fetch(
    `https://api.example.com/api/documents?limit=${perPage}&offset=${offset}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return await response.json();
}

// Display document library
async function loadDocumentLibrary() {
  const documents = await fetchDocuments();
  
  documents.forEach(doc => {
    const sizeMB = (doc.file_size / (1024 * 1024)).toFixed(1);
    console.log(`${doc.original_filename} - ${sizeMB} MB`);
  });
}
```

## Get Document Metadata

Retrieve metadata for a specific document.

### Endpoint

```
GET /api/documents/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "550e8400-e29b-41d4-a716-446655440000.pdf",
  "original_filename": "report-q4-2024.pdf",
  "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.pdf",
  "content_type": "application/pdf",
  "file_size": 2097152,
  "uploaded_at": "2024-01-01T00:00:00Z"
}
```

## Download Document

Download the PDF document.

### Endpoint

```
GET /api/documents/:id/file
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `200 OK`  
**Content-Type**: `application/pdf`  
**Body**: Raw PDF bytes

### Examples

```bash
# Download to file
curl https://api.example.com/api/documents/$DOCUMENT_ID/file \
  -H "Authorization: Bearer $TOKEN" \
  -o downloaded-document.pdf
```

```javascript
async function downloadDocument(documentId, filename) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/documents/${documentId}/file`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  const blob = await response.blob();
  const url = window.URL.createObjectURL(blob);
  
  const a = document.createElement('a');
  a.href = url;
  a.download = filename || `document-${documentId}.pdf`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  window.URL.revokeObjectURL(url);
}
```

## Delete Document

Delete a document and its S3 storage.

### Endpoint

```
DELETE /api/documents/:id
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `204 No Content`

### Example

```bash
curl -X DELETE https://api.example.com/api/documents/$DOCUMENT_ID \
  -H "Authorization: Bearer $TOKEN"
```

```javascript
async function deleteDocument(documentId) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/documents/${documentId}`,
    {
      method: 'DELETE',
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return response.ok;
}
```

## Best Practices

### 1. Validate PDF Files

```javascript
function validatePDFFile(file) {
  if (file.type !== 'application/pdf') {
    throw new Error('Only PDF files are supported');
  }

  const maxSize = 50 * 1024 * 1024; // 50MB
  if (file.size > maxSize) {
    throw new Error('File too large. Maximum size is 50MB.');
  }

  return true;
}
```

### 2. Display Document List

```javascript
function DocumentList({ documents }) {
  return (
    <div className="document-list">
      {documents.map(doc => (
        <div key={doc.id} className="document-item">
          <div className="doc-icon">📄</div>
          <div className="doc-info">
            <h3>{doc.original_filename}</h3>
            <p>{formatFileSize(doc.file_size)}</p>
            <p>{formatDate(doc.uploaded_at)}</p>
          </div>
          <div className="doc-actions">
            <button onClick={() => downloadDocument(doc.id, doc.original_filename)}>
              Download
            </button>
            <button onClick={() => viewDocument(doc.url)}>
              View
            </button>
            <button onClick={() => deleteDocument(doc.id)}>
              Delete
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}

function formatFileSize(bytes) {
  if (bytes < 1024 * 1024) {
    return (bytes / 1024).toFixed(0) + ' KB';
  }
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

function formatDate(dateString) {
  return new Date(dateString).toLocaleDateString();
}
```

### 3. PDF Viewer Integration

```javascript
// Using PDF.js
import * as pdfjsLib from 'pdfjs-dist';

async function renderPDF(url, canvasElement) {
  const loadingTask = pdfjsLib.getDocument(url);
  const pdf = await loadingTask.promise;
  
  // Render first page
  const page = await pdf.getPage(1);
  const viewport = page.getViewport({ scale: 1.5 });
  
  const context = canvasElement.getContext('2d');
  canvasElement.height = viewport.height;
  canvasElement.width = viewport.width;

  await page.render({
    canvasContext: context,
    viewport: viewport,
  }).promise;
}

// React component
function PDFViewer({ documentId }) {
  const canvasRef = useRef(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadAndRenderPDF();
  }, [documentId]);

  async function loadAndRenderPDF() {
    try {
      const token = localStorage.getItem('token');
      const url = `https://api.example.com/api/documents/${documentId}/file`;
      
      // Add authorization
      const response = await fetch(url, {
        headers: { 'Authorization': `Bearer ${token}` },
      });
      
      const blob = await response.blob();
      const objectUrl = URL.createObjectURL(blob);
      
      await renderPDF(objectUrl, canvasRef.current);
      setLoading(false);
    } catch (error) {
      console.error('Failed to load PDF:', error);
      setLoading(false);
    }
  }

  return (
    <div>
      {loading && <p>Loading PDF...</p>}
      <canvas ref={canvasRef} />
    </div>
  );
}
```

### 4. Search Documents

Documents are automatically indexed for semantic search:

```javascript
// Search documents by content
const results = await fetch(
  'https://api.example.com/api/search?q=quarterly+report&type=document',
  {
    headers: { 'Authorization': `Bearer ${token}` },
  }
).then(r => r.json());

console.log('Found documents:', results);
```

See [Semantic Search](semantic-search.md) for details.

### 5. Bulk Upload

```javascript
async function uploadMultipleDocuments(files) {
  const results = [];
  
  for (const file of files) {
    try {
      const result = await uploadDocument(file);
      results.push({ success: true, file: file.name, data: result });
    } catch (error) {
      results.push({ success: false, file: file.name, error: error.message });
    }
  }

  return results;
}

// Usage
const fileInput = document.querySelector('input[type="file"]');
fileInput.multiple = true;

fileInput.addEventListener('change', async (e) => {
  const files = Array.from(e.target.files);
  const results = await uploadMultipleDocuments(files);
  
  const succeeded = results.filter(r => r.success).length;
  const failed = results.filter(r => !r.success).length;
  
  console.log(`Uploaded ${succeeded} documents, ${failed} failed`);
});
```

## Next Steps

- [Images](images.md) - Image management
- [Videos](videos.md) - Video streaming
- [Audio](audio.md) - Audio file management
- [Semantic Search](semantic-search.md) - Search documents by content
- [API Reference](api-reference.md) - Complete endpoint reference

