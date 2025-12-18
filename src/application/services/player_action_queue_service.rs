//! Player Action Queue Service - Enqueues and processes player actions
//!
//! This service manages the PlayerActionQueue, which receives player actions
//! and routes them to the LLMReasoningQueue for processing.

use std::sync::Arc;

use crate::application::ports::outbound::{
    ProcessingQueuePort, QueueError, QueueItem, QueueItemId, QueuePort,
};
use crate::application::dto::{LLMRequestItem, LLMRequestType, PlayerActionItem};
use crate::domain::value_objects::GamePromptRequest;

/// Service for managing the player action queue
pub struct PlayerActionQueueService<Q: QueuePort<PlayerActionItem>, LQ: ProcessingQueuePort<LLMRequestItem>> {
    pub(crate) queue: Arc<Q>,
    llm_queue: Arc<LQ>,
}

impl<Q: QueuePort<PlayerActionItem>, LQ: ProcessingQueuePort<LLMRequestItem>>
    PlayerActionQueueService<Q, LQ>
{
    /// Create a new player action queue service
    pub fn new(queue: Arc<Q>, llm_queue: Arc<LQ>) -> Self {
        Self { queue, llm_queue }
    }

    /// Enqueue a player action for processing
    pub async fn enqueue_action(
        &self,
        session_id: crate::domain::value_objects::SessionId,
        player_id: String,
        pc_id: Option<crate::domain::value_objects::PlayerCharacterId>,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> Result<QueueItemId, QueueError> {
        let item = PlayerActionItem {
            session_id,
            player_id,
            pc_id,
            action_type,
            target,
            dialogue,
            timestamp: chrono::Utc::now(),
        };

        // Normal priority for player actions
        self.queue.enqueue(item, 0).await
    }

    /// Process the next player action from the queue
    ///
    /// This method:
    /// 1. Dequeues a player action
    /// 2. Builds the LLM request context
    /// 3. Enqueues it to the LLM queue
    /// 4. Marks the action as completed
    ///
    /// Returns the action item ID if processed, None if queue was empty
    pub async fn process_next<F, Fut>(
        &self,
        build_prompt: F,
    ) -> Result<Option<QueueItemId>, QueueError>
    where
        F: FnOnce(PlayerActionItem) -> Fut,
        Fut: std::future::Future<Output = Result<GamePromptRequest, QueueError>>,
    {
        let Some(item) = self.queue.dequeue().await? else {
            return Ok(None);
        };

        // Clone payload before passing to callback (item.payload is already Clone)
        let payload = item.payload.clone();
        let session_id = payload.session_id;
        let pc_id = payload.pc_id;
        let item_id = item.id;

        // Build the prompt request from the action (async)
        let prompt = build_prompt(payload).await?;

        // Create LLM request item
        let llm_request = LLMRequestItem {
            request_type: LLMRequestType::NPCResponse {
                action_item_id: item_id,
            },
            session_id: Some(session_id),
            world_id: None, // NPC responses use session_id for routing, not world_id
            pc_id,
            prompt: Some(prompt),
            suggestion_context: None,
            callback_id: item_id.to_string(),
        };

        // Enqueue to LLM queue (normal priority)
        self.llm_queue.enqueue(llm_request, 0).await?;

        // Mark action as completed (LLM queue handles the rest)
        self.queue.complete(item_id).await?;

        Ok(Some(item_id))
    }

    /// Get queue depth (number of pending actions)
    pub async fn depth(&self) -> Result<usize, QueueError> {
        self.queue.depth().await
    }

    /// Get a specific action item by ID
    pub async fn get_action(&self, id: QueueItemId) -> Result<Option<QueueItem<PlayerActionItem>>, QueueError> {
        self.queue.get(id).await
    }
}
