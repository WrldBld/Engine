//! Helper functions for WebSocket queue integration
//!
//! These functions assist with building prompts and processing queue items
//! in the WebSocket handler and background workers.

use crate::application::dto::PlayerActionItem;
use crate::application::ports::outbound::QueueError;
use crate::application::services::{
    ChallengeService, ChallengeServiceImpl, NarrativeEventService, NarrativeEventServiceImpl,
};
use crate::domain::value_objects::{
    ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext, ConversationTurn,
    GamePromptRequest, PlayerActionContext, SceneContext,
};
use crate::infrastructure::session::SessionManager;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Build a GamePromptRequest from a PlayerActionItem using session context
pub async fn build_prompt_from_action(
    sessions: &Arc<RwLock<SessionManager>>,
    challenge_service: &Arc<ChallengeServiceImpl>,
    narrative_event_service: &Arc<NarrativeEventServiceImpl>,
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
        .map(|turn| ConversationTurn {
            speaker: turn.speaker.clone(),
            text: turn.content.clone(),
        })
        .collect();

    // Extract world_id from the session's world snapshot
    let world_id = world_snapshot.world.id;

    // Query active challenges and convert to ActiveChallengeContext
    let active_challenges: Vec<ActiveChallengeContext> = match challenge_service
        .list_active(world_id)
        .await
    {
        Ok(challenges) => challenges
            .into_iter()
            .map(|c| ActiveChallengeContext {
                id: c.id.to_string(),
                name: c.name,
                skill_name: c.skill_id.to_string(), // TODO: Look up actual skill name from skill service
                difficulty_display: c.difficulty.display(),
                description: c.description,
                trigger_hints: c
                    .trigger_conditions
                    .iter()
                    .map(|t| t.description.clone())
                    .collect(),
            })
            .collect(),
        Err(e) => {
            tracing::warn!("Failed to load active challenges: {}", e);
            vec![]
        }
    };

    // Query active narrative events and convert to ActiveNarrativeEventContext
    let active_narrative_events: Vec<ActiveNarrativeEventContext> = match narrative_event_service
        .list_active(world_id)
        .await
    {
        Ok(events) => events
            .into_iter()
            .map(|e| ActiveNarrativeEventContext {
                id: e.id.to_string(),
                name: e.name,
                description: e.description,
                scene_direction: e.scene_direction,
                priority: e.priority,
                trigger_hints: e
                    .trigger_conditions
                    .iter()
                    .map(|t| t.description.clone())
                    .collect(),
                featured_npc_names: e
                    .featured_npcs
                    .iter()
                    .filter_map(|npc_id| {
                        world_snapshot
                            .characters
                            .iter()
                            .find(|c| c.id == *npc_id)
                            .map(|c| c.name.clone())
                    })
                    .collect(),
            })
            .collect(),
        Err(e) => {
            tracing::warn!("Failed to load active narrative events: {}", e);
            vec![]
        }
    };

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
