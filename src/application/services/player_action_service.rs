//! Player Action Service - Orchestrates player action processing through LLM
//!
//! This service handles the core gameplay loop where a player takes an action,
//! it gets processed through the LLM to generate an NPC response, and that
//! response is sent to the DM for approval.
//!
//! # Architecture
//!
//! This service lives in the application layer and depends only on ports:
//! - `SessionManagementPort` for session state access
//! - `LlmPort` for LLM interaction (via LLMService)
//!
//! It does NOT depend on infrastructure types directly.

use crate::application::ports::outbound::{
    BroadcastMessage, LlmPort, PendingApprovalInfo, ProposedToolInfo,
    SessionManagementPort, SessionWorldContext,
};
use crate::application::services::llm_service::{
    CharacterContext, GamePromptRequest, LLMService, PlayerActionContext, SceneContext,
};
use crate::domain::value_objects::SessionId;

/// Errors that can occur during player action processing
#[derive(Debug, thiserror::Error)]
pub enum PlayerActionError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("No responding character found")]
    NoRespondingCharacter,

    #[error("No scene context available")]
    NoSceneContext,

    #[error("LLM processing failed: {0}")]
    LlmError(String),

    #[error("Session error: {0}")]
    SessionError(String),
}

/// Result of processing a player action
#[derive(Debug, Clone)]
pub struct PlayerActionResult {
    /// The action ID for tracking
    pub action_id: String,
    /// Whether an approval request was sent to the DM
    pub approval_requested: bool,
}

/// Service for processing player actions through LLM
///
/// This service orchestrates the flow of:
/// 1. Receiving a player action
/// 2. Building context from session state
/// 3. Calling the LLM to generate an NPC response
/// 4. Creating a pending approval for the DM
/// 5. Notifying the DM of the approval request
pub struct PlayerActionService<L: LlmPort> {
    llm_service: LLMService<L>,
}

impl<L: LlmPort> PlayerActionService<L> {
    /// Create a new player action service
    pub fn new(llm_client: L) -> Self {
        Self {
            llm_service: LLMService::new(llm_client),
        }
    }

