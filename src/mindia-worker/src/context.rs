//! Task handler context trait
//!
//! The API implements this trait for its application state. The worker calls
//! `dispatch_task` when processing a task; the implementation matches on task
//! type and invokes the appropriate handler.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::{Arc, Weak};

use mindia_core::models::Task;

/// Context for task dispatch.
///
/// Implemented by the API's application state. The worker holds a weak
/// reference and calls `dispatch_task` when processing a claimed task.
#[async_trait]
pub trait TaskHandlerContext: Send + Sync {
    /// Dispatch a task to the appropriate handler and return the result.
    async fn dispatch_task(self: Arc<Self>, task: &Task) -> Result<serde_json::Value>;
}

/// Placeholder context used when no real context exists yet (e.g. during init).
/// Dispatch always errors.
struct NoopContext;

#[async_trait]
impl TaskHandlerContext for NoopContext {
    async fn dispatch_task(self: Arc<Self>, _task: &Task) -> Result<serde_json::Value> {
        Err(anyhow!("NoopContext: no handler context available"))
    }
}

/// Returns a weak reference to a no-op context. Use as placeholder when building
/// TaskQueue before the real AppState context exists.
pub fn empty_context_weak() -> Weak<dyn TaskHandlerContext> {
    let n: Arc<dyn TaskHandlerContext> = Arc::new(NoopContext);
    Arc::downgrade(&n)
}
