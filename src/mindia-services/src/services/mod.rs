// Infrastructure services moved to mindia-infra:
// - webhook, webhook_retry, analytics, rate_limiter, cleanup, capacity, archive
// These are now only available via mindia-infra
pub mod s3;
// Storage moved to mindia-storage crate
// Media processing moved to mindia-processing:
// - image, transform, compression, media_validator, audio, ffmpeg
#[cfg(feature = "clamav")]
pub mod clamav;
// Note: content_moderation moved to mindia-plugins as aws_rekognition_moderation plugin
// Note: ffmpeg and audio modules moved to mindia-processing
#[cfg(feature = "semantic-search")]
pub mod anthropic;
#[cfg(feature = "semantic-search")]
pub mod semantic_search;

// Re-export from mindia-infra
#[cfg(feature = "archive")]
pub use mindia_infra::{create_archive, ArchiveFormat};
pub use mindia_storage::{create_storage, Storage, StorageError, StorageResult};
pub use s3::S3Service; // Re-export from mindia-storage
                       // Media processing re-exported from mindia-processing
#[cfg(feature = "clamav")]
pub use clamav::{ClamAVService, ScanResult};
pub use mindia_processing::{
    ImageProcessor, ImageTransformer, OutputFormat, QualityPreset, ResizeDimensions,
    SmartCropConfig, StretchMode, WatermarkConfig, WatermarkPosition, WatermarkSize,
};
// ContentModerationService moved to mindia-plugins as aws_rekognition_moderation plugin
// FFmpegService and AudioService re-exported from mindia-processing
#[cfg(feature = "video")]
pub use mindia_processing::FFmpegService;
#[cfg(feature = "audio")]
pub use mindia_processing::{AudioMetadata, AudioService};
// VideoProcessor, VideoJobQueue, EmbeddingJobQueue moved to mindia-api because they depend on AppState
#[cfg(feature = "semantic-search")]
pub use anthropic::AnthropicService;
#[cfg(feature = "semantic-search")]
pub use semantic_search::{SemanticSearchProvider, EMBEDDING_DIM};
// Infrastructure services (cleanup, analytics, rate-limit, webhook, capacity) are now only available via mindia-infra
// TaskQueue and TaskHandlers moved to mindia-api because they depend on AppState
