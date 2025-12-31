//! Mindia Services Layer
//!
//! This crate provides business logic services and processing functionality.

pub mod services;

// Re-export commonly used types from services module
// Re-export infrastructure services from mindia-infra
#[cfg(feature = "storage-s3")]
pub use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
#[cfg(feature = "archive")]
pub use mindia_infra::{create_archive, ArchiveFormat};
pub use services::s3::S3Service;
// Re-export storage from mindia-storage
pub use mindia_storage::{
    create_storage, LocalStorage, S3Storage, Storage, StorageBackend, StorageError, StorageResult,
};
// Re-export media processing from mindia-processing
#[cfg(feature = "cleanup")]
pub use mindia_infra::CleanupService;
#[cfg(feature = "video")]
pub use mindia_processing::FFmpegService;
#[cfg(feature = "audio")]
pub use mindia_processing::{AudioMetadata, AudioService};
pub use mindia_processing::{
    FilterConfig, ImageProcessor, ImageTransformer, MediaValidator, OutputFormat, QualityPreset,
    ResizeDimensions, SmartCropConfig, StretchMode, WatermarkConfig, WatermarkPosition,
    WatermarkSize,
};
#[cfg(feature = "clamav")]
pub use services::clamav::{ClamAVService, ScanResult};
// ContentModerationService moved to mindia-plugins as aws_rekognition_moderation plugin
// VideoProcessor, VideoJobQueue, EmbeddingJobQueue moved to mindia-api because they depend on AppState
#[cfg(feature = "rate-limit")]
pub use mindia_infra::RateLimiter;
#[cfg(feature = "analytics")]
pub use mindia_infra::{start_storage_metrics_refresh, AnalyticsService};
#[cfg(feature = "semantic-search")]
pub use services::{anthropic::AnthropicService, semantic_search::SemanticSearchProvider};
// TaskQueue and TaskHandlers moved to mindia-api because they depend on AppState
#[cfg(feature = "capacity")]
pub use mindia_infra::CapacityChecker;
#[cfg(feature = "webhook")]
pub use mindia_infra::{
    WebhookRetryService, WebhookRetryServiceConfig, WebhookService, WebhookServiceConfig,
};
