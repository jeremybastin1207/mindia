//! Data models for the application
//!
//! This module contains all data structures used throughout the application,
//! organized by domain. Each sub-module represents a specific feature area.

mod analytics;
mod audio;
mod billing;
mod document;
mod file_group;
mod folder;
mod image;
mod media;
mod named_transformation;
mod organization;
mod plan;
pub mod plugin;
pub mod presigned_upload;
mod search;
mod storage;
mod subscription;
mod task;
mod tenant;
mod usage;
mod user;
mod video;
mod webhook;
mod workflow;

// Re-export all models for convenient imports
pub use analytics::*;
pub use audio::*;
pub use billing::*;
pub use document::*;
pub use file_group::*;
pub use folder::*;
pub use image::*;
pub use organization::*;
// Media module exports - exclude types that conflict with individual modules
pub use media::{
    to_audio, to_document, to_image, to_video, Media, MediaItem, MediaRow, MediaType,
    ProcessingStatus, TypeMetadata,
};
pub use named_transformation::*;
pub use plan::*;
pub use plugin::*;
pub use presigned_upload::*;
pub use search::*;
pub use storage::*;
pub use subscription::*;
pub use task::*;
pub use tenant::*;
pub use usage::*;
pub use user::*;
pub use video::*;
pub use webhook::*;
pub use workflow::*;
