# Folders

Mindia's folder system allows you to organize your media files hierarchically, making it easier to manage large collections of images, videos, audio files, and documents.

## Overview

Folders provide a tree-like structure for organizing media:
- **Root folders**: Top-level folders (no parent)
- **Nested folders**: Folders within other folders
- **Media organization**: Move files between folders to organize your content

## Features

- ✅ Hierarchical folder structure
- ✅ Move media files between folders
- ✅ Folder metadata (name, creation date, counts)
- ✅ Tree view for navigation
- ✅ Tenant isolation (folders are per-tenant)

## Getting Started

### 1. Create a Folder

Create a new folder:

```bash
POST /api/folders
Content-Type: application/json

{
  "name": "Vacation Photos",
  "parent_id": null  // null for root folder, or UUID for nested folder
}
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Vacation Photos",
  "parent_id": null,
  "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

**Create a nested folder:**
```bash
POST /api/folders
Content-Type: application/json

{
  "name": "2024",
  "parent_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### 2. List Folders

List all folders or filter by parent:

```bash
# List all folders
GET /api/folders

# List only root folders
GET /api/folders?parent_id=null

# List folders in a specific parent
GET /api/folders?parent_id=550e8400-e29b-41d4-a716-446655440000
```

**Response:**
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Vacation Photos",
    "parent_id": null,
    "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
  }
]
```

### 3. Get Folder Tree

Get the complete hierarchical folder structure:

```bash
GET /api/folders/tree
```

**Response:**
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Vacation Photos",
    "parent_id": null,
    "children": [
      {
        "id": "770e8400-e29b-41d4-a716-446655440003",
        "name": "2024",
        "parent_id": "550e8400-e29b-41d4-a716-446655440000",
        "children": []
      }
    ]
  }
]
```

### 4. Get Folder Details

Get a specific folder with media and subfolder counts:

```bash
GET /api/folders/{id}
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Vacation Photos",
  "parent_id": null,
  "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z",
  "media_count": 42,
  "subfolder_count": 3
}
```

### 5. Update Folder

Update folder name or move it to a different parent:

```bash
PUT /api/folders/{id}
Content-Type: application/json

{
  "name": "Updated Folder Name",  // Optional
  "parent_id": "770e8400-e29b-41d4-a716-446655440003"  // Optional
}
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Updated Folder Name",
  "parent_id": "770e8400-e29b-41d4-a716-446655440003",
  "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T01:00:00Z"
}
```

**Note:** Moving a folder cannot create cycles. If you try to move a folder into one of its descendants, the request will fail.

### 6. Delete Folder

Delete a folder (must be empty):

```bash
DELETE /api/folders/{id}
```

**Response:** `204 No Content`

**Note:** Folders can only be deleted if they contain no media files or subfolders. Move or delete contents first.

### 7. Move Media to Folder

Move a media file (image, video, audio, document) to a folder:

```bash
PUT /api/media/{media_id}/folder
Content-Type: application/json

{
  "folder_id": "550e8400-e29b-41d4-a716-446655440000"  // null to move to root
}
```

**Response:** `200 OK`

## Folder Constraints

1. **Unique Names**: Folder names must be unique within the same parent folder
2. **No Cycles**: A folder cannot be moved into one of its descendants
3. **Empty Deletion**: Folders must be empty (no media or subfolders) before deletion
4. **Name Length**: Folder names are limited to 255 characters
5. **Tenant Isolation**: Folders are isolated per tenant

## Use Cases

### Organizing Photos by Year

```javascript
// Create year folders
const year2024 = await createFolder('2024', null);
const year2023 = await createFolder('2023', null);

// Create month folders
const jan2024 = await createFolder('January', year2024.id);
const feb2024 = await createFolder('February', year2024.id);

// Move photos to appropriate folders
await moveMedia(photoId, jan2024.id);
```

### Project-Based Organization

```javascript
// Create project structure
const projects = await createFolder('Projects', null);
const projectA = await createFolder('Project A', projects.id);
const projectB = await createFolder('Project B', projects.id);

// Organize media by project
await moveMedia(videoId, projectA.id);
await moveMedia(documentId, projectA.id);
```

### Building a Folder Tree UI

```javascript
async function buildFolderTree() {
  const tree = await fetch('/api/folders/tree', {
    headers: { 'Authorization': `Bearer ${apiKey}` }
  }).then(r => r.json());
  
  function renderFolder(folder, level = 0) {
    const indent = '  '.repeat(level);
    console.log(`${indent}📁 ${folder.name}`);
    folder.children.forEach(child => renderFolder(child, level + 1));
  }
  
  tree.forEach(folder => renderFolder(folder));
}
```

## Best Practices

1. **Plan Your Structure**: Design your folder hierarchy before creating many folders
2. **Consistent Naming**: Use consistent naming conventions (e.g., "YYYY-MM-DD" for dates)
3. **Avoid Deep Nesting**: Keep folder depth reasonable (3-4 levels max) for better performance
4. **Regular Cleanup**: Periodically review and reorganize folders
5. **Use Root Folders**: Group related content at the root level, then nest as needed

## Error Handling

**409 Conflict**: Folder name already exists in parent
- Solution: Use a different name or update the existing folder

**400 Bad Request**: Invalid folder name or cycle detected
- Solution: Check name length (max 255 chars) and ensure no circular references

**404 Not Found**: Folder or parent folder not found
- Solution: Verify folder IDs are correct and belong to your tenant

**400 Bad Request**: Cannot delete non-empty folder
- Solution: Move or delete all media and subfolders first

## API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/folders` | Create folder |
| GET | `/api/folders` | List folders |
| GET | `/api/folders/tree` | Get folder tree |
| GET | `/api/folders/{id}` | Get folder details |
| PUT | `/api/folders/{id}` | Update folder |
| DELETE | `/api/folders/{id}` | Delete folder |
| PUT | `/api/media/{id}/folder` | Move media to folder |

## Related Documentation

- [Images](images.md) - Image management
- [Videos](videos.md) - Video management
- [Audio](audio.md) - Audio file management
- [Documents](documents.md) - Document management
- [API Reference](api-reference.md) - Complete API documentation

