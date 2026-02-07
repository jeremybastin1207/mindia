//! Mindia Media Processing Library
//!
//! This crate provides media processing services for images, videos, audio, and documents.

pub mod metadata;
pub mod pipeline;
pub mod traits;

pub mod compression;
pub mod image;
pub mod upload;
pub mod validator;

#[cfg(feature = "video")]
pub mod video;

#[cfg(feature = "audio")]
pub mod audio;

#[cfg(feature = "document")]
pub mod document;

// Re-export commonly used types
pub use compression::{OutputFormat, QualityPreset};
pub use image::{
    FilterConfig, ImageFilters, ImageOrientation, ImageProcessor, ImageTransformer,
    ResizeDimensions, SmartCrop, SmartCropConfig, StretchMode, Watermark, WatermarkConfig,
    WatermarkPosition, WatermarkSize,
};
pub use metadata::{AudioMetadata, DocumentMetadata, ImageMetadata, MediaMetadata, VideoMetadata};
pub use pipeline::ProcessingPipeline;
pub use traits::{MediaProcessor, MediaTransformer, TransformType};
pub use validator::MediaValidator;

#[cfg(feature = "video")]
pub use video::{
    FFmpegService, HLSVariant, VideoOrchestrator, VideoOrchestratorConfig, VideoProcessor,
    VideoStorage, VideoTransformer,
};

#[cfg(feature = "audio")]
pub use audio::{AudioProcessor, AudioService, AudioTransformer};

pub use upload::{
    upload_pipeline, UploadConfig, UploadData, UploadProcessor, ValidatedFile, VirusScanner,
};
#[cfg(feature = "image")]
pub use upload::{ImageUploadProcessor, UploadImageMetadata};
