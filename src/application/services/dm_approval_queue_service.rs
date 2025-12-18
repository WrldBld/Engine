//! DM Approval Queue Service - Manages DM approval workflow
//!
//! This service manages the DMApprovalQueue, which holds decisions awaiting
//! DM approval. It provides history, delay, and expiration features.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::application::ports::outbound::{
    ApprovalQueuePort, BroadcastMessage, QueueError, QueueItem, QueueItemId,
    SessionManagementPort,
};
use crate::application::services::tool_execution_service::ToolExecutionService;
use crate::application::dto::ApprovalItem;
use crate::domain::value_objects::{ApprovalDecision, GameTool, SessionId};

/// Maximum number of times a response can be rejected before requiring TakeOver
const MAX_RETRY_COUNT: u32 = 3;

/// Service for managing the DM approval queue
pub struct DMApprovalQueueService<Q: ApprovalQueuePort<ApprovalItem>> {
    pub(crate) queue: Arc<Q>,
    tool_execution_service: ToolExecutionService,
}

impl<Q: ApprovalQueuePort<ApprovalItem>> DMApprovalQueueService<Q> {
    /// Create a new DM approval queue service
    pub fn new(queue: Arc<Q>) -> Self {
        Self {
            queue,
            tool_execution_service: ToolExecutionService::new(),
        }
    }

    /// Get all pending approvals for a session (for DM UI)
    pub async fn get_pending(&self, session_id: SessionId) -> Result<Vec<QueueItem<ApprovalItem>>, QueueError> {
        // The underlying ApprovalQueuePort implementation may not filter by
        // session_id (see SQLite/InMemory comments), so we defensively filter
        // here using the payload's session_id field.
        let items = self.queue.list_by_session(session_id).await?;
        Ok(items
            .into_iter()
            .filter(|item| item.payload.session_id == session_id)
            .collect())
    }

