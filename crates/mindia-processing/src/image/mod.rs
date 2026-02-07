//! Image processing module
//!
//! This module provides image processing capabilities including:
//! - Metadata extraction and validation (processor)
//! - Image transformations (transformer, resize, orientation)
//! - Advanced transforms (filters, smart_crop, watermark)

pub mod filters;
pub mod orientation;
pub mod processor;
pub mod resize;
pub mod smart_crop;
pub mod transformer;
pub mod watermark;

pub use processor::ImageProcessor;
pub use transformer::ImageTransformer;

// Re-export commonly used types
pub use filters::{FilterConfig, ImageFilters};
pub use orientation::ImageOrientation;
pub use resize::{ResizeDimensions, StretchMode};
pub use smart_crop::{SmartCrop, SmartCropConfig};
pub use watermark::{Watermark, WatermarkConfig, WatermarkPosition, WatermarkSize};

pub type ImageDimensions = crate::metadata::ImageMetadata;
