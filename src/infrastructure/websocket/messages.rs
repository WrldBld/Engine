//! WebSocket protocol messages (Engine ↔ Player)
//!
//! This module defines the wire-format DTOs for WebSocket communication:
//! `ClientMessage`, `ServerMessage`, and their supporting structs. It is
//! intentionally free of business logic so that the transport layer can
//! remain thin and stable.

use serde::{Deserialize, Serialize};

use crate::application::dto::{ChallengeSuggestionInfo, NarrativeEventSuggestionInfo};
use crate::domain::value_objects::{ApprovalDecision, ProposedToolInfo};

/// Messages from client (Player) to server (Engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Join a game session
    JoinSession {
        user_id: String,
        role: ParticipantRole,
        /// Optional world ID to join (creates demo session if not provided)
        #[serde(default)]
        world_id: Option<String>,
    },
    /// Player performs an action
    PlayerAction {
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    },
    /// Request to change scene
    RequestSceneChange { scene_id: String },
    /// DM updates directorial context
    DirectorialUpdate { context: DirectorialContext },
    /// DM approves/rejects LLM response
    ApprovalDecision {
        request_id: String,
        decision: ApprovalDecision,
    },
    /// Player submits a challenge roll (legacy - accepts raw roll value)
    ChallengeRoll {
        challenge_id: String,
        roll: i32,
    },
    /// Player submits a challenge roll with dice input (formula or manual)
    ChallengeRollInput {
        challenge_id: String,
        /// Dice input - either "formula" with dice string, or "manual" with result
        input_type: DiceInputType,
    },
    /// DM triggers a challenge manually
    TriggerChallenge {
        challenge_id: String,
        target_character_id: String,
    },
    /// DM approves/rejects/modifies a suggested challenge
    ChallengeSuggestionDecision {
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    },
    /// DM approves/rejects a suggested narrative event trigger
    NarrativeEventSuggestionDecision {
        request_id: String,
        event_id: String,
        approved: bool,
        /// Optional selected outcome if DM pre-selects an outcome
        selected_outcome: Option<String>,
    },
    /// Heartbeat ping
    Heartbeat,

    /// DM requests regeneration of challenge outcome(s)
    RegenerateOutcome {
        /// The approval request ID this relates to
        request_id: String,
        /// Which outcome to regenerate ("success", "failure", "critical_success", "critical_failure")
        /// If None, regenerate all outcomes
        outcome_type: Option<String>,
        /// Optional guidance for regeneration
        guidance: Option<String>,
    },

    /// DM discards a challenge suggestion
    DiscardChallenge {
        /// The approval request ID containing the challenge
        request_id: String,
        /// Feedback on why discarding (optional, for LLM learning)
        feedback: Option<String>,
    },

    /// DM creates an ad-hoc challenge (no LLM involved)
    CreateAdHocChallenge {
        /// Name of the challenge
        challenge_name: String,
        /// Skill being tested
        skill_name: String,
        /// Difficulty display (e.g., "DC 15", "Hard")
        difficulty: String,
        /// Target PC ID
        target_pc_id: String,
        /// Outcome descriptions
        outcomes: AdHocOutcomes,
    },
}

