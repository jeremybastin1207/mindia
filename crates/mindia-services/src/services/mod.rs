#[cfg(feature = "semantic-search")]
pub mod anthropic;
#[cfg(feature = "clamav")]
pub mod clamav;
#[cfg(feature = "storage-s3")]
pub mod s3;
#[cfg(feature = "semantic-search")]
pub mod semantic_search;

// Re-export from mindia-infra
#[cfg(feature = "archive")]
pub use crate::archive::{create_archive, ArchiveFormat};
#[cfg(feature = "semantic-search")]
pub use anthropic::DefaultSemanticSearchService;
#[cfg(feature = "clamav")]
pub use clamav::{ClamAVService, ScanResult};
#[cfg(feature = "video")]
pub use mindia_processing::FFmpegService;
#[cfg(feature = "audio")]
pub use mindia_processing::{AudioMetadata, AudioService};
pub use mindia_processing::{
    ImageProcessor, ImageTransformer, OutputFormat, QualityPreset, ResizeDimensions,
    SmartCropConfig, StretchMode, WatermarkConfig, WatermarkPosition, WatermarkSize,
};
pub use mindia_storage::{create_storage, Storage, StorageError, StorageResult};
#[cfg(feature = "storage-s3")]
pub use s3::S3Service;
#[cfg(feature = "semantic-search")]
pub use semantic_search::{normalize_embedding_dim, SemanticSearchProvider, EMBEDDING_DIM};
