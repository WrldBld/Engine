//! Helper functions for WebSocket queue integration
//!
//! These functions assist with building prompts and processing queue items
//! in the WebSocket handler and background workers.

use crate::application::services::{ChallengeServiceImpl, NarrativeEventServiceImpl};
use crate::domain::value_objects::{
    CharacterContext, GamePromptRequest, PlayerActionContext, SceneContext,
};
use crate::application::dto::PlayerActionItem;
use crate::infrastructure::session::SessionManager;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::application::ports::outbound::QueueError;

/// Build a GamePromptRequest from a PlayerActionItem using session context
pub async fn build_prompt_from_action(
    sessions: &Arc<RwLock<SessionManager>>,
    _challenge_service: &ChallengeServiceImpl,
    _narrative_event_service: &NarrativeEventServiceImpl,
    action: &PlayerActionItem,
) -> Result<GamePromptRequest, QueueError> {
    // Get session context
    let sessions_read = sessions.read().await;
    let session = sessions_read
        .get_session(action.session_id)
        .ok_or_else(|| QueueError::Backend("Session not found".to_string()))?;

    let world_snapshot = &session.world_snapshot;
    
    // Get current scene
    let current_scene = match &world_snapshot.current_scene_id {
        Some(scene_id_str) => {
            world_snapshot
                .scenes
                .iter()
                .find(|s| s.id.to_string() == *scene_id_str)
        }
        None => {
            tracing::warn!("No current scene set in world snapshot");
            world_snapshot.scenes.first()
        }
    };

    let current_scene = current_scene.ok_or_else(|| {
        QueueError::Backend("No scenes available in world snapshot".to_string())
    })?;

    // Get location
    let location = world_snapshot
        .locations
        .iter()
        .find(|l| l.id == current_scene.location_id);

    // Determine responding character
    let responding_character = if let Some(target_name) = &action.target {
        world_snapshot
            .characters
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(target_name))
    } else {
        current_scene
            .featured_characters
            .first()
            .and_then(|char_id| {
                world_snapshot.characters.iter().find(|c| c.id == *char_id)
            })
    };

    let responding_character = responding_character.ok_or_else(|| {
        QueueError::Backend("No responding character found".to_string())
    })?;

    // Build scene context
    let scene_context = SceneContext {
        scene_name: current_scene.name.clone(),
        location_name: location
            .map(|l| l.name.clone())
            .unwrap_or_else(|| "Unknown".to_string()),
        time_context: match &current_scene.time_context {
            crate::domain::entities::TimeContext::Unspecified => "Unspecified".to_string(),
            crate::domain::entities::TimeContext::TimeOfDay(tod) => format!("{:?}", tod),
            crate::domain::entities::TimeContext::During(s) => s.clone(),
            crate::domain::entities::TimeContext::Custom(s) => s.clone(),
        },
        present_characters: current_scene
            .featured_characters
            .iter()
            .filter_map(|char_id| {
                world_snapshot
                    .characters
                    .iter()
                    .find(|c| c.id == *char_id)
                    .map(|c| c.name.clone())
            })
            .collect(),
    };

    // Build character context
    let character_context = CharacterContext {
        name: responding_character.name.clone(),
        archetype: format!("{:?}", responding_character.current_archetype),
        current_mood: None, // Character mood tracking not yet implemented
        wants: responding_character
            .wants
            .iter()
            .map(|w| format!("{:?}", w))
            .collect(),
        relationship_to_player: None, // Relationship tracking not yet implemented
    };

    // Get directorial notes
    let directorial_notes = current_scene.directorial_notes.clone();

    // Get conversation history from session
    let conversation_history = session
        .get_recent_history(20) // Get last 20 turns for context
        .iter()
        .map(|turn| crate::application::services::llm_service::ConversationTurn {
            speaker: turn.speaker.clone(),
            text: turn.content.clone(),
        })
        .collect();

    // Get active challenges (simplified - would need world_id and scene_id)
    let active_challenges = vec![]; // TODO: Implement challenge lookup when challenge service supports it

    // Get active narrative events (simplified)
    let active_narrative_events = vec![]; // TODO: Implement narrative event lookup when service supports it

    // Build the prompt request
    Ok(GamePromptRequest {
        player_action: PlayerActionContext {
            action_type: action.action_type.clone(),
            target: action.target.clone(),
            dialogue: action.dialogue.clone(),
        },
        scene_context,
        directorial_notes,
        conversation_history,
        responding_character: character_context,
        active_challenges,
        active_narrative_events,
    })
}
