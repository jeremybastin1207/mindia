//! Unified media upload service module
//!
//! This module provides a unified upload service that handles all media types
//! (image, audio, video, document) through a configurable pipeline, eliminating
//! code duplication across handlers.

#[cfg(feature = "clamav")]
pub mod clamav_scanner;
pub mod image_processor;
pub mod processor_adapter;
pub mod service;
pub mod traits;
pub mod types;

#[cfg(feature = "audio")]
pub mod audio_processor;

#[cfg(feature = "video")]
pub mod video_processor;

#[cfg(feature = "document")]
pub mod document_processor;

mod config_impls;

pub use image_processor::{ImageMetadata, ImageProcessorImpl};
pub use service::MediaUploadService;
pub use traits::{MediaProcessor, MediaUploadConfig};

#[cfg(feature = "document")]
pub use document_processor::DocumentProcessorImpl;
