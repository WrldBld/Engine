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

    // =========================================================================
    // Challenge Outcome Approval Messages (P3.3)
    // =========================================================================

    /// DM approves/edits/requests suggestion for challenge outcome
    ChallengeOutcomeDecision {
        /// ID of the pending resolution
        resolution_id: String,
        /// The decision (accept, edit, or suggest)
        decision: ChallengeOutcomeDecisionData,
    },

    /// DM requests LLM to suggest alternative outcome descriptions
    RequestOutcomeSuggestion {
        /// ID of the pending resolution
        resolution_id: String,
        /// Optional guidance for the LLM
        guidance: Option<String>,
    },

    /// DM requests LLM to generate outcome branches
    RequestOutcomeBranches {
        /// ID of the pending resolution
        resolution_id: String,
        /// Optional guidance for branch generation
        guidance: Option<String>,
    },

    /// DM selects an outcome branch
    SelectOutcomeBranch {
        /// ID of the pending resolution
        resolution_id: String,
        /// ID of the selected branch
        branch_id: String,
        /// Optional edits to the branch description
        modified_description: Option<String>,
    },

    // =========================================================================
    // Phase 23D: NPC Location Sharing (Observations)
    // =========================================================================

    /// DM shares NPC location with player (creates HeardAbout observation)
    ShareNpcLocation {
        /// The PC who will receive this info
        pc_id: String,
        /// The NPC whose location is being shared
        npc_id: String,
        /// The location where the NPC is (claimed to be)
        location_id: String,
        /// The region within the location
        region_id: String,
        /// Optional notes (e.g., "The bartender told you...")
        notes: Option<String>,
    },

    // =========================================================================
    // Phase 23C: Navigation
    // =========================================================================

    /// Player selects a PC to play
    SelectPlayerCharacter {
        /// The PC ID to select
        pc_id: String,
    },

    /// Player moves to a different region within the same location
    MoveToRegion {
        /// The PC that is moving
        pc_id: String,
        /// The target region ID
        region_id: String,
    },

    /// Player exits to a different location
    ExitToLocation {
        /// The PC that is moving
        pc_id: String,
        /// The target location ID
        location_id: String,
        /// Optional specific arrival region (uses location default if not provided)
        arrival_region_id: Option<String>,
    },

    // =========================================================================
    // Phase 23E: DM Event System
    // =========================================================================

    /// DM triggers an NPC approach event (NPC approaches a player)
    TriggerApproachEvent {
        /// The NPC who is approaching
        npc_id: String,
        /// The PC being approached
        target_pc_id: String,
        /// Narrative description of the approach
        description: String,
    },

    /// DM triggers a location event (narration for all PCs in a region)
    TriggerLocationEvent {
        /// The region where the event happens
        region_id: String,
        /// Narrative description of the event
        description: String,
    },

    // =========================================================================
    // Phase 23F: Game Time Control
    // =========================================================================

    /// DM advances the in-game time
    AdvanceGameTime {
        /// Number of hours to advance
        hours: u32,
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

    // =========================================================================
    // Challenge Outcome Approval Messages (P3.3)
    // =========================================================================

    /// Challenge roll submitted, awaiting DM approval (sent to rolling player)
    ChallengeRollSubmitted {
        challenge_id: String,
        challenge_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        /// Determined outcome type (e.g., "Success", "Critical Failure")
        outcome_type: String,
        /// Status: "awaiting_dm_approval"
        status: String,
    },

    /// Pending challenge outcome for DM approval queue
    ChallengeOutcomePending {
        /// Unique ID for this pending resolution
        resolution_id: String,
        challenge_id: String,
        challenge_name: String,
        character_id: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        /// Determined outcome type (e.g., "Success", "Critical Failure")
        outcome_type: String,
        /// Pre-defined outcome description from challenge
        outcome_description: String,
        /// Triggers that will execute when approved
        outcome_triggers: Vec<ProposedToolInfo>,
        /// Roll breakdown string
        #[serde(default)]
        roll_breakdown: Option<String>,
    },

    /// LLM-generated outcome suggestions are ready (sent to DM)
    OutcomeSuggestionReady {
        /// Resolution ID this applies to
        resolution_id: String,
        /// Alternative outcome descriptions
        suggestions: Vec<String>,
    },

    /// LLM-generated outcome branches are ready for selection (sent to DM)
    OutcomeBranchesReady {
        /// Resolution ID this applies to
        resolution_id: String,
        /// Outcome tier (e.g., "Success", "Critical Failure")
        outcome_type: String,
        /// Available branches to choose from
        branches: Vec<OutcomeBranchData>,
    },

    // =========================================================================
    // Phase 23E: DM Event System
    // =========================================================================

    /// An NPC is approaching the player (sent to target PC)
    ApproachEvent {
        /// The NPC's ID
        npc_id: String,
        /// The NPC's name
        npc_name: String,
        /// The NPC's sprite asset (if any)
        npc_sprite: Option<String>,
        /// Narrative description of the approach
        description: String,
    },

    /// A location event occurred (sent to all PCs in region)
    LocationEvent {
        /// The region where the event occurred
        region_id: String,
        /// Narrative description of the event
        description: String,
    },

    /// NPC location was shared with the player (sent to target PC)
    NpcLocationShared {
        /// The NPC's ID
        npc_id: String,
        /// The NPC's name
        npc_name: String,
        /// The region name where NPC was seen/heard about
        region_name: String,
        /// Optional notes from the source
        notes: Option<String>,
    },

    // =========================================================================
    // Phase 23C: Navigation & Scene Updates
    // =========================================================================

    /// PC was selected for play
    PcSelected {
        /// The selected PC's ID
        pc_id: String,
        /// The PC's name
        pc_name: String,
        /// Current location ID
        location_id: String,
        /// Current region ID (if any)
        region_id: Option<String>,
    },

    /// Scene changed due to PC movement
    SceneChanged {
        /// The PC that moved
        pc_id: String,
        /// New region info
        region: RegionData,
        /// NPCs present in the new region
        npcs_present: Vec<NpcPresenceData>,
        /// Navigation options from this region
        navigation: NavigationData,
    },

    /// Movement was blocked (locked door, etc.)
    MovementBlocked {
        /// The PC that tried to move
        pc_id: String,
        /// Why movement was blocked
        reason: String,
    },

    // =========================================================================
    // Phase 23F: Game Time Control
    // =========================================================================

    /// Game time has been updated (broadcast to all)
    GameTimeUpdated {
        /// Display string (e.g., "Day 3, 2:30 PM")
        display: String,
        /// Time of day (Morning, Afternoon, Evening, Night)
        time_of_day: String,
        /// Whether time is paused
        is_paused: bool,
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

/// DM's decision on a challenge outcome (wire format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ChallengeOutcomeDecisionData {
    /// Accept the outcome as-is
    Accept,
    /// Edit the outcome description
    Edit {
        /// The modified outcome text
        modified_description: String,
    },
    /// Request LLM to suggest alternatives
    Suggest {
        /// Optional guidance for the LLM
        #[serde(default)]
        guidance: Option<String>,
    },
}

/// Outcome branch data for DM selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeBranchData {
    /// Unique identifier for this branch
    pub id: String,
    /// Short title/summary of this outcome
    pub title: String,
    /// Full narrative description
    pub description: String,
    /// Optional mechanical effects
    #[serde(default)]
    pub effects: Vec<String>,
}

