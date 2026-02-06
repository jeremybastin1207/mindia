# File Groups

File groups allow you to associate multiple media files together and download them as a single archive. This is useful for batch downloads, organizing related content, or creating downloadable collections.

## Overview

File groups provide:
- **File Association**: Group multiple files (images, videos, audio, documents) together
- **Archive Downloads**: Download entire groups as ZIP or TAR archives
- **Sequential Access**: Access files by index within a group
- **Metadata**: Track group creation time and file counts

## Use Cases

- Download multiple related files at once
- Create downloadable content packages
- Organize files for batch processing
- Share collections of media files

## Getting Started

### 1. Create a File Group

Create a group and associate files with it:

```bash
POST /api/groups
Content-Type: application/json

{
  "files": [
    "550e8400-e29b-41d4-a716-446655440000",
    "660e8400-e29b-41d4-a716-446655440001",
    "770e8400-e29b-41d4-a716-446655440002"
  ]
}
```

**Response:**
```json
{
  "id": "880e8400-e29b-41d4-a716-446655440003",
  "created_at": "2024-01-01T00:00:00Z",
  "file_count": 3
}
```

**Note:** All file IDs must belong to your tenant and exist in the system.

### 2. Get File Group Details

Get information about a file group:

```bash
GET /api/groups/{id}
```

**Response:**
```json
{
  "id": "880e8400-e29b-41d4-a716-446655440003",
  "created_at": "2024-01-01T00:00:00Z",
  "files": [
    {
      "media_id": "550e8400-e29b-41d4-a716-446655440000",
      "index": 0,
      "filename": "photo1.jpg",
      "content_type": "image/jpeg",
      "file_size": 1048576,
      "url": "https://bucket.s3.amazonaws.com/uploads/550e8400-e29b-41d4-a716-446655440000.jpg"
    },
    {
      "media_id": "660e8400-e29b-41d4-a716-446655440001",
      "index": 1,
      "filename": "video1.mp4",
      "content_type": "video/mp4",
      "file_size": 52428800,
      "url": "https://bucket.s3.amazonaws.com/uploads/660e8400-e29b-41d4-a716-446655440001.mp4"
    },
    {
      "media_id": "770e8400-e29b-41d4-a716-446655440002",
      "index": 2,
      "filename": "document1.pdf",
      "content_type": "application/pdf",
      "file_size": 2097152,
      "url": "https://bucket.s3.amazonaws.com/uploads/770e8400-e29b-41d4-a716-446655440002.pdf"
    }
  ]
}
```

### 3. Get File Group Info (Summary)

Get a summary without full file details:

```bash
GET /api/groups/{id}/info
```

**Response:**
```json
{
  "id": "880e8400-e29b-41d4-a716-446655440003",
  "created_at": "2024-01-01T00:00:00Z",
  "file_count": 3
}
```

### 4. Access File by Index

Get a redirect to a specific file in the group by its index:

```bash
GET /api/groups/{id}/nth/{index}
```

**Example:**
```bash
GET /api/groups/880e8400-e29b-41d4-a716-446655440003/nth/0
```

**Response:** `302 Found` with `Location` header pointing to the file URL

**Note:** Index is 0-based. The first file is at index 0.

### 5. Download Group as Archive

Download the entire group as a ZIP or TAR archive:

```bash
# Download as ZIP
GET /api/groups/{id}/archive/zip

# Download as TAR
GET /api/groups/{id}/archive/tar
```

**Response:**
- Content-Type: `application/zip` or `application/x-tar`
- Content-Disposition: `attachment; filename="group.zip"` or `attachment; filename="group.tar"`
- Body: Binary archive file

**Example:**
```bash
curl -X GET \
  -H "Authorization: Bearer YOUR_API_KEY" \
  "https://api.mindia.com/api/groups/880e8400-e29b-41d4-a716-446655440003/archive/zip" \
  -o my-group.zip
```

### 6. Delete File Group

Delete a file group (does not delete the associated files):

```bash
DELETE /api/groups/{id}
```

**Response:** `204 No Content`

**Note:** Deleting a group only removes the association. The actual media files remain in storage.

## Archive Formats

### ZIP Format
- **Content-Type**: `application/zip`
- **Extension**: `.zip`
- **Compatibility**: Works on all platforms
- **Compression**: Yes (deflate)

### TAR Format
- **Content-Type**: `application/x-tar`
- **Extension**: `.tar`
- **Compatibility**: Unix/Linux systems
- **Compression**: No (uncompressed)

**Recommendation:** Use ZIP for maximum compatibility.

## File Ordering

Files in a group maintain the order they were added:
- Files are indexed starting from 0
- The order matches the order in the `files` array when creating the group
- Archive files preserve this order

## Example: Complete Workflow

```javascript
// 1. Upload multiple files
const file1 = await uploadImage('photo1.jpg');
const file2 = await uploadVideo('video1.mp4');
const file3 = await uploadDocument('doc1.pdf');

// 2. Create a file group
const group = await fetch('/api/groups', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${apiKey}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    files: [file1.id, file2.id, file3.id]
  })
}).then(r => r.json());

console.log(`Created group ${group.id} with ${group.file_count} files`);

// 3. Download as ZIP archive
const archiveResponse = await fetch(
  `/api/groups/${group.id}/archive/zip`,
  {
    headers: { 'Authorization': `Bearer ${apiKey}` }
  }
);

const blob = await archiveResponse.blob();
const url = window.URL.createObjectURL(blob);
const a = document.createElement('a');
a.href = url;
a.download = 'my-files.zip';
a.click();

// 4. Access individual file by index
const fileUrl = await fetch(
  `/api/groups/${group.id}/nth/0`,
  {
    headers: { 'Authorization': `Bearer ${apiKey}` },
    redirect: 'manual' // Don't follow redirect
  }
).then(r => r.headers.get('Location'));

console.log('First file URL:', fileUrl);
```

## Best Practices

1. **Group Size**: Keep groups reasonably sized (10-50 files) for better performance
2. **Archive Downloads**: Use ZIP format for maximum compatibility
3. **File Validation**: Ensure all file IDs exist before creating a group
4. **Cleanup**: Delete groups when no longer needed to keep your account organized
5. **Order Matters**: Consider file order when creating groups for logical archive organization

## Error Handling

**400 Bad Request**: Invalid file IDs or empty file list
- Solution: Verify all file IDs exist and belong to your tenant

**404 Not Found**: File group not found
- Solution: Check the group ID is correct

**400 Bad Request**: Cannot create archive for empty group
- Solution: Ensure the group contains at least one file

**404 Not Found**: File at index not found
- Solution: Check the index is within the group's file count (0 to file_count-1)

## Limitations

- **File Count**: No hard limit, but very large groups (>100 files) may take longer to process
- **Archive Size**: Large archives may take time to generate and download
- **File Types**: All media types (images, videos, audio, documents) can be grouped together
- **No Updates**: Groups are immutable once created. Delete and recreate to change files.

## API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/groups` | Create file group |
| GET | `/api/groups/{id}` | Get group with file details |
| GET | `/api/groups/{id}/info` | Get group summary |
| GET | `/api/groups/{id}/nth/{index}` | Redirect to file by index |
| GET | `/api/groups/{id}/archive/{format}` | Download archive (zip/tar) |
| DELETE | `/api/groups/{id}` | Delete file group |

## Related Documentation

- [Images](images.md) - Image upload and management
- [Videos](videos.md) - Video upload and management
- [Audio](audio.md) - Audio file management
- [Documents](documents.md) - Document management
- [API Reference](api-reference.md) - Complete API documentation

