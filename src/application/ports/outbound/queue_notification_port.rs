//! Queue notification port - Abstract interface for queue event notifications
//!
//! This port allows different notification strategies:
//! - InProcessNotifier: Uses tokio::sync::Notify (for SQLite/InMemory)
//! - RedisPubSubNotifier: Uses Redis SUBSCRIBE/PUBLISH (future)

use async_trait::async_trait;
use std::time::Duration;

/// Result of waiting for work notification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    /// Worker was notified of new work
    Notified,
    /// Timeout expired (periodic recovery check)
    Timeout,
}

/// Port for queue work notifications
/// 
/// Implementations:
/// - `InProcessNotifier`: For single-process deployments (SQLite, InMemory)
/// - `RedisPubSubNotifier`: For distributed deployments (future)
#[async_trait]
pub trait QueueNotificationPort: Send + Sync + Clone {
    /// Signal that new work is available on this queue
    /// 
    /// For InProcess: Wakes one waiting worker via tokio::sync::Notify
    /// For Redis: Publishes to queue-specific channel
    async fn notify_work_available(&self);

    /// Wait for new work notification or timeout
    /// 
    /// # Arguments
    /// * `timeout` - Maximum time to wait. Used for periodic recovery polls.
    /// 
    /// # Returns
    /// * `WaitResult::Notified` - Work is available
    /// * `WaitResult::Timeout` - Timeout expired, worker should check queue anyway
    async fn wait_for_work(&self, timeout: Duration) -> WaitResult;

    /// Get the queue name this notifier is for (for debugging/logging)
    fn queue_name(&self) -> &str;
}

