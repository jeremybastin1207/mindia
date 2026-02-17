//! Mindia Services Layer
//!
//! This crate is the **business service layer**: it hosts orchestration and
//! domain services (e.g. video transcoding orchestration, ClamAV, semantic search)
//! and re-exports a unified API from infrastructure, processing, and storage
//! so that the API crate depends on a single service facade. Keep business
//! logic and coordination here; keep thin HTTP handling in mindia-api.

pub mod services;

#[cfg(feature = "video")]
pub mod video_orchestration;

#[cfg(feature = "capacity")]
pub use mindia_infra::CapacityChecker;
#[cfg(feature = "cleanup")]
pub use mindia_infra::CleanupService;
#[cfg(feature = "rate-limit")]
pub use mindia_infra::RateLimiter;
#[cfg(feature = "archive")]
pub use mindia_infra::{create_archive, ArchiveFormat};
#[cfg(feature = "analytics")]
pub use mindia_infra::{start_storage_metrics_refresh, AnalyticsService};
#[cfg(feature = "webhook")]
pub use mindia_infra::{
    WebhookRetryService, WebhookRetryServiceConfig, WebhookService, WebhookServiceConfig,
};
#[cfg(feature = "video")]
pub use mindia_processing::FFmpegService;
#[cfg(feature = "audio")]
pub use mindia_processing::{AudioMetadata, AudioService};
pub use mindia_processing::{
    FilterConfig, ImageProcessor, ImageTransformer, MediaValidator, OutputFormat, QualityPreset,
    ResizeDimensions, SmartCropConfig, StretchMode, WatermarkConfig, WatermarkPosition,
    WatermarkSize,
};
pub use mindia_storage::{
    create_storage, LocalStorage, S3Storage, Storage, StorageBackend, StorageError, StorageResult,
};
#[cfg(feature = "clamav")]
pub use services::clamav::{ClamAVService, ScanResult};
#[cfg(feature = "storage-s3")]
pub use services::s3::S3Service;
#[cfg(feature = "semantic-search")]
pub use services::{
    anthropic::AnthropicService,
    semantic_search::{normalize_embedding_dim, SemanticSearchProvider},
};
#[cfg(feature = "video")]
pub use video_orchestration::{VideoOrchestrator, VideoOrchestratorConfig};
