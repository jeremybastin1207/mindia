//! Audio processing module

pub mod processor;
pub mod transformer;

pub use processor::{AudioProcessor, AudioService};
pub use transformer::{
    AudioTransformOptions, AudioTransformParams, AudioTransformType, AudioTransformer,
};

// Re-export metadata types
pub use crate::metadata::AudioMetadata;