// =============================================================================
// Phase 23C: Navigation Data Structures
// =============================================================================

/// Region data for scene display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionData {
    /// Region ID
    pub id: String,
    /// Region name
    pub name: String,
    /// Parent location ID
    pub location_id: String,
    /// Parent location name
    pub location_name: String,
    /// Backdrop image asset
    pub backdrop_asset: Option<String>,
    /// Atmosphere description
    pub atmosphere: Option<String>,
}

/// NPC presence data for scene display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcPresenceData {
    /// Character ID
    pub character_id: String,
    /// Character name
    pub name: String,
    /// Sprite asset for display
    pub sprite_asset: Option<String>,
    /// Portrait asset
    pub portrait_asset: Option<String>,
}

/// Navigation options from current region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationData {
    /// Connected regions within same location
    pub connected_regions: Vec<NavigationTarget>,
    /// Exits to other locations
    pub exits: Vec<NavigationExit>,
}

/// A navigation target (region within same location)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationTarget {
    /// Region ID
    pub region_id: String,
    /// Region name
    pub name: String,
    /// Whether this path is locked
    pub is_locked: bool,
    /// Description of why it's locked (if applicable)
    pub lock_description: Option<String>,
}

/// An exit to another location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationExit {
    /// Target location ID
    pub location_id: String,
    /// Target location name
    pub location_name: String,
    /// Arrival region in target location
    pub arrival_region_id: String,
    /// Description of the exit
    pub description: Option<String>,
}
