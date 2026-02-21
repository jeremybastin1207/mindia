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
#[cfg(feature = "rate-limit")]
pub use mindia_infra::RateLimiter;

#[cfg(feature = "webhook")]
pub mod webhook;
#[cfg(feature = "webhook")]
pub use webhook::{
    WebhookRetryService, WebhookRetryServiceConfig, WebhookService, WebhookServiceConfig,
};

#[cfg(feature = "analytics")]
pub mod analytics;
#[cfg(feature = "analytics")]
pub use analytics::{start_storage_metrics_refresh, AnalyticsService};

#[cfg(feature = "cleanup")]
pub mod cleanup;
#[cfg(feature = "cleanup")]
pub use cleanup::CleanupService;

#[cfg(feature = "archive")]
pub mod archive;
#[cfg(feature = "archive")]
pub use archive::{create_archive, ArchiveFormat};
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
#[cfg(feature = "semantic-search")]
pub use services::{
    anthropic::DefaultSemanticSearchService,
    semantic_search::{normalize_embedding_dim, SemanticSearchProvider},
};
#[cfg(feature = "video")]
pub use video_orchestration::{VideoOrchestrator, VideoOrchestratorConfig};
