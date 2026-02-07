//! MediaUploadConfig and UploadConfig implementations for AppState config types

use crate::state::{AudioConfig, DocumentConfig, ImageConfig, VideoConfig};

use super::traits::MediaUploadConfig;

impl MediaUploadConfig for ImageConfig {
    fn max_file_size(&self) -> usize {
        self.max_file_size
    }

    fn allowed_extensions(&self) -> &[String] {
        &self.allowed_extensions
    }

    fn allowed_content_types(&self) -> &[String] {
        &self.allowed_content_types
    }

    fn media_type_name(&self) -> &'static str {
        "image"
    }
}

#[cfg(feature = "audio")]
impl MediaUploadConfig for AudioConfig {
    fn max_file_size(&self) -> usize {
        self.max_file_size
    }

    fn allowed_extensions(&self) -> &[String] {
        &self.allowed_extensions
    }

    fn allowed_content_types(&self) -> &[String] {
        &self.allowed_content_types
    }

    fn media_type_name(&self) -> &'static str {
        "audio"
    }
}

#[cfg(feature = "video")]
impl MediaUploadConfig for VideoConfig {
    fn max_file_size(&self) -> usize {
        self.max_file_size
    }

    fn allowed_extensions(&self) -> &[String] {
        &self.allowed_extensions
    }

    fn allowed_content_types(&self) -> &[String] {
        &self.allowed_content_types
    }

    fn media_type_name(&self) -> &'static str {
        "video"
    }
}

#[cfg(feature = "document")]
impl MediaUploadConfig for DocumentConfig {
    fn max_file_size(&self) -> usize {
        self.max_file_size
    }

    fn allowed_extensions(&self) -> &[String] {
        &self.allowed_extensions
    }

    fn allowed_content_types(&self) -> &[String] {
        &self.allowed_content_types
    }

    fn media_type_name(&self) -> &'static str {
        "document"
    }
}
