#[cfg(feature = "content-moderation")]
mod content_moderation_handler;
#[cfg(feature = "semantic-search")]
mod embedding_handler;
#[cfg(feature = "plugin")]
mod plugin_handler;
#[cfg(feature = "video")]
mod video_handler;

#[cfg(feature = "content-moderation")]
pub use content_moderation_handler::ContentModerationTaskHandler;
#[cfg(feature = "semantic-search")]
pub use embedding_handler::EmbeddingTaskHandler;
#[cfg(feature = "plugin")]
pub use plugin_handler::PluginTaskHandler;
#[cfg(feature = "video")]
pub use video_handler::VideoTaskHandler;

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::state::AppState;
use mindia_core::models::Task;

/// Trait for task handlers.
///
/// **CPU-bound work:** If a handler does CPU-intensive work (e.g. image processing,
/// document parsing, heavy computation), run that work inside
/// `tokio::task::spawn_blocking` so it does not block the async runtime:
///
/// ```ignore
/// let result = tokio::task::spawn_blocking(|| {
///     cpu_intensive_operation(data)
/// }).await??;
/// ```
#[async_trait]
pub trait TaskHandler {
    async fn process(&self, task: &Task, state: Arc<AppState>) -> Result<serde_json::Value>;
}
