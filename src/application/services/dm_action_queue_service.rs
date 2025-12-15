//! DM Action Queue Service - Enqueues and processes DM actions
//!
//! This service manages the DMActionQueue, which receives DM actions
//! (approval decisions, direct NPC control, event triggers, scene transitions)
//! and processes them immediately.

use std::sync::Arc;

use crate::application::ports::outbound::{QueueError, QueueItem, QueueItemId, QueuePort};
use crate::application::dto::{DMAction, DMActionItem};

/// Service for managing the DM action queue
pub struct DMActionQueueService<Q: QueuePort<DMActionItem>> {
    pub(crate) queue: Arc<Q>,
}

impl<Q: QueuePort<DMActionItem>> DMActionQueueService<Q> {
    /// Create a new DM action queue service
    pub fn new(queue: Arc<Q>) -> Self {
        Self { queue }
    }

    /// Enqueue a DM action for processing
    ///
    /// DM actions have high priority (1) to ensure they are processed
    /// before player actions.
    pub async fn enqueue_action(
        &self,
        session_id: crate::domain::value_objects::SessionId,
        dm_id: String,
        action: DMAction,
    ) -> Result<QueueItemId, QueueError> {
        let item = DMActionItem {
            session_id,
            dm_id,
            action,
            timestamp: chrono::Utc::now(),
        };

        // High priority for DM actions
        self.queue.enqueue(item, 1).await
    }

    /// Process the next DM action from the queue
    ///
    /// Returns the action item ID if processed, None if queue was empty
    pub async fn process_next<F, Fut>(
        &self,
        process_action: F,
    ) -> Result<Option<QueueItemId>, QueueError>
    where
        F: FnOnce(DMActionItem) -> Fut,
        Fut: std::future::Future<Output = Result<(), QueueError>>,
    {
        let Some(item) = self.queue.dequeue().await? else {
            return Ok(None);
        };

        // Clone payload before passing to callback (item.payload is already Clone)
        match process_action(item.payload.clone()).await {
            Ok(()) => {
                self.queue.complete(item.id).await?;
                Ok(Some(item.id))
            }
            Err(e) => {
                // Mark as failed
                self.queue.fail(item.id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    /// Get queue depth (number of pending actions)
    pub async fn depth(&self) -> Result<usize, QueueError> {
        self.queue.depth().await
    }

    /// Get a specific action item by ID
    pub async fn get_action(&self, id: QueueItemId) -> Result<Option<QueueItem<DMActionItem>>, QueueError> {
        self.queue.get(id).await
    }
}
