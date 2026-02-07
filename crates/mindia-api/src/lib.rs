//! Mindia API Library
//!
//! This crate provides the HTTP API handlers, middleware, and application setup.

// Allow dead code for planned features and API documentation structures
#![allow(dead_code)]

// Module declarations
mod api_doc;
mod constants;
mod handlers;
mod http_metrics;
mod job_queue;
mod middleware;
mod services;
mod setup;
mod task_dispatch;
mod task_handlers;
mod telemetry;
mod utils;
mod validation;
mod video_storage_impl;

// Public modules
pub mod auth;
pub mod error;
#[cfg(feature = "semantic-search")]
#[cfg(feature = "plugin")]
pub mod plugins;
pub mod state;

// Re-exports
pub use error::ErrorResponse;
pub use mindia_worker::{TaskQueue, TaskQueueConfig};
pub use task_handlers::TaskHandler;

#[cfg(feature = "video")]
pub use mindia_processing::{VideoOrchestrator, VideoOrchestratorConfig, VideoStorage};
#[cfg(feature = "content-moderation")]
pub use task_handlers::ContentModerationTaskHandler;
#[cfg(feature = "semantic-search")]
pub use task_handlers::EmbeddingTaskHandler;
#[cfg(feature = "plugin")]
pub use task_handlers::PluginTaskHandler;
#[cfg(feature = "video")]
pub use task_handlers::VideoTaskHandler;
