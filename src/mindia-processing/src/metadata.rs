//! Unified media metadata types

use serde::{Deserialize, Serialize};

/// Unified metadata structure for all media types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaMetadata {
    Image(ImageMetadata),
    Video(VideoMetadata),
    Audio(AudioMetadata),
    Document(DocumentMetadata),
}

/// Image metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub size_bytes: Option<u64>,
    pub exif_orientation: Option<u8>,
    pub color_space: Option<String>,
}

/// Video metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub bitrate: Option<u64>,
    pub framerate: Option<f32>,
}

/// Audio metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub codec: Option<String>,
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub page_count: Option<u32>,
    pub format: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub size_bytes: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_metadata_serialization() {
        let metadata = ImageMetadata {
            width: 1920,
            height: 1080,
            format: "JPEG".to_string(),
            size_bytes: Some(1024000),
            exif_orientation: Some(6),
            color_space: Some("RGB".to_string()),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ImageMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.width, deserialized.width);
        assert_eq!(metadata.height, deserialized.height);
        assert_eq!(metadata.format, deserialized.format);
        assert_eq!(metadata.size_bytes, deserialized.size_bytes);
        assert_eq!(metadata.exif_orientation, deserialized.exif_orientation);
    }

    #[test]
    fn test_video_metadata_serialization() {
        let metadata = VideoMetadata {
            duration: 120.5,
            width: 1920,
            height: 1080,
            codec: "h264".to_string(),
            bitrate: Some(5000000),
            framerate: Some(30.0),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: VideoMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.duration, deserialized.duration);
        assert_eq!(metadata.width, deserialized.width);
        assert_eq!(metadata.codec, deserialized.codec);
    }

    #[test]
    fn test_audio_metadata_serialization() {
        let metadata = AudioMetadata {
            duration: Some(180.0),
            bitrate: Some(320),
            sample_rate: Some(44100),
            channels: Some(2),
            codec: Some("mp3".to_string()),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: AudioMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.duration, deserialized.duration);
        assert_eq!(metadata.bitrate, deserialized.bitrate);
        assert_eq!(metadata.sample_rate, deserialized.sample_rate);
    }

    #[test]
    fn test_document_metadata_serialization() {
        let metadata = DocumentMetadata {
            page_count: Some(10),
            format: "pdf".to_string(),
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
            size_bytes: Some(512000),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: DocumentMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.page_count, deserialized.page_count);
        assert_eq!(metadata.format, deserialized.format);
        assert_eq!(metadata.title, deserialized.title);
    }

    #[test]
    fn test_media_metadata_enum() {
        let image_meta = ImageMetadata {
            width: 100,
            height: 100,
            format: "PNG".to_string(),
            size_bytes: Some(1000),
            exif_orientation: None,
            color_space: None,
        };

        let media_meta = MediaMetadata::Image(image_meta.clone());

        match media_meta {
            MediaMetadata::Image(meta) => {
                assert_eq!(meta.width, image_meta.width);
                assert_eq!(meta.height, image_meta.height);
            }
            _ => panic!("Expected Image variant"),
        }

        let video_meta = VideoMetadata {
            duration: 60.0,
            width: 1920,
            height: 1080,
            codec: "h264".to_string(),
            bitrate: None,
            framerate: None,
        };

        let media_meta = MediaMetadata::Video(video_meta.clone());

        match media_meta {
            MediaMetadata::Video(meta) => {
                assert_eq!(meta.duration, video_meta.duration);
                assert_eq!(meta.width, video_meta.width);
            }
            _ => panic!("Expected Video variant"),
        }
    }

    #[test]
    fn test_metadata_with_optional_fields() {
        // Test metadata with all optional fields None
        let image_meta = ImageMetadata {
            width: 100,
            height: 100,
            format: "PNG".to_string(),
            size_bytes: None,
            exif_orientation: None,
            color_space: None,
        };

        let json = serde_json::to_string(&image_meta).unwrap();
        assert!(json.contains("width"));
        assert!(json.contains("height"));

        let deserialized: ImageMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.size_bytes, None);
        assert_eq!(deserialized.exif_orientation, None);
    }
}
