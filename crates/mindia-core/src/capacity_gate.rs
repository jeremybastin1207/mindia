//! Capacity gate trait for task queue workers.
//!
//! Implementations check whether an instance has enough resources (CPU, memory, disk)
//! to accept a new task. Used by the task queue to avoid claiming tasks when under
//! resource pressure.

use async_trait::async_trait;

/// Gate that determines whether this instance can accept new tasks.
///
/// Used by the task queue before claiming tasks. If `can_accept_task` returns false,
/// the worker skips claiming for this poll cycle; the task stays pending for another
/// instance.
#[async_trait]
pub trait CapacityGate: Send + Sync {
    /// Returns true if this instance has enough resources to accept a new task.
    async fn can_accept_task(&self) -> bool;
}