    /// Process a player action through the LLM and create an approval request
    ///
    /// # Arguments
    ///
    /// * `session` - The session management port for accessing session state
    /// * `session_id` - The session where the action occurred
    /// * `action_id` - Unique ID for this action
    /// * `action_type` - Type of action (e.g., "speak", "examine")
    /// * `target` - Optional target of the action (e.g., NPC name)
    /// * `dialogue` - Optional player dialogue
    ///
    /// # Returns
    ///
    /// A `PlayerActionResult` indicating success, or an error
    pub async fn process_action<S: SessionManagementPort>(
        &self,
        session: &mut S,
        session_id: SessionId,
        action_id: String,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> Result<PlayerActionResult, PlayerActionError> {
        // Get world context from session
        let world_context = session
            .get_session_world_context(session_id)
            .ok_or_else(|| PlayerActionError::SessionNotFound(session_id.to_string()))?;

        // Determine the responding character
        let responding_char_name = self.determine_responding_character(&target, &world_context)?;
        let character_info = world_context
            .characters
            .get(&responding_char_name)
            .ok_or(PlayerActionError::NoRespondingCharacter)?;

        // Build the game prompt request
        let prompt_request = self.build_prompt_request(
            &action_type,
            &target,
            &dialogue,
            &world_context,
            character_info,
        );

        // Call LLM service
        let response = self
            .llm_service
            .generate_npc_response(prompt_request)
            .await
            .map_err(|e| PlayerActionError::LlmError(e.to_string()))?;

        // Convert proposed tool calls to pending approval format
        let proposed_tools: Vec<ProposedToolInfo> = response
            .proposed_tool_calls
            .iter()
            .map(|tool| ProposedToolInfo {
                id: format!("{}_{}", tool.tool_name, uuid::Uuid::new_v4()),
                name: tool.tool_name.clone(),
                description: tool.description.clone(),
                arguments: tool.arguments.clone(),
            })
            .collect();

        // Create pending approval
        let pending_approval = PendingApprovalInfo {
            request_id: action_id.clone(),
            npc_name: responding_char_name.clone(),
            proposed_dialogue: response.npc_dialogue.clone(),
            internal_reasoning: response.internal_reasoning.clone(),
            proposed_tools: proposed_tools.clone(),
            retry_count: 0,
        };

        // Store pending approval
        session
            .add_pending_approval(session_id, pending_approval)
            .map_err(|e| PlayerActionError::SessionError(e.to_string()))?;

        // Build approval message for DM
        let approval_message = self.build_approval_message(
            &action_id,
            &responding_char_name,
            &response.npc_dialogue,
            &response.internal_reasoning,
            &proposed_tools,
            &response.challenge_suggestion,
            &response.narrative_event_suggestion,
        );

        // Send to DM
        session
            .send_to_dm(session_id, &approval_message)
            .map_err(|e| PlayerActionError::SessionError(e.to_string()))?;

        tracing::info!(
            "Sent ApprovalRequired for action {} to DM",
            action_id
        );

        Ok(PlayerActionResult {
            action_id,
            approval_requested: true,
        })
    }

    /// Determine which character should respond to the action
    fn determine_responding_character(
        &self,
        target: &Option<String>,
        context: &SessionWorldContext,
    ) -> Result<String, PlayerActionError> {
        if let Some(target_name) = target {
            // Try to find the target character (case-insensitive)
            for char_name in context.characters.keys() {
                if char_name.eq_ignore_ascii_case(target_name) {
                    return Ok(char_name.clone());
                }
            }
        }

        // Fall back to first present character
        context
            .present_character_names
            .first()
            .and_then(|name| {
                context
                    .characters
                    .keys()
                    .find(|k| k.eq_ignore_ascii_case(name))
                    .cloned()
            })
            .ok_or(PlayerActionError::NoRespondingCharacter)
    }

    /// Build the game prompt request for the LLM
    fn build_prompt_request(
        &self,
        action_type: &str,
        target: &Option<String>,
        dialogue: &Option<String>,
        world_context: &SessionWorldContext,
        character_info: &crate::application::ports::outbound::CharacterContextInfo,
    ) -> GamePromptRequest {
        let scene_context = SceneContext {
            scene_name: world_context.scene_name.clone(),
            location_name: world_context.location_name.clone(),
            time_context: world_context.time_context.clone(),
            present_characters: world_context.present_character_names.clone(),
        };

        let character_context = CharacterContext {
            name: character_info.name.clone(),
            archetype: character_info.archetype.clone(),
            current_mood: None,
            wants: character_info.wants.clone(),
            relationship_to_player: None,
        };

        GamePromptRequest {
            player_action: PlayerActionContext {
                action_type: action_type.to_string(),
                target: target.clone(),
                dialogue: dialogue.clone(),
            },
            scene_context,
            directorial_notes: world_context.directorial_notes.clone(),
            conversation_history: vec![],
            responding_character: character_context,
            active_challenges: vec![],
            active_narrative_events: vec![],
        }
    }

    /// Build the approval message to send to the DM
    fn build_approval_message(
        &self,
        action_id: &str,
        npc_name: &str,
        proposed_dialogue: &str,
        internal_reasoning: &str,
        proposed_tools: &[ProposedToolInfo],
        challenge_suggestion: &Option<crate::application::services::llm_service::ChallengeSuggestion>,
        narrative_event_suggestion: &Option<crate::application::services::llm_service::NarrativeEventSuggestion>,
    ) -> BroadcastMessage {
        // Build the message structure that matches ServerMessage::ApprovalRequired
        let message_value = serde_json::json!({
            "type": "ApprovalRequired",
            "request_id": action_id,
            "npc_name": npc_name,
            "proposed_dialogue": proposed_dialogue,
            "internal_reasoning": internal_reasoning,
            "proposed_tools": proposed_tools.iter().map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "name": t.name,
                    "description": t.description,
                    "arguments": t.arguments,
                })
            }).collect::<Vec<_>>(),
            "challenge_suggestion": challenge_suggestion.as_ref().map(|cs| {
                serde_json::json!({
                    "challenge_id": cs.challenge_id,
                    "challenge_name": "",
                    "skill_name": "",
                    "difficulty_display": "",
                    "confidence": format!("{:?}", cs.confidence).to_lowercase(),
                    "reasoning": cs.reasoning,
                })
            }),
            "narrative_event_suggestion": narrative_event_suggestion.as_ref().map(|nes| {
                serde_json::json!({
                    "event_id": nes.event_id,
                    "event_name": "",
                    "description": "",
                    "scene_direction": "",
                    "confidence": format!("{:?}", nes.confidence).to_lowercase(),
                    "reasoning": nes.reasoning,
                    "matched_triggers": nes.matched_triggers,
                })
            }),
        });

        BroadcastMessage {
            content: message_value,
        }
    }

    /// Notify the DM that LLM processing has started
    pub fn notify_llm_processing<S: SessionManagementPort>(
        &self,
        session: &S,
        session_id: SessionId,
        action_id: &str,
    ) -> Result<(), PlayerActionError> {
        let message = BroadcastMessage {
            content: serde_json::json!({
                "type": "LLMProcessing",
                "action_id": action_id,
            }),
        };

        session
            .send_to_dm(session_id, &message)
            .map_err(|e| PlayerActionError::SessionError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here using mock implementations of the ports
}
