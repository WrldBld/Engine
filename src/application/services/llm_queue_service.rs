//! LLM Queue Service - Concurrency-controlled LLM processing
//!
//! This service manages the LLMReasoningQueue, which processes LLM requests
//! with controlled concurrency using semaphores. It routes responses to the
//! DMApprovalQueue for NPC responses.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use crate::application::ports::outbound::{
    ApprovalQueuePort, LlmPort, ProcessingQueuePort, QueueError, QueueItemId, QueueNotificationPort,
};
use crate::application::services::llm_service::LLMService;
use crate::application::dto::{
    ApprovalItem, ChallengeSuggestionInfo, DecisionType, DecisionUrgency, LLMRequestItem,
    LLMRequestType, NarrativeEventSuggestionInfo,
};
use crate::domain::value_objects::ProposedToolInfo;

/// Priority constants for queue operations
const PRIORITY_NORMAL: u8 = 0;
const PRIORITY_HIGH: u8 = 1;

/// Service for managing the LLM reasoning queue
pub struct LLMQueueService<Q: ProcessingQueuePort<LLMRequestItem>, L: LlmPort + Clone, N: QueueNotificationPort> {
    pub(crate) queue: Arc<Q>,
    llm_service: Arc<LLMService<L>>,
    approval_queue: Arc<dyn ApprovalQueuePort<ApprovalItem>>,
    semaphore: Arc<Semaphore>,
    notifier: N,
}

