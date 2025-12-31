//! TaskHandlerContext implementation for AppState.
//!
//! Dispatches tasks to the appropriate handler based on task type.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use mindia_core::models::{Task, TaskType};
use mindia_worker::TaskHandlerContext;

use crate::state::AppState;
#[cfg(feature = "semantic-search")]
use crate::task_handlers::EmbeddingTaskHandler;
use crate::task_handlers::TaskHandler;
#[cfg(feature = "video")]
use crate::task_handlers::VideoTaskHandler;

#[async_trait]
impl TaskHandlerContext for AppState {
    #[allow(unreachable_patterns)]
    async fn dispatch_task(self: Arc<Self>, task: &Task) -> Result<serde_json::Value> {
        match task.task_type {
            #[cfg(feature = "video")]
            TaskType::VideoTranscode => {
                let handler = VideoTaskHandler;
                handler.process(task, self).await
            }
            #[cfg(feature = "semantic-search")]
            TaskType::GenerateEmbedding => {
                let handler = EmbeddingTaskHandler;
                handler.process(task, self).await
            }
            #[cfg(feature = "plugin")]
            TaskType::PluginExecution => {
                let handler = self.plugin_task_handler.clone();
                handler.process(task, self).await
            }
            #[cfg(feature = "content-moderation")]
            TaskType::ContentModeration => {
                let handler = self.content_moderation_handler.clone();
                handler.process(task, self).await
            }
            _ => Err(anyhow::anyhow!(
                "Task type {:?} not supported (required features not enabled)",
                task.task_type
            )),
        }
    }
}
