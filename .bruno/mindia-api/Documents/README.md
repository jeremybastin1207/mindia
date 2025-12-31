# Documents

API requests for managing PDF documents.

## Available Requests

- **Upload Document** - Upload a PDF document file
- **List Documents** - Get paginated list of all documents
- **Get Document by ID** - Retrieve metadata for a specific document
- **Download Document** - Download the original PDF file
- **Delete Document** - Remove document from storage and database

## Usage

1. Upload a document using **Upload Document**
2. The document ID is automatically saved to `{{documentId}}` variable
3. Use other requests to manage the document

## Features

- PDF file upload and storage
- Automatic text extraction
- Semantic search indexing (if enabled)
- Storage behavior control (temporary or permanent)
- Folder organization support

