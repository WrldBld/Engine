//! In-process queue notifier using tokio::sync::Notify
//!
//! This implementation is for single-process deployments where
//! workers run in the same process as enqueuers.

use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use tokio::sync::Notify;

use crate::application::ports::outbound::{QueueNotificationPort, WaitResult};

/// In-process notifier using tokio::sync::Notify
/// 
/// Suitable for:
/// - SQLite backend (single process)
/// - InMemory backend (testing)
/// 
/// NOT suitable for:
/// - Multi-process deployments
/// - Redis backend (use RedisPubSubNotifier instead)
#[derive(Clone)]
pub struct InProcessNotifier {
    notify: Arc<Notify>,
    queue_name: String,
}

impl InProcessNotifier {
    /// Create a new in-process notifier for a queue
    pub fn new(queue_name: impl Into<String>) -> Self {
        Self {
            notify: Arc::new(Notify::new()),
            queue_name: queue_name.into(),
        }
    }
}

#[async_trait]
impl QueueNotificationPort for InProcessNotifier {
    async fn notify_work_available(&self) {
        self.notify.notify_one();
    }

    async fn wait_for_work(&self, timeout: Duration) -> WaitResult {
        match tokio::time::timeout(timeout, self.notify.notified()).await {
            Ok(()) => WaitResult::Notified,
            Err(_) => WaitResult::Timeout,
        }
    }

    fn queue_name(&self) -> &str {
        &self.queue_name
    }
}

