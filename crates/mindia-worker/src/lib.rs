//! Mindia Worker – background task queue and worker infrastructure.
//!
//! This crate provides the task queue (polling, retry, worker pool) and the
//! `TaskHandlerContext` trait. The API implements the trait for its application
//! state and dispatches to handlers; handlers remain in the API crate.

mod context;
mod queue;

pub use context::{empty_context_weak, TaskHandlerContext};
pub use queue::{TaskQueue, TaskQueueConfig};
