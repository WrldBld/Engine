//! LLM context types - Types for building LLM prompts from game state
//!
//! # Architectural Note
//!
//! These types intentionally include `serde::Serialize` and `serde::Deserialize`.
//! They are designed specifically for JSON serialization to LLM services.
//! Serialization is intrinsic to their purpose, not an infrastructure concern.
//!
//! See: plans/snazzy-zooming-hamming.md, Batch 7 Architectural Decision Record

use serde::{Deserialize, Serialize};

/// Request for generating an NPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePromptRequest {
    /// The player's action that triggered this response
    pub player_action: PlayerActionContext,
    /// Current scene information
    pub scene_context: SceneContext,
    /// Director's notes for guiding the AI response
    pub directorial_notes: String,
    /// Previous conversation turns for context
    pub conversation_history: Vec<ConversationTurn>,
    /// The NPC who is responding
    pub responding_character: CharacterContext,
    /// Active challenges that could be triggered
    pub active_challenges: Vec<ActiveChallengeContext>,
    /// Active narrative events that could be triggered
    pub active_narrative_events: Vec<ActiveNarrativeEventContext>,
}

/// Context about the player's action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerActionContext {
    /// Type of action: "speak", "examine", "use_item", etc.
    pub action_type: String,
    /// Target of the action (NPC name, object, etc.)
    pub target: Option<String>,
    /// Dialogue content if the action is speech
    pub dialogue: Option<String>,
}

/// Context about the current scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneContext {
    /// Name of the current scene
    pub scene_name: String,
    /// Name of the location
    pub location_name: String,
    /// Time of day / narrative time context
    pub time_context: String,
    /// Names of characters present in the scene
    pub present_characters: Vec<String>,
}

/// Context about the responding character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterContext {
    /// Character's name
    pub name: String,
    /// Character archetype / personality summary
    pub archetype: String,
    /// Current emotional state
    pub current_mood: Option<String>,
    /// Character's motivations and desires
    pub wants: Vec<String>,
    /// How this character relates to the player
    pub relationship_to_player: Option<String>,
}

/// A single turn in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Name of the speaker
    pub speaker: String,
    /// What was said
    pub text: String,
}

/// Context about an active challenge that may be triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveChallengeContext {
    /// Unique identifier for the challenge
    pub id: String,
    /// Display name of the challenge
    pub name: String,
    /// Full description of the challenge
    pub description: String,
    /// Name of the skill required
    pub skill_name: String,
    /// Human-readable difficulty display (e.g. "DC 15", "Hard")
    pub difficulty_display: String,
    /// Keywords/phrases that trigger this challenge
    pub trigger_hints: Vec<String>,
}

/// Context about an active narrative event that may be triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveNarrativeEventContext {
    /// Unique identifier for the narrative event
    pub id: String,
    /// Display name of the event
    pub name: String,
    /// Brief description of what this event represents
    pub description: String,
    /// Scene direction text to help DM narrate when triggered
    pub scene_direction: String,
    /// Keywords/phrases that indicate this event should trigger
    pub trigger_hints: Vec<String>,
    /// Names of NPCs featured in this event
    pub featured_npc_names: Vec<String>,
    /// Priority level (higher = more important)
    pub priority: i32,
}
