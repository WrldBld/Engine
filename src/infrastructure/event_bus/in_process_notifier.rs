//! In-Process Event Notifier - Wake subscribers when new events arrive
//!
//! This notifier uses tokio::sync::Notify to provide instant wake-ups
//! for in-process subscribers, complementing the polling mechanism.

use std::sync::Arc;
use tokio::sync::Notify;

/// A simple notifier for in-process event subscribers
#[derive(Clone)]
pub struct InProcessEventNotifier {
    notify: Arc<Notify>,
}

impl InProcessEventNotifier {
    /// Create a new notifier
    pub fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
        }
    }

    /// Wake up all waiters
    pub fn notify(&self) {
        self.notify.notify_waiters();
    }

    /// Wait for a notification
    pub async fn wait(&self) {
        self.notify.notified().await;
    }
}

impl Default for InProcessEventNotifier {
    fn default() -> Self {
        Self::new()
    }
}

