//! Core traits for media processing
//!
//! This module defines the unified interface for all media processors and transformers.

use async_trait::async_trait;
use bytes::Bytes;

/// Transform type enumeration for all media types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformType {
    // Image transforms
    ImageResize,
    ImageCrop,
    ImageRotate,
    ImageFlip,
    ImageWatermark,
    ImageFilter,
    ImageFormatConvert,

    // Video transforms
    VideoTranscode,
    VideoScale,
    VideoTrim,
    VideoThumbnail,
    VideoWatermark,
    VideoExtractAudio,

    // Audio transforms
    AudioTranscode,
    AudioTrim,
    AudioNormalize,
    AudioBitrate,
    AudioWaveform,

    // Document transforms
    DocumentThumbnail,
    DocumentTextExtract,
    DocumentCompress,
}

/// Media processor trait - handles metadata extraction and validation
#[async_trait]
pub trait MediaProcessor: Send + Sync {
    type Metadata: Send + Sync;

    /// Extract metadata from media data
    async fn extract_metadata(&self, data: &[u8]) -> Result<Self::Metadata, anyhow::Error>;

    /// Validate media data (format, magic bytes, etc.)
    fn validate(&self, data: &[u8]) -> Result<(), anyhow::Error>;

    /// Get media dimensions if applicable (width, height)
    /// Returns None for non-visual media types
    fn get_dimensions(&self, data: &[u8]) -> Option<(u32, u32)>;
}

/// Media transformer trait - handles transformations
#[async_trait]
pub trait MediaTransformer: Send + Sync {
    type Options: Send + Sync;

    /// Apply transformation to media data
    async fn transform(&self, data: &[u8], options: Self::Options) -> Result<Bytes, anyhow::Error>;

    /// List supported transform types
    fn supported_transforms(&self) -> Vec<TransformType>;
}
