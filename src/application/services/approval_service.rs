//! Approval Service - Handles DM approval workflow for NPC responses
//!
//! This service manages the approval workflow where the DM can:
//! - Accept the LLM-generated response as-is
//! - Accept with modifications to dialogue or tool calls
//! - Reject and request re-generation with feedback
//! - Take over and provide a completely custom response
//!
//! # Architecture
//!
//! This service lives in the application layer and depends only on ports:
//! - `SessionManagementPort` for session state access
//! - `GameSessionPort` for conversation history updates
//!
//! It does NOT depend on infrastructure types directly.

use crate::application::ports::outbound::{
    BroadcastMessage, GameSessionPort, PendingApprovalInfo, SessionManagementError,
    SessionManagementPort,
};
use crate::domain::value_objects::SessionId;

/// Maximum number of times a response can be rejected before requiring TakeOver
const MAX_RETRY_COUNT: u32 = 3;

/// Errors that can occur during approval processing
#[derive(Debug, thiserror::Error)]
pub enum ApprovalError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Approval request not found: {0}")]
    ApprovalNotFound(String),

    #[error("Not authorized to approve")]
    NotAuthorized,

    #[error("Maximum retries exceeded")]
    MaxRetriesExceeded,

    #[error("Session error: {0}")]
    SessionError(String),
}

impl From<SessionManagementError> for ApprovalError {
    fn from(err: SessionManagementError) -> Self {
        match err {
            SessionManagementError::SessionNotFound(id) => ApprovalError::SessionNotFound(id),
            SessionManagementError::ApprovalNotFound(id) => ApprovalError::ApprovalNotFound(id),
            SessionManagementError::NotAuthorized => ApprovalError::NotAuthorized,
            _ => ApprovalError::SessionError(err.to_string()),
        }
    }
}

/// DM's decision on an approval request
#[derive(Debug, Clone)]
pub enum ApprovalDecision {
    /// Accept the response as-is
    Accept,
    /// Accept with modifications
    AcceptWithModification {
        /// Modified dialogue text
        modified_dialogue: String,
        /// IDs of tools that are approved
        approved_tools: Vec<String>,
        /// IDs of tools that are rejected
        rejected_tools: Vec<String>,
    },
    /// Reject and request re-generation
    Reject {
        /// Feedback for the LLM to improve the response
        feedback: String,
    },
    /// DM takes over with their own response
    TakeOver {
        /// DM's custom dialogue
        dm_response: String,
    },
}

/// Result of processing an approval decision
#[derive(Debug, Clone)]
pub struct ApprovalResult {
    /// The request ID that was processed
    pub request_id: String,
    /// Whether the response was broadcast to players
    pub broadcast_sent: bool,
    /// If rejected, the current retry count
    pub retry_count: Option<u32>,
    /// If max retries exceeded
    pub max_retries_exceeded: bool,
}

/// Service for handling DM approval workflow
///
/// This service orchestrates the approval flow:
/// - Validates the DM has authority
/// - Processes the decision
/// - Updates session state
/// - Broadcasts results to players
pub struct ApprovalService;

impl ApprovalService {
    /// Create a new approval service
    pub fn new() -> Self {
        Self
    }

