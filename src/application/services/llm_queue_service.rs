//! LLM Queue Service - Concurrency-controlled LLM processing
//!
//! This service manages the LLMReasoningQueue, which processes LLM requests
//! with controlled concurrency using semaphores. It routes responses to the
//! DMApprovalQueue for NPC responses.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use crate::application::ports::outbound::{
    ApprovalQueuePort, ChallengeRepositoryPort, LlmPort, NarrativeEventRepositoryPort,
    ProcessingQueuePort, QueueError, QueueItemId, QueueItemStatus, QueueNotificationPort, SkillRepositoryPort,
};
use crate::application::services::llm::LLMService;
use crate::application::services::generation_service::GenerationEvent;
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
    llm_client: Arc<L>, // Keep for SuggestionService
    approval_queue: Arc<dyn ApprovalQueuePort<ApprovalItem>>,
    challenge_repo: Arc<dyn ChallengeRepositoryPort>,
    skill_repo: Arc<dyn SkillRepositoryPort>,
    narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort>,
    semaphore: Arc<Semaphore>,
    notifier: N,
    generation_event_tx: tokio::sync::mpsc::UnboundedSender<GenerationEvent>,
}

impl<Q: ProcessingQueuePort<LLMRequestItem> + 'static, L: LlmPort + Clone + 'static, N: QueueNotificationPort + 'static> LLMQueueService<Q, L, N> {
    /// Create a new LLM queue service
    ///
    /// # Arguments
    ///
    /// * `queue` - The LLM request queue
    /// * `llm_client` - The LLM client for processing requests
    /// * `approval_queue` - The approval queue for routing NPC responses
    /// * `challenge_repo` - Repository for looking up challenge details
    /// * `skill_repo` - Repository for looking up skill details
    /// * `narrative_event_repo` - Repository for looking up narrative event details
    /// * `batch_size` - Maximum concurrent LLM requests (default: 1)
    /// * `notifier` - The notifier for waking workers
    /// * `generation_event_tx` - Channel for emitting generation events (suggestions)
    pub fn new(
        queue: Arc<Q>,
        llm_client: Arc<L>,
        approval_queue: Arc<dyn ApprovalQueuePort<ApprovalItem>>,
        challenge_repo: Arc<dyn ChallengeRepositoryPort>,
        skill_repo: Arc<dyn SkillRepositoryPort>,
        narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort>,
        batch_size: usize,
        notifier: N,
        generation_event_tx: tokio::sync::mpsc::UnboundedSender<GenerationEvent>,
    ) -> Self {
        Self {
            queue,
            llm_service: Arc::new(LLMService::new((*llm_client).clone())),
            llm_client,
            approval_queue,
            challenge_repo,
            skill_repo,
            narrative_event_repo,
            semaphore: Arc::new(Semaphore::new(batch_size.max(1))),
            notifier,
            generation_event_tx,
        }
    }

    /// Enqueue an LLM request
    pub async fn enqueue(&self, request: LLMRequestItem) -> Result<QueueItemId, QueueError> {
        self.queue.enqueue(request, PRIORITY_NORMAL).await
    }

    /// Cancel a suggestion request by its callback_id (request_id)
    pub async fn cancel_suggestion(&self, request_id: &str) -> Result<bool, QueueError> {
        // Search through pending and processing items
        let pending_items = self.queue.list_by_status(QueueItemStatus::Pending).await?;
        let processing_items = self.queue.list_by_status(QueueItemStatus::Processing).await?;
        
        // Find item with matching callback_id
        for item in pending_items.iter().chain(processing_items.iter()) {
            if item.payload.callback_id == request_id {
                // Mark as failed with cancellation message
                self.queue.fail(item.id, "Cancelled by user").await?;
                
                // Emit cancellation event
                let _ = self.generation_event_tx.send(GenerationEvent::SuggestionFailed {
                    request_id: request_id.to_string(),
                    field_type: match &item.payload.request_type {
                        LLMRequestType::Suggestion { field_type, .. } => field_type.clone(),
                        _ => String::new(),
                    },
                    error: "Cancelled by user".to_string(),
                    world_id: item.payload.world_id.clone(),
                });
                
                return Ok(true);
            }
        }
        
        Ok(false) // Not found
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
            let llm_client_clone = self.llm_client.clone();
            let queue_clone = self.queue.clone();
            let approval_queue_clone = self.approval_queue.clone();
            let challenge_repo_clone = self.challenge_repo.clone();
            let skill_repo_clone = self.skill_repo.clone();
            let narrative_event_repo_clone = self.narrative_event_repo.clone();
            let generation_event_tx_clone = self.generation_event_tx.clone();
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
                        let Some(prompt) = request.prompt.as_ref() else {
                            let error = "Missing prompt for NPC response".to_string();
                            tracing::error!("{}", error);
                            let _ = queue_clone.fail(item_id, &error).await;
                            return;
                        };
                        match llm_service_clone.generate_npc_response(prompt.clone()).await {
                            Ok(response) => {
                                // Create approval item for DM
                                let Some(session_id) = request.session_id else {
                                    tracing::error!("Missing session_id in LLM request");
                                    let _ = queue_clone.fail(item_id, "Missing session_id").await;
                                    return;
                                };

                                // Extract NPC name from the prompt's responding character
                                let npc_name = prompt.responding_character.name.clone();

                                // Extract challenge suggestion from LLM response
                                let challenge_suggestion = if let Some(cs) = response.challenge_suggestion {
                                    // Parse the challenge ID from string
                                    let challenge_id_result = uuid::Uuid::parse_str(&cs.challenge_id)
                                        .map(crate::domain::value_objects::ChallengeId::from_uuid);
                                    
                                    match challenge_id_result {
                                        Ok(challenge_id) => {
                                    // Look up challenge details
                                    match challenge_repo_clone.get(challenge_id).await {
                                        Ok(Some(challenge)) => {
                                            // Fetch skill_id from REQUIRES_SKILL edge
                                            let skill_id = match challenge_repo_clone.get_required_skill(challenge_id).await {
                                                Ok(sid) => sid,
                                                Err(e) => {
                                                    tracing::warn!("Failed to get required skill for challenge {}: {}", challenge_id, e);
                                                    None
                                                }
                                            };

                                            // Look up skill name
                                            let skill_name = if let Some(sid) = skill_id {
                                                match skill_repo_clone.get(sid).await {
                                                    Ok(Some(skill)) => skill.name,
                                                    Ok(None) => {
                                                        tracing::warn!("Skill {} not found for challenge {}", sid, cs.challenge_id);
                                                        sid.to_string()
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("Failed to look up skill {}: {}", sid, e);
                                                        sid.to_string()
                                                    }
                                                }
                                            } else {
                                                "Unknown Skill".to_string()
                                            };

                                            Some(ChallengeSuggestionInfo {
                                                challenge_id: cs.challenge_id,
                                                challenge_name: challenge.name,
                                                skill_name,
                                                difficulty_display: challenge.difficulty.display(),
                                                confidence: format!("{:?}", cs.confidence),
                                                reasoning: cs.reasoning,
                                                target_pc_id: request.pc_id.map(|id| id.to_string()),
                                            })
                                        }
                                        Ok(None) => {
                                            tracing::warn!("Challenge {} not found, using minimal info", cs.challenge_id);
                                            Some(ChallengeSuggestionInfo {
                                                challenge_id: cs.challenge_id.clone(),
                                                challenge_name: format!("Challenge {}", cs.challenge_id),
                                                skill_name: String::new(),
                                                difficulty_display: String::new(),
                                                confidence: format!("{:?}", cs.confidence),
                                                reasoning: cs.reasoning.clone(),
                                                target_pc_id: request.pc_id.map(|id| id.to_string()),
                                            })
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to look up challenge {}: {}", cs.challenge_id, e);
                                            Some(ChallengeSuggestionInfo {
                                                challenge_id: cs.challenge_id.clone(),
                                                challenge_name: format!("Challenge {}", cs.challenge_id),
                                                skill_name: String::new(),
                                                difficulty_display: String::new(),
                                                confidence: format!("{:?}", cs.confidence),
                                                reasoning: cs.reasoning.clone(),
                                                target_pc_id: request.pc_id.map(|id| id.to_string()),
                                            })
                                        }
                                    }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to parse challenge ID {}: {}", cs.challenge_id, e);
                                            Some(ChallengeSuggestionInfo {
                                                challenge_id: cs.challenge_id,
                                                challenge_name: format!("Challenge {}", "invalid-id"),
                                                skill_name: String::new(),
                                                difficulty_display: String::new(),
                                                confidence: format!("{:?}", cs.confidence),
                                                reasoning: cs.reasoning,
                                                target_pc_id: request.pc_id.map(|id| id.to_string()),
                                            })
                                        }
                                    }
                                } else {
                                    None
                                };

                                // Extract narrative event suggestion from LLM response
                                let narrative_event_suggestion = if let Some(nes) = response.narrative_event_suggestion {
                                    // Parse the narrative event ID from string
                                    let event_id_result = uuid::Uuid::parse_str(&nes.event_id)
                                        .map(crate::domain::value_objects::NarrativeEventId::from_uuid);
                                    
                                    match event_id_result {
                                        Ok(event_id) => {
                                    // Look up narrative event details
                                    match narrative_event_repo_clone.get(event_id).await {
                                        Ok(Some(event)) => {
                                            Some(NarrativeEventSuggestionInfo {
                                                event_id: nes.event_id.clone(),
                                                event_name: event.name,
                                                description: event.description,
                                                scene_direction: event.scene_direction,
                                                confidence: format!("{:?}", nes.confidence),
                                                reasoning: nes.reasoning.clone(),
                                                matched_triggers: nes.matched_triggers.clone(),
                                            })
                                        }
                                        Ok(None) => {
                                            tracing::warn!("Narrative event {} not found, using minimal info", nes.event_id);
                                            Some(NarrativeEventSuggestionInfo {
                                                event_id: nes.event_id.clone(),
                                                event_name: format!("Event {}", nes.event_id),
                                                description: String::new(),
                                                scene_direction: String::new(),
                                                confidence: format!("{:?}", nes.confidence),
                                                reasoning: nes.reasoning.clone(),
                                                matched_triggers: nes.matched_triggers.clone(),
                                            })
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to look up narrative event {}: {}", nes.event_id, e);
                                            Some(NarrativeEventSuggestionInfo {
                                                event_id: nes.event_id.clone(),
                                                event_name: format!("Event {}", nes.event_id),
                                                description: String::new(),
                                                scene_direction: String::new(),
                                                confidence: format!("{:?}", nes.confidence),
                                                reasoning: nes.reasoning.clone(),
                                                matched_triggers: nes.matched_triggers.clone(),
                                            })
                                        }
                                    }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to parse narrative event ID {}: {}", nes.event_id, e);
                                            Some(NarrativeEventSuggestionInfo {
                                                event_id: nes.event_id,
                                                event_name: format!("Event {}", "invalid-id"),
                                                description: String::new(),
                                                scene_direction: String::new(),
                                                confidence: format!("{:?}", nes.confidence),
                                                reasoning: nes.reasoning,
                                                matched_triggers: nes.matched_triggers,
                                            })
                                        }
                                    }
                                } else {
                                    None
                                };

                                let approval = ApprovalItem {
                                    session_id,
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
                    LLMRequestType::Suggestion { field_type, entity_id } => {
                        // Process suggestion request
                        let Some(context) = request.suggestion_context.as_ref() else {
                            let error = "Missing suggestion context".to_string();
                            tracing::error!("{}", error);
                            let _ = queue_clone.fail(item_id, &error).await;
                            return;
                        };
                        
                        let request_id = request.callback_id.clone();
                        let field_type_clone = field_type.clone();
                        let entity_id_clone = entity_id.clone();
                        let world_id_clone = request.world_id.clone();
                        
                        // Emit queued event
                        let _ = generation_event_tx_clone.send(GenerationEvent::SuggestionQueued {
                            request_id: request_id.clone(),
                            field_type: field_type_clone.clone(),
                            entity_id: entity_id_clone.clone(),
                            world_id: world_id_clone.clone(),
                        });
                        
                        // Create suggestion service
                        use crate::application::services::SuggestionService;
                        let suggestion_service = SuggestionService::new((*llm_client_clone).clone());
                        
                        // Process based on field type
                        let result = match field_type.as_str() {
                            "character_name" => suggestion_service.suggest_character_names(context).await,
                            "character_description" => suggestion_service.suggest_character_description(context).await,
                            "character_wants" => suggestion_service.suggest_character_wants(context).await,
                            "character_fears" => suggestion_service.suggest_character_fears(context).await,
                            "character_backstory" => suggestion_service.suggest_character_backstory(context).await,
                            "location_name" => suggestion_service.suggest_location_names(context).await,
                            "location_description" => suggestion_service.suggest_location_description(context).await,
                            "location_atmosphere" => suggestion_service.suggest_location_atmosphere(context).await,
                            "location_features" => suggestion_service.suggest_location_features(context).await,
                            "location_secrets" => suggestion_service.suggest_location_secrets(context).await,
                            _ => {
                                let error = format!("Unknown suggestion field type: {}", field_type);
                                tracing::error!("{}", error);
                                let _ = generation_event_tx_clone.send(GenerationEvent::SuggestionFailed {
                                    request_id: request_id.clone(),
                                    field_type: field_type_clone.clone(),
                                    error: error.clone(),
                                    world_id: world_id_clone.clone(),
                                });
                                let _ = queue_clone.fail(item_id, &error).await;
                                return;
                            }
                        };
                        
                        match result {
                            Ok(suggestions) => {
                                tracing::info!("Suggestion request {} completed with {} suggestions", request_id, suggestions.len());
                                let _ = generation_event_tx_clone.send(GenerationEvent::SuggestionComplete {
                                    request_id: request_id.clone(),
                                    field_type: field_type_clone.clone(),
                                    suggestions,
                                    world_id: world_id_clone,
                                });
                                let _ = queue_clone.complete(item_id).await;
                            }
                            Err(e) => {
                                let error = e.to_string();
                                tracing::error!("Suggestion request {} failed: {}", request_id, error);
                                let _ = generation_event_tx_clone.send(GenerationEvent::SuggestionFailed {
                                    request_id: request_id.clone(),
                                    field_type: field_type_clone.clone(),
                                    error: error.clone(),
                                    world_id: world_id_clone,
                                });
                                let _ = queue_clone.fail(item_id, &error).await;
                            }
                        }
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
