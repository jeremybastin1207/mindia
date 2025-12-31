//! Video processing module

#[cfg(feature = "video")]
pub mod orchestration;
pub mod processor;
pub mod service;
pub mod transformer;
pub mod video_storage;

#[cfg(feature = "video")]
pub use orchestration::{VideoOrchestrator, VideoOrchestratorConfig};
pub use processor::VideoProcessor;
pub use service::{FFmpegService, HLSVariant};
pub use transformer::{
    VideoTransformOptions, VideoTransformParams, VideoTransformType, VideoTransformer,
};
pub use video_storage::VideoStorage;
