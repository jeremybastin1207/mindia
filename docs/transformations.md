# Transformations

Mindia supports transformations and processing for images, video, audio, and documents. This section describes what is available for each type.

## Overview

| Type | Transformation support | Details |
|------|------------------------|---------|
| **Image** | On-the-fly | Resize, format conversion, and quality adjustment via URL. No extra storage. |
| **Video** | Async transcoding | HLS adaptive bitrate with multiple quality variants. |
| **Audio** | Serve as stored | Files are delivered as uploaded. See [Audio](audio) for upload and delivery. |
| **Document** | Serve as stored | PDFs are delivered as uploaded. See [Documents](documents) for upload and delivery. |

## Image

[Image transformations](image-transformations) are fully on-the-fly: change the URL to get different sizes and formats (resize, WebP/JPEG/PNG, quality). No duplicate files are stored.

## Audio

[Audio transformations](audio-transformations) — current behavior and future options for audio delivery and conversion.

## Video

[Video transformations](video-transformations) — HLS transcoding, quality variants (360p–1080p), and streaming.

## Document

[Document transformations](document-transformations) — current behavior and future options for document delivery and conversion.
