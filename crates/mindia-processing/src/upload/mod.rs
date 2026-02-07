//! Upload pipeline: validate → scan → process → store.

pub mod traits;
pub mod types;

#[cfg(feature = "audio")]
pub mod audio_processor;
#[cfg(feature = "document")]
pub mod document_processor;
#[cfg(feature = "image")]
pub mod image_processor;
#[cfg(feature = "video")]
pub mod video_processor;

mod pipeline;

pub use pipeline::upload_pipeline;
pub use traits::{UploadConfig, UploadProcessor, VirusScanner};
pub use types::{UploadData, ValidatedFile};

#[cfg(feature = "audio")]
pub use audio_processor::{AudioUploadProcessor, UploadAudioMetadata};
#[cfg(feature = "document")]
pub use document_processor::{DocumentUploadProcessor, UploadDocumentMetadata};
#[cfg(feature = "image")]
pub use image_processor::{ImageUploadProcessor, UploadImageMetadata};
#[cfg(feature = "video")]
pub use video_processor::{UploadVideoMetadata, VideoUploadProcessor};