    /// Get an approval item by its string ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<QueueItem<ApprovalItem>>, QueueError> {
        let uuid = uuid::Uuid::parse_str(id)
            .map_err(|e| QueueError::Backend(format!("Invalid UUID: {}", e)))?;
        let item_id = QueueItemId::from_uuid(uuid);
        self.queue.get(item_id).await
    }

    /// Process DM approval decision
    ///
    /// This method handles the DM's decision on an approval request and
    /// routes it to the appropriate handler based on the decision type.
    pub async fn process_decision<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        item_id: QueueItemId,
        decision: ApprovalDecision,
    ) -> Result<ApprovalOutcome, QueueError> {
        let item = self
            .queue
            .get(item_id)
            .await?
            .ok_or_else(|| QueueError::NotFound(item_id.to_string()))?;

        let outcome = match decision {
            ApprovalDecision::Accept => {
                self.handle_accept(session, session_id, &item.payload).await?
            }
            ApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
            } => {
                self.handle_accept_modified(session, session_id, &item.payload, &modified_dialogue, &approved_tools, &rejected_tools).await?
            }
            ApprovalDecision::Reject { feedback } => {
                self.handle_reject(&item.payload, &feedback).await?
            }
            ApprovalDecision::TakeOver { dm_response } => {
                self.handle_takeover(session, session_id, &item.payload, &dm_response).await?
            }
        };

        // Mark item based on outcome
        match &outcome {
            ApprovalOutcome::Broadcast { .. } => {
                self.queue.complete(item_id).await?;
            }
            ApprovalOutcome::Rejected {
                needs_reprocessing: true,
                ..
            } => {
                // Item stays in queue, will be reprocessed
                self.queue
                    .delay(item_id, Utc::now() + Duration::from_secs(1))
                    .await?;
            }
            ApprovalOutcome::Rejected {
                needs_reprocessing: false,
                ..
            }
            | ApprovalOutcome::MaxRetriesExceeded { .. } => {
                self.queue.fail(item_id, "Rejected by DM").await?;
            }
        }

        Ok(outcome)
    }

    /// Handle accepting an approval as-is
    async fn handle_accept<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        approval: &ApprovalItem,
    ) -> Result<ApprovalOutcome, QueueError> {
        // Broadcast the dialogue to players using ServerMessage format
        let message = serde_json::json!({
            "type": "dialogue_response",
            "speaker_id": approval.npc_name,
            "speaker_name": approval.npc_name,
            "text": approval.proposed_dialogue,
            "choices": []
        });

        session
            .broadcast_to_session(
                session_id,
                &BroadcastMessage {
                    content: message,
                },
            )
            .map_err(|e| QueueError::Backend(format!("Session error: {}", e)))?;
        
        // Add to conversation history
        session
            .add_to_conversation_history(session_id, &approval.npc_name, &approval.proposed_dialogue)
            .map_err(|e| QueueError::Backend(format!("Session error: {}", e)))?;

        // Execute approved tool calls
        for tool_info in &approval.proposed_tools {
            // Convert ProposedToolInfo to GameTool
            // Parse the tool arguments JSON to determine tool type
            if let Ok(tool) = self.parse_tool_from_info(tool_info) {
                if let Err(e) = self
                    .tool_execution_service
                    .execute_tool(&tool, session, session_id)
                    .await
                {
                    tracing::warn!("Failed to execute tool {}: {}", tool_info.name, e);
                    // Continue with other tools even if one fails
                }
            }
        }

        Ok(ApprovalOutcome::Broadcast {
            dialogue: approval.proposed_dialogue.clone(),
        })
    }

    /// Handle accepting with modifications
    async fn handle_accept_modified<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        approval: &ApprovalItem,
        modified_dialogue: &str,
        approved_tools: &[String],
        _rejected_tools: &[String],
    ) -> Result<ApprovalOutcome, QueueError> {
        // Broadcast the modified dialogue using ServerMessage format
        let message = serde_json::json!({
            "type": "dialogue_response",
            "speaker_id": approval.npc_name,
            "speaker_name": approval.npc_name,
            "text": modified_dialogue,
            "choices": []
        });

        session
            .broadcast_to_session(
                session_id,
                &BroadcastMessage {
                    content: message,
                },
            )
            .map_err(|e| QueueError::Backend(format!("Session error: {}", e)))?;
        
        // Add to conversation history
        session
            .add_to_conversation_history(session_id, &approval.npc_name, modified_dialogue)
            .map_err(|e| QueueError::Backend(format!("Session error: {}", e)))?;

        // Execute approved tool calls (filter based on approved_tools list)
        // approved_tools contains tool IDs that should be executed
        for tool_info in &approval.proposed_tools {
            // Check if this tool is in the approved list
            if approved_tools.contains(&tool_info.id) {
                if let Ok(tool) = self.parse_tool_from_info(tool_info) {
                    if let Err(e) = self
                        .tool_execution_service
                        .execute_tool(&tool, session, session_id)
                        .await
                    {
                        tracing::warn!("Failed to execute tool {}: {}", tool_info.name, e);
                    }
                }
            }
            // Tools in rejected_tools are simply skipped
        }

        Ok(ApprovalOutcome::Broadcast {
            dialogue: modified_dialogue.to_string(),
        })
    }

    /// Handle rejecting an approval
    async fn handle_reject(
        &self,
        approval: &ApprovalItem,
        feedback: &str,
    ) -> Result<ApprovalOutcome, QueueError> {
        if approval.retry_count >= MAX_RETRY_COUNT {
            return Ok(ApprovalOutcome::MaxRetriesExceeded {
                feedback: feedback.to_string(),
            });
        }

        // Re-enqueue to LLM queue with feedback for regeneration
        // This would require access to the LLM queue service
        // For now, we mark it as needing reprocessing
        // The actual re-enqueue would happen in a worker that processes rejected approvals
        tracing::info!(
            "Approval rejected with feedback: {}. Will be reprocessed.",
            feedback
        );

        Ok(ApprovalOutcome::Rejected {
            feedback: feedback.to_string(),
            needs_reprocessing: true,
        })
    }

    /// Handle DM taking over
    async fn handle_takeover<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        approval: &ApprovalItem,
        dm_response: &str,
    ) -> Result<ApprovalOutcome, QueueError> {
        // Broadcast DM's response using ServerMessage format
        let message = serde_json::json!({
            "type": "dialogue_response",
            "speaker_id": approval.npc_name,
            "speaker_name": approval.npc_name,
            "text": dm_response,
            "choices": []
        });

        session
            .broadcast_to_session(
                session_id,
                &BroadcastMessage {
                    content: message,
                },
            )
            .map_err(|e| QueueError::Backend(format!("Session error: {}", e)))?;
        
        // Add to conversation history
        session
            .add_to_conversation_history(session_id, &approval.npc_name, dm_response)
            .map_err(|e| QueueError::Backend(format!("Session error: {}", e)))?;

        Ok(ApprovalOutcome::Broadcast {
            dialogue: dm_response.to_string(),
        })
    }

    /// Delay a decision for later
    pub async fn delay_decision(
        &self,
        item_id: QueueItemId,
        duration: Duration,
    ) -> Result<(), QueueError> {
        self.queue.delay(item_id, Utc::now() + duration).await
    }

    /// Get decision history for session
    pub async fn get_history(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> Result<Vec<QueueItem<ApprovalItem>>, QueueError> {
        let items = self.queue.get_history(session_id, limit).await?;
        Ok(items
            .into_iter()
            .filter(|item| item.payload.session_id == session_id)
            .collect())
    }

    /// Expire old pending approvals
    pub async fn expire_old(&self, older_than: Duration) -> Result<usize, QueueError> {
        self.queue.expire_old(older_than).await
    }

    /// Discard a challenge suggestion from an approval item
    ///
    /// This is called when the DM doesn't want a challenge suggested by the LLM.
    /// The approval item is marked as failed since the DM rejected the challenge.
    /// A new LLM request should be made for a non-challenge response.
    pub async fn discard_challenge(&self, _client_id: &str, request_id: &str) {
        // Parse request_id to item ID
        if let Ok(uuid) = uuid::Uuid::parse_str(request_id) {
            let item_id = QueueItemId::from_uuid(uuid);

            // Mark the item as failed since the DM rejected the challenge
            if let Err(e) = self.queue.fail(item_id, "Challenge discarded by DM").await {
                tracing::warn!("Failed to mark approval as failed: {}", e);
            }

            // NOTE: Re-enqueueing to LLM queue for a non-challenge response would require:
            // 1. Access to the LLM queue service (add as dependency)
            // 2. Retrieve the original prompt from source_action_id
            // 3. Add guidance to the prompt context (e.g., "do not suggest a challenge")
            // 4. Create a new LLMRequestItem with modified context
            // For now, the DM can manually trigger a new response if needed.
            tracing::info!(
                "Challenge discarded for approval {}. DM should trigger new response if needed.",
                request_id
            );
        } else {
            tracing::warn!("Invalid request_id format: {}", request_id);
        }
    }

    /// Parse ProposedToolInfo into GameTool
    fn parse_tool_from_info(
        &self,
        tool_info: &crate::domain::value_objects::ProposedToolInfo,
    ) -> Result<GameTool, QueueError> {
        // Parse tool based on name and arguments (arguments is serde_json::Value)
        let args = &tool_info.arguments;
        
        match tool_info.name.as_str() {
            "give_item" => {
                let item_name = args
                    .get("item_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing item_name".to_string()))?
                    .to_string();
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing description".to_string()))?
                    .to_string();
                Ok(GameTool::GiveItem {
                    item_name,
                    description,
                })
            }
            "reveal_info" => {
                let info_type = args
                    .get("info_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing info_type".to_string()))?
                    .to_string();
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing content".to_string()))?
                    .to_string();
                let importance_str = args
                    .get("importance")
                    .and_then(|v| v.as_str())
                    .unwrap_or("minor");
                let importance = match importance_str {
                    "minor" => crate::domain::value_objects::InfoImportance::Minor,
                    "major" => crate::domain::value_objects::InfoImportance::Major,
                    "critical" => crate::domain::value_objects::InfoImportance::Critical,
                    _ => crate::domain::value_objects::InfoImportance::Minor,
                };
                Ok(GameTool::RevealInfo {
                    info_type,
                    content,
                    importance,
                })
            }
            "change_relationship" => {
                let change_str = args
                    .get("change")
                    .and_then(|v| v.as_str())
                    .unwrap_or("improve");
                let change = match change_str {
                    "improve" => crate::domain::value_objects::RelationshipChange::Improve,
                    "worsen" => crate::domain::value_objects::RelationshipChange::Worsen,
                    _ => crate::domain::value_objects::RelationshipChange::Improve,
                };
                let amount_str = args
                    .get("amount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("slight");
                let amount = match amount_str {
                    "slight" => crate::domain::value_objects::ChangeAmount::Slight,
                    "moderate" => crate::domain::value_objects::ChangeAmount::Moderate,
                    "significant" => crate::domain::value_objects::ChangeAmount::Significant,
                    _ => crate::domain::value_objects::ChangeAmount::Slight,
                };
                let reason = args
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(GameTool::ChangeRelationship {
                    change,
                    amount,
                    reason,
                })
            }
            "trigger_event" => {
                let event_type = args
                    .get("event_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing event_type".to_string()))?
                    .to_string();
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing description".to_string()))?
                    .to_string();
                Ok(GameTool::TriggerEvent {
                    event_type,
                    description,
                })
            }
            _ => Err(QueueError::Backend(format!("Unknown tool: {}", tool_info.name))),
        }
    }
}

/// Outcome of processing an approval decision
#[derive(Debug, Clone)]
pub enum ApprovalOutcome {
    /// Approval was broadcast to players
    Broadcast {
        dialogue: String,
    },
    /// Approval was rejected
    Rejected {
        feedback: String,
        needs_reprocessing: bool,
    },
    /// Maximum retries exceeded
    MaxRetriesExceeded {
        feedback: String,
    },
}