/// Messages from server (Engine) to client (Player)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Session successfully joined with full details
    SessionJoined {
        session_id: String,
        role: ParticipantRole,
        participants: Vec<ParticipantInfo>,
        world_snapshot: serde_json::Value,
    },
    /// A player joined the session (broadcast to others)
    PlayerJoined {
        user_id: String,
        role: ParticipantRole,
        character_name: Option<String>,
    },
    /// A player left the session (broadcast to others)
    PlayerLeft { user_id: String },
    /// Player action was received and is being processed
    ActionReceived {
        action_id: String,
        player_id: String,
        action_type: String,
    },
    /// Scene update
    SceneUpdate {
        scene: SceneData,
        characters: Vec<CharacterData>,
        interactions: Vec<InteractionData>,
    },
    /// NPC dialogue response
    DialogueResponse {
        speaker_id: String,
        speaker_name: String,
        text: String,
        choices: Vec<DialogueChoice>,
    },
    /// LLM is processing (shown to DM)
    LLMProcessing { action_id: String },
    /// Action queued for processing
    ActionQueued {
        action_id: String,
        player_name: String,
        action_type: String,
        queue_depth: usize,
    },
    /// Queue status update (sent to DM)
    QueueStatus {
        player_actions_pending: usize,
        llm_requests_pending: usize,
        llm_requests_processing: usize,
        approvals_pending: usize,
    },
    /// Approval required (sent to DM)
    ApprovalRequired {
        request_id: String,
        npc_name: String,
        proposed_dialogue: String,
        internal_reasoning: String,
        proposed_tools: Vec<ProposedToolInfo>,
        challenge_suggestion: Option<ChallengeSuggestionInfo>,
        narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
    },
    /// Response was approved and executed
    ResponseApproved {
        npc_dialogue: String,
        executed_tools: Vec<String>,
    },
    /// Challenge prompt sent to player
    ChallengePrompt {
        challenge_id: String,
        challenge_name: String,
        skill_name: String,
        difficulty_display: String,
        description: String,
        character_modifier: i32,
        /// Suggested dice formula based on rule system (e.g., "1d20", "1d100", "2d6")
        #[serde(default)]
        suggested_dice: Option<String>,
        /// Human-readable hint about the rule system (e.g., "Roll d20, add your Persuasion modifier")
        #[serde(default)]
        rule_system_hint: Option<String>,
    },
    /// Challenge result broadcast to all
    ChallengeResolved {
        challenge_id: String,
        challenge_name: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome: String,
        outcome_description: String,
        /// Roll breakdown string (e.g., "1d20(14) + 3 = 17" or "Manual: 18")
        #[serde(default)]
        roll_breakdown: Option<String>,
        /// Individual dice results if rolled with formula
        #[serde(default)]
        individual_rolls: Option<Vec<i32>>,
    },
    /// Narrative event has been triggered
    NarrativeEventTriggered {
        event_id: String,
        event_name: String,
        outcome_description: String,
        scene_direction: String,
    },
    /// Party is split across multiple locations (sent to DM)
    SplitPartyNotification {
        location_count: usize,
        locations: Vec<SplitPartyLocation>,
    },
    /// Error message
    Error { code: String, message: String },
    /// Heartbeat response
    Pong,

    // Generation events (for Creator Mode)
    /// A generation batch has been queued
    GenerationQueued {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        position: u32,
    },
    /// Generation progress update
    GenerationProgress { batch_id: String, progress: u8 },
    /// Generation batch completed
    GenerationComplete {
        batch_id: String,
        asset_count: u32,
    },
    /// Generation batch failed
    GenerationFailed { batch_id: String, error: String },
    /// A suggestion request has been queued
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
    },
    /// A suggestion request is being processed
    SuggestionProgress {
        request_id: String,
        status: String,
    },
    /// A suggestion request has completed
    SuggestionComplete {
        request_id: String,
        suggestions: Vec<String>,
    },
    /// A suggestion request has failed
    SuggestionFailed {
        request_id: String,
        error: String,
    },
    /// ComfyUI connection state changed
    ComfyUIStateChanged {
        state: String, // "connected", "degraded", "disconnected", "circuit_open"
        message: Option<String>, // Human-readable status
        retry_in_seconds: Option<u32>, // Countdown for reconnect
    },

    /// Outcome has been regenerated (sent to DM)
    OutcomeRegenerated {
        /// The approval request ID this relates to
        request_id: String,
        /// Which outcome was regenerated
        outcome_type: String,
        /// New outcome details
        new_outcome: OutcomeDetailData,
    },

    /// Challenge was discarded (confirmation to DM)
    ChallengeDiscarded {
        request_id: String,
    },

    /// Ad-hoc challenge created and sent to player
    AdHocChallengeCreated {
        challenge_id: String,
        challenge_name: String,
        target_pc_id: String,
    },
}

/// Information about a session participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub user_id: String,
    pub role: ParticipantRole,
    pub character_name: Option<String>,
}

/// Participant role in the session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRole {
    DungeonMaster,
    Player,
    Spectator,
}

/// Scene data from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneData {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub location_name: String,
    pub backdrop_asset: Option<String>,
    pub time_context: String,
    pub directorial_notes: String,
}

/// Character data for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    pub id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub position: CharacterPosition,
    pub is_speaking: bool,
}

/// Character position on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterPosition {
    Left,
    Center,
    Right,
    OffScreen,
}

/// Available interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionData {
    pub id: String,
    pub name: String,
    pub interaction_type: String,
    pub target_name: Option<String>,
    pub is_available: bool,
}

/// Dialogue choice for player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    pub id: String,
    pub text: String,
    pub is_custom_input: bool,
}

/// Location information for split party notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitPartyLocation {
    pub location_id: String,
    pub location_name: String,
    pub pc_count: usize,
    pub pc_names: Vec<String>,
}

/// Directorial context from DM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorialContext {
    pub scene_notes: String,
    pub tone: String,
    pub npc_motivations: Vec<NpcMotivationData>,
    pub forbidden_topics: Vec<String>,
}

/// NPC motivation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMotivationData {
    pub character_id: String,
    pub mood: String,
    pub immediate_goal: String,
    pub secret_agenda: Option<String>,
}

/// Dice input type for challenge rolls
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DiceInputType {
    /// Roll dice using a formula string like "1d20+5"
    Formula(String),
    /// Use a manual result (physical dice roll)
    Manual(i32),
}

/// Ad-hoc challenge outcomes for DM-created challenges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdHocOutcomes {
    /// What happens on success
    pub success: String,
    /// What happens on failure
    pub failure: String,
    /// Optional critical success outcome
    #[serde(default)]
    pub critical_success: Option<String>,
    /// Optional critical failure outcome
    #[serde(default)]
    pub critical_failure: Option<String>,
}

/// Outcome detail data for regenerated outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDetailData {
    /// Narrative flavor text
    pub flavor_text: String,
    /// Scene direction (what happens)
    pub scene_direction: String,
    /// Proposed tool calls for this outcome
    #[serde(default)]
    pub proposed_tools: Vec<ProposedToolInfo>,
}


