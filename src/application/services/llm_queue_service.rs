//! LLM Queue Service - Concurrency-controlled LLM processing
//!
//! This service manages the LLMReasoningQueue, which processes LLM requests
//! with controlled concurrency using semaphores. It routes responses to the
//! DMApprovalQueue for NPC responses.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use crate::application::ports::outbound::{
    ApprovalQueuePort, BroadcastMessage, LlmPort, ProcessingQueuePort, QueueError, QueueItemId,
    QueuePort, SessionManagementPort,
};
use crate::application::services::llm_service::LLMService;
use crate::domain::value_objects::{
    ApprovalItem, DecisionType, DecisionUrgency, LLMRequestItem, LLMRequestType, ProposedToolInfo,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Priority constants for queue operations
const PRIORITY_NORMAL: u8 = 0;
const PRIORITY_HIGH: u8 = 1;

/// Service for managing the LLM reasoning queue
pub struct LLMQueueService<Q: ProcessingQueuePort<LLMRequestItem>, L: LlmPort> {
    pub(crate) queue: Arc<Q>,
    llm_service: Arc<LLMService<L>>,
    approval_queue: Arc<dyn ApprovalQueuePort<ApprovalItem>>,
    semaphore: Arc<Semaphore>,
}

impl<Q: ProcessingQueuePort<LLMRequestItem>, L: LlmPort> LLMQueueService<Q, L> {
    /// Create a new LLM queue service
    ///
    /// # Arguments
    ///
    /// * `queue` - The LLM request queue
    /// * `llm_client` - The LLM client for processing requests
    /// * `approval_queue` - The approval queue for routing NPC responses
    /// * `batch_size` - Maximum concurrent LLM requests (default: 1)
    pub fn new(
        queue: Arc<Q>,
        llm_client: Arc<L>,
        approval_queue: Arc<dyn ApprovalQueuePort<ApprovalItem>>,
        batch_size: usize,
    ) -> Self {
        Self {
            queue,
            llm_service: Arc::new(LLMService::new((*llm_client).clone())),
            approval_queue,
            semaphore: Arc::new(Semaphore::new(batch_size.max(1))),
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
    pub async fn run_worker(&self) {
        loop {
            // Wait for capacity
            let permit = match self.semaphore.acquire().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Semaphore error: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Try to get next item
            let item = match self.queue.dequeue().await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    drop(permit);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => {
                    tracing::error!("Failed to dequeue LLM request: {}", e);
                    drop(permit);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Process in spawned task (permit moves into task)
            let llm_service = self.llm_service.clone();
            let queue = self.queue.clone();
            let approval_queue = self.approval_queue.clone();
            let request = item.payload.clone();
            let item_id = item.id;

            tokio::spawn(async move {
                let _permit = permit; // Keep permit alive during processing

                match &request.request_type {
                    LLMRequestType::NPCResponse { action_item_id } => {
                        // Process NPC response request
                        match llm_service.generate_npc_response(request.prompt.clone()).await {
                            Ok(response) => {
                                // Create approval item for DM
                                let session_id = request
                                    .session_id
                                    .ok_or_else(|| QueueError::Backend("Missing session_id".to_string()));
                                
                                if let Err(e) = session_id {
                                    tracing::error!("Missing session_id in LLM request: {}", e);
                                    let _ = queue.fail(item_id, &e.to_string()).await;
                                    return;
                                }
                                
                                // Extract NPC name from the prompt's responding character
                                let npc_name = request.prompt.responding_character.name.clone();

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
                                };

                                // Enqueue approval and notify DM
                                match approval_queue
                                    .enqueue(approval.clone(), DecisionUrgency::AwaitingPlayer as u8)
                                    .await
                                {
                                    Ok(approval_item_id) => {
                                        // Convert ProposedToolInfo to ProposedTool for WebSocket message
                                        let proposed_tools: Vec<crate::infrastructure::websocket::ProposedTool> = approval
                                            .proposed_tools
                                            .iter()
                                            .map(|t| crate::infrastructure::websocket::ProposedTool {
                                                id: t.id.clone(),
                                                name: t.name.clone(),
                                                description: t.description.clone(),
                                                arguments: t.arguments.clone(),
                                            })
                                            .collect();

                                        // Create ApprovalRequired message
                                        let approval_msg = crate::infrastructure::websocket::ServerMessage::ApprovalRequired {
                                            request_id: approval_item_id.to_string(),
                                            npc_name: approval.npc_name.clone(),
                                            proposed_dialogue: approval.proposed_dialogue.clone(),
                                            internal_reasoning: approval.internal_reasoning.clone(),
                                            proposed_tools,
                                            challenge_suggestion: None, // TODO: Extract from LLM response if available
                                            narrative_event_suggestion: None, // TODO: Extract from LLM response if available
                                        };

                                        // Store pending approval in session and send to DM
                                        // This requires access to session manager, which we'll handle in a worker
                                        tracing::info!(
                                            "Enqueued approval {} for NPC {} in session {}",
                                            approval_item_id,
                                            approval.npc_name,
                                            approval.session_id
                                        );

                                        let _ = queue.complete(item_id).await;
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to enqueue approval: {}", e);
                                        let _ = queue.fail(item_id, &e.to_string()).await;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("LLM generation failed: {}", e);
                                let _ = queue.fail(item_id, &e.to_string()).await;
                            }
                        }
                    }
                    LLMRequestType::Suggestion { .. } => {
                        // Send suggestion result via WebSocket (Phase 15)
                        // No DM approval needed
                        tracing::info!("Suggestion request - will be handled in Phase 15");
                        let _ = queue.complete(item_id).await;
                    }
                    LLMRequestType::ChallengeReasoning { .. } => {
                        // Add to approval queue with challenge type
                        // TODO: Implement challenge reasoning approval
                        tracing::info!("Challenge reasoning request - approval not yet implemented");
                        let _ = queue.complete(item_id).await;
                    }
                }
            });
        }
    }

}