impl<Q: ProcessingQueuePort<LLMRequestItem> + 'static, L: LlmPort + Clone + 'static, N: QueueNotificationPort + 'static> LLMQueueService<Q, L, N> {
    /// Create a new LLM queue service
    ///
    /// # Arguments
    ///
    /// * `queue` - The LLM request queue
    /// * `llm_client` - The LLM client for processing requests
    /// * `approval_queue` - The approval queue for routing NPC responses
    /// * `batch_size` - Maximum concurrent LLM requests (default: 1)
    /// * `notifier` - The notifier for waking workers
    pub fn new(
        queue: Arc<Q>,
        llm_client: Arc<L>,
        approval_queue: Arc<dyn ApprovalQueuePort<ApprovalItem>>,
        batch_size: usize,
        notifier: N,
    ) -> Self {
        Self {
            queue,
            llm_service: Arc::new(LLMService::new((*llm_client).clone())),
            approval_queue,
            semaphore: Arc::new(Semaphore::new(batch_size.max(1))),
            notifier,
        }
    }

    /// Enqueue an LLM request
    pub async fn enqueue(&self, request: LLMRequestItem) -> Result<QueueItemId, QueueError> {
        self.queue.enqueue(request, PRIORITY_NORMAL).await
    }

    /// Background worker that processes LLM requests
    ///
    /// This method runs in a loop, processing items from the queue with
    /// concurrency control via semaphore. Each request is processed in
    /// a spawned task to allow parallel processing up to batch_size.
    ///
    /// # Arguments
    /// * `recovery_interval` - Fallback poll interval for crash recovery
    pub async fn run_worker(self: Arc<Self>, recovery_interval: Duration) {
        loop {
            // Try to get next item
            let item = match self.queue.dequeue().await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    // Queue empty - wait for notification or recovery timeout
                    let _ = self.notifier.wait_for_work(recovery_interval).await;
                    continue;
                }
                Err(e) => {
                    tracing::error!("Failed to dequeue LLM request: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Process in spawned task - acquire permit inside the task for proper lifetime
            // Clone all needed data before spawning to avoid lifetime issues
            let semaphore = self.semaphore.clone();
            let llm_service_clone = self.llm_service.clone();
            let queue_clone = self.queue.clone();
            let approval_queue_clone = self.approval_queue.clone();
            let request = item.payload.clone();
            let item_id = item.id;

            tokio::spawn(async move {
                // Wait for capacity inside the spawned task
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Semaphore error: {}", e);
                        return;
                    }
                };

                match &request.request_type {
                    LLMRequestType::NPCResponse { action_item_id } => {
                        // Process NPC response request
                        match llm_service_clone.generate_npc_response(request.prompt.clone()).await {
                            Ok(response) => {
                                // Create approval item for DM
                                let session_id = request
                                    .session_id
                                    .ok_or_else(|| QueueError::Backend("Missing session_id".to_string()));
                                
                                if let Err(e) = session_id {
                                    tracing::error!("Missing session_id in LLM request: {}", e);
                                    let _ = queue_clone.fail(item_id, &e.to_string()).await;
                                    return;
                                }
                                
                                // Extract NPC name from the prompt's responding character
                                let npc_name = request.prompt.responding_character.name.clone();

                                // Extract challenge suggestion from LLM response
                                let challenge_suggestion = response.challenge_suggestion.map(|cs| {
                                    ChallengeSuggestionInfo {
                                        challenge_id: cs.challenge_id,
                                        challenge_name: String::new(), // TODO: Look up challenge name from challenge service
                                        skill_name: String::new(),     // TODO: Look up skill name from skill service
                                        difficulty_display: String::new(), // TODO: Look up difficulty from challenge service
                                        confidence: format!("{:?}", cs.confidence),
                                        reasoning: cs.reasoning,
                                    }
                                });

                                // Extract narrative event suggestion from LLM response
                                let narrative_event_suggestion = response.narrative_event_suggestion.map(|nes| {
                                    NarrativeEventSuggestionInfo {
                                        event_id: nes.event_id,
                                        event_name: String::new(), // TODO: Look up event name from narrative event service
                                        description: String::new(), // TODO: Look up description from narrative event service
                                        scene_direction: String::new(), // TODO: Look up scene direction from narrative event service
                                        confidence: format!("{:?}", nes.confidence),
                                        reasoning: nes.reasoning,
                                        matched_triggers: nes.matched_triggers,
                                    }
                                });

                                let approval = ApprovalItem {
                                    session_id: session_id.unwrap(),
                                    source_action_id: *action_item_id,
                                    decision_type: DecisionType::NPCResponse,
                                    urgency: DecisionUrgency::AwaitingPlayer,
                                    npc_name,
                                    proposed_dialogue: response.npc_dialogue.clone(),
                                    internal_reasoning: response.internal_reasoning.clone(),
                                    proposed_tools: response
                                        .proposed_tool_calls
                                        .iter()
                                        .map(|t| ProposedToolInfo {
                                            id: uuid::Uuid::new_v4().to_string(), // Generate ID for tool call
                                            name: t.tool_name.clone(),
                                            description: format!("Tool call: {}", t.tool_name),
                                            arguments: t.arguments.clone(),
                                        })
                                        .collect(),
                                    retry_count: 0,
                                    challenge_suggestion,
                                    narrative_event_suggestion,
                                };

                                // Enqueue approval and notify DM
                                match approval_queue_clone
                                    .enqueue(approval.clone(), DecisionUrgency::AwaitingPlayer as u8)
                                    .await
                                {
                                    Ok(approval_item_id) => {
                                        // Note: ApprovalRequired message is created and sent by the approval notification worker
                                        // The suggestions are stored in ApprovalItem and will be used by the worker
                                        // No need to create the message here

                                        // Store pending approval in session and send to DM
                                        // This requires access to session manager, which we'll handle in a worker
                                        tracing::info!(
                                            "Enqueued approval {} for NPC {} in session {}",
                                            approval_item_id,
                                            approval.npc_name,
                                            approval.session_id
                                        );

                                        let _ = queue_clone.complete(item_id).await;
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to enqueue approval: {}", e);
                                        let _ = queue_clone.fail(item_id, &e.to_string()).await;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("LLM generation failed: {}", e);
                                let _ = queue_clone.fail(item_id, &e.to_string()).await;
                            }
                        }
                    }
                    LLMRequestType::Suggestion { .. } => {
                        // Send suggestion result via WebSocket (Phase 15)
                        // No DM approval needed
                        tracing::info!("Suggestion request - will be handled in Phase 15");
                        let _ = queue_clone.complete(item_id).await;
                    }
                    LLMRequestType::ChallengeReasoning { .. } => {
                        // Add to approval queue with challenge type
                        // TODO: Implement challenge reasoning approval
                        tracing::info!("Challenge reasoning request - approval not yet implemented");
                        let _ = queue_clone.complete(item_id).await;
                    }
                }
            });
        }
    }
}