    /// Process an approval decision from the DM
    ///
    /// # Arguments
    ///
    /// * `session` - The session management port for accessing session state
    /// * `session_id` - The session where the approval is happening
    /// * `client_id` - The client making the decision (must be DM)
    /// * `request_id` - The approval request ID
    /// * `decision` - The DM's decision
    ///
    /// # Returns
    ///
    /// An `ApprovalResult` indicating what happened
    pub fn process_decision<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        client_id: u64,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> Result<ApprovalResult, ApprovalError> {
        // Verify the client is the DM
        if !session.is_client_dm(client_id) {
            return Err(ApprovalError::NotAuthorized);
        }

        // Get the pending approval
        let pending = session
            .get_pending_approval(session_id, request_id)
            .ok_or_else(|| ApprovalError::ApprovalNotFound(request_id.to_string()))?;

        match decision {
            ApprovalDecision::Accept => {
                self.handle_accept(session, session_id, request_id, &pending)
            }
            ApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
            } => self.handle_accept_with_modification(
                session,
                session_id,
                request_id,
                &pending,
                &modified_dialogue,
                &approved_tools,
                &rejected_tools,
            ),
            ApprovalDecision::Reject { feedback } => {
                self.handle_reject(session, session_id, request_id, &pending, &feedback)
            }
            ApprovalDecision::TakeOver { dm_response } => {
                self.handle_takeover(session, session_id, request_id, &pending, &dm_response)
            }
        }
    }

    /// Handle Accept decision - broadcast approved response to players
    fn handle_accept<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        request_id: &str,
        pending: &PendingApprovalInfo,
    ) -> Result<ApprovalResult, ApprovalError> {
        // Build dialogue response message
        let dialogue_message = self.build_dialogue_message(
            &pending.npc_name,
            &pending.proposed_dialogue,
        );

        // Broadcast to players
        session.broadcast_to_players(session_id, &dialogue_message)?;

        // Store in conversation history
        session.add_to_conversation_history(
            session_id,
            &pending.npc_name,
            &pending.proposed_dialogue,
        )?;

        // Remove from pending approvals
        session.remove_pending_approval(session_id, request_id)?;

        tracing::info!(
            "Approved NPC response for request {}. Executed tools: {:?}",
            request_id,
            pending.proposed_tools.iter().map(|t| &t.id).collect::<Vec<_>>()
        );

        Ok(ApprovalResult {
            request_id: request_id.to_string(),
            broadcast_sent: true,
            retry_count: None,
            max_retries_exceeded: false,
        })
    }

    /// Handle AcceptWithModification - use modified dialogue, filter tools
    fn handle_accept_with_modification<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        request_id: &str,
        pending: &PendingApprovalInfo,
        modified_dialogue: &str,
        approved_tools: &[String],
        _rejected_tools: &[String],
    ) -> Result<ApprovalResult, ApprovalError> {
        // Build dialogue response with modified text
        let dialogue_message = self.build_dialogue_message(
            &pending.npc_name,
            modified_dialogue,
        );

        // Broadcast to players
        session.broadcast_to_players(session_id, &dialogue_message)?;

        // Store modified dialogue in conversation history
        session.add_to_conversation_history(
            session_id,
            &pending.npc_name,
            modified_dialogue,
        )?;

        // Remove from pending approvals
        session.remove_pending_approval(session_id, request_id)?;

        // Log which tools were approved
        let filtered_tools: Vec<&String> = pending
            .proposed_tools
            .iter()
            .filter(|t| approved_tools.contains(&t.id))
            .map(|t| &t.id)
            .collect();

        tracing::info!(
            "Approved modified NPC response for request {}. Approved tools: {:?}",
            request_id,
            filtered_tools
        );

        Ok(ApprovalResult {
            request_id: request_id.to_string(),
            broadcast_sent: true,
            retry_count: None,
            max_retries_exceeded: false,
        })
    }

    /// Handle Reject decision - increment retry count, potentially re-call LLM
    fn handle_reject<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        request_id: &str,
        _pending: &PendingApprovalInfo,
        feedback: &str,
    ) -> Result<ApprovalResult, ApprovalError> {
        // Increment retry count
        let new_count = session.increment_retry_count(session_id, request_id)?;

        // Check if max retries exceeded
        if new_count >= MAX_RETRY_COUNT {
            // Remove the pending approval
            session.remove_pending_approval(session_id, request_id)?;

            // Notify DM of max retries
            let error_message = BroadcastMessage {
                content: serde_json::json!({
                    "type": "Error",
                    "code": "APPROVAL_MAX_RETRIES",
                    "message": "Maximum approval retries exceeded. Please use TakeOver instead.",
                }),
            };
            session.send_to_dm(session_id, &error_message)?;

            tracing::warn!(
                "Max retries ({}) exceeded for request {}. Rejecting.",
                MAX_RETRY_COUNT,
                request_id
            );

            return Ok(ApprovalResult {
                request_id: request_id.to_string(),
                broadcast_sent: false,
                retry_count: Some(new_count),
                max_retries_exceeded: true,
            });
        }

        tracing::info!(
            "Rejection #{} for request {}. Feedback: {}",
            new_count,
            request_id,
            feedback
        );

        // Notify DM that re-processing will happen
        let processing_message = BroadcastMessage {
            content: serde_json::json!({
                "type": "LLMProcessing",
                "action_id": request_id,
            }),
        };
        session.send_to_dm(session_id, &processing_message)?;

        // Note: The actual re-call to LLM would be triggered separately
        // The caller should initiate a new LLM call with the feedback context

        Ok(ApprovalResult {
            request_id: request_id.to_string(),
            broadcast_sent: false,
            retry_count: Some(new_count),
            max_retries_exceeded: false,
        })
    }

    /// Handle TakeOver decision - use DM's custom response
    fn handle_takeover<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        request_id: &str,
        pending: &PendingApprovalInfo,
        dm_response: &str,
    ) -> Result<ApprovalResult, ApprovalError> {
        // Build dialogue response with DM's text
        let dialogue_message = self.build_dialogue_message(
            &pending.npc_name,
            dm_response,
        );

        // Broadcast to players
        session.broadcast_to_players(session_id, &dialogue_message)?;

        // Store DM's response in conversation history
        session.add_to_conversation_history(
            session_id,
            &pending.npc_name,
            dm_response,
        )?;

        // Remove from pending approvals
        session.remove_pending_approval(session_id, request_id)?;

        tracing::info!("DM took over response for request {}", request_id);

        Ok(ApprovalResult {
            request_id: request_id.to_string(),
            broadcast_sent: true,
            retry_count: None,
            max_retries_exceeded: false,
        })
    }

    /// Build a dialogue response message for broadcasting
    fn build_dialogue_message(&self, npc_name: &str, dialogue: &str) -> BroadcastMessage {
        BroadcastMessage {
            content: serde_json::json!({
                "type": "DialogueResponse",
                "speaker_id": npc_name,
                "speaker_name": npc_name,
                "text": dialogue,
                "choices": [],
            }),
        }
    }
}

impl Default for ApprovalService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here using mock implementations of the ports
}
