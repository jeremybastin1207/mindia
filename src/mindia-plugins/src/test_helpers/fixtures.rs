//! Test fixtures and helper functions for creating test data

use chrono::Utc;
use mindia_core::models::{Audio, Image, StorageLocation};
use mindia_core::storage_types::StorageBackend;
use serde_json::json;
use uuid::Uuid;

fn test_storage_location(key: &str, url: &str) -> StorageLocation {
    StorageLocation {
        id: Uuid::new_v4(),
        backend: StorageBackend::S3,
        bucket: Some("test-bucket".to_string()),
        key: key.to_string(),
        url: url.to_string(),
    }
}

/// Create a test Audio instance
pub fn create_test_audio(tenant_id: Uuid, media_id: Uuid, filename: Option<String>) -> Audio {
    let storage_key = filename
        .clone()
        .unwrap_or_else(|| format!("media/{}/test_audio.mp3", tenant_id));
    let url = format!("https://s3.amazonaws.com/test-bucket/{}", storage_key);

    Audio {
        id: media_id,
        tenant_id,
        filename: storage_key.clone(),
        original_filename: filename.unwrap_or_else(|| "test_audio.mp3".to_string()),
        storage: test_storage_location(&storage_key, &url),
        content_type: "audio/mpeg".to_string(),
        file_size: 1024 * 1024, // 1 MB
        duration: Some(120.0),  // 2 minutes
        bitrate: Some(128),
        sample_rate: Some(44100),
        channels: Some(2),
        uploaded_at: Utc::now(),
        updated_at: Utc::now(),
        store_behavior: "permanent".to_string(),
        store_permanently: true,
        expires_at: None,
    }
}

/// Create a test Image instance
pub fn create_test_image(tenant_id: Uuid, media_id: Uuid, filename: Option<String>) -> Image {
    let storage_key = filename
        .clone()
        .unwrap_or_else(|| format!("media/{}/test_image.jpg", tenant_id));
    let url = format!("https://s3.amazonaws.com/test-bucket/{}", storage_key);

    Image {
        id: media_id,
        tenant_id,
        filename: storage_key.clone(),
        original_filename: filename.unwrap_or_else(|| "test_image.jpg".to_string()),
        storage: test_storage_location(&storage_key, &url),
        content_type: "image/jpeg".to_string(),
        file_size: 512 * 1024, // 512 KB
        width: Some(1920),
        height: Some(1080),
        uploaded_at: Utc::now(),
        updated_at: Utc::now(),
        store_behavior: "permanent".to_string(),
        store_permanently: true,
        expires_at: None,
    }
}

/// Create a test tenant ID
pub fn create_test_tenant_id() -> Uuid {
    Uuid::new_v4()
}

/// Create a test media ID
pub fn create_test_media_id() -> Uuid {
    Uuid::new_v4()
}

/// Create AssemblyAI plugin config JSON
pub fn create_assembly_ai_config(api_key: Option<&str>) -> serde_json::Value {
    json!({
        "api_key": api_key.unwrap_or("test-api-key"),
        "language_code": "en"
    })
}

/// Create AWS Rekognition plugin config JSON
pub fn create_aws_rekognition_config(
    region: Option<&str>,
    min_confidence: Option<f32>,
) -> serde_json::Value {
    json!({
        "region": region.unwrap_or("us-east-1"),
        "min_confidence": min_confidence.unwrap_or(70.0),
        "detect_labels": true,
        "detect_text": true,
        "detect_custom_labels": false
    })
}

/// Create AWS Transcribe plugin config JSON
pub fn create_aws_transcribe_config(
    region: Option<&str>,
    s3_bucket: Option<&str>,
) -> serde_json::Value {
    json!({
        "region": region.unwrap_or("us-east-1"),
        "s3_bucket": s3_bucket.unwrap_or("test-transcribe-bucket"),
        "language_code": "en-US",
        "media_format": "mp3"
    })
}

/// Create Google Vision plugin config JSON
pub fn create_google_vision_config(
    api_key: Option<&str>,
    features: Option<Vec<String>>,
) -> serde_json::Value {
    json!({
        "api_key": api_key.unwrap_or("test-api-key"),
        "project_id": "test-project",
        "features": features.unwrap_or_else(|| vec![
            "LABEL_DETECTION".to_string(),
            "TEXT_DETECTION".to_string()
        ]),
        "min_score": 0.5
    })
}

/// Sample audio data for testing (MP3 header + dummy data)
pub fn create_test_audio_data() -> Vec<u8> {
    // Minimal MP3 header + dummy data
    let mut data = vec![0xFF, 0xFB, 0x90, 0x00]; // MP3 sync word
    data.extend(vec![0u8; 1000]); // Dummy audio data
    data
}

/// Sample image data for testing (minimal JPEG)
pub fn create_test_image_data() -> Vec<u8> {
    // Minimal JPEG header
    let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]; // JPEG SOI + APP0
    data.extend(vec![0u8; 1000]); // Dummy image data
    data
}
