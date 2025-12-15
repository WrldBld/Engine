//! SQLite Event Bus - Persistent event bus backed by SQLite
//!
//! Publishes events to SQLite storage and triggers in-process notifications.

use async_trait::async_trait;
use std::sync::Arc;

use crate::application::dto::AppEvent;
use crate::application::ports::outbound::{
    AppEventRepositoryPort, EventBusError, EventBusPort,
};

use super::in_process_notifier::InProcessEventNotifier;

/// SQLite-backed event bus implementation
pub struct SqliteEventBus {
    repository: Arc<dyn AppEventRepositoryPort>,
    notifier: InProcessEventNotifier,
}

impl SqliteEventBus {
    /// Create a new SQLite event bus
    pub fn new(
        repository: Arc<dyn AppEventRepositoryPort>,
        notifier: InProcessEventNotifier,
    ) -> Self {
        Self {
            repository,
            notifier,
        }
    }
}

#[async_trait]
impl EventBusPort<AppEvent> for SqliteEventBus {
    async fn publish(&self, event: AppEvent) -> Result<(), EventBusError> {
        // Insert into storage
        self.repository
            .insert(&event)
            .await
            .map_err(|e| EventBusError::Transport(e.to_string()))?;

        // Best-effort notification (don't fail the whole operation if this fails)
        self.notifier.notify();

        Ok(())
    }
}

