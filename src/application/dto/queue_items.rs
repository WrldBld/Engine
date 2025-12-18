//! Queue item types - Payloads for different queue types
//!
//! These types represent the data that flows through the queue system.
//! Each queue type has its own item type that implements Serialize/Deserialize.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{
    ApprovalDecision, GamePromptRequest, ProposedToolInfo, QueueItemId, SceneId, SessionId,
};

/// Player action waiting to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerActionItem {
    pub session_id: SessionId,
    pub player_id: String,
    /// The player character ID performing this action (for challenge targeting)
    #[serde(default)]
    pub pc_id: Option<crate::domain::value_objects::PlayerCharacterId>,
    pub action_type: String,
    pub target: Option<String>,
    pub dialogue: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// DM action waiting to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DMActionItem {
    pub session_id: SessionId,
    pub dm_id: String,
    pub action: DMAction,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DMAction {
    ApprovalDecision {
        request_id: String,
        decision: ApprovalDecision,
    },
    DirectNPCControl {
        npc_id: String,
        dialogue: String,
    },
    TriggerEvent {
        event_id: String,
    },
    TransitionScene {
        scene_id: SceneId,
    },
}

/// LLM request waiting to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequestItem {
    pub request_type: LLMRequestType,
    pub session_id: Option<SessionId>,
    /// World ID for routing suggestion responses (world-scoped events)
    #[serde(default)]
    pub world_id: Option<crate::domain::value_objects::WorldId>,
    /// The player character ID associated with this request (for challenge targeting)
    #[serde(default)]
    pub pc_id: Option<crate::domain::value_objects::PlayerCharacterId>,
    #[serde(default)]
    pub prompt: Option<GamePromptRequest>, // None for suggestions
    #[serde(default)]
    pub suggestion_context: Option<crate::application::services::SuggestionContext>, // Some for suggestions
    pub callback_id: String, // For routing response back
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LLMRequestType {
    NPCResponse { action_item_id: QueueItemId },
    Suggestion { field_type: String, entity_id: Option<String> },
    ChallengeReasoning { challenge_id: String },
}

/// Asset generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGenerationItem {
    pub session_id: Option<SessionId>,
    pub entity_type: String,
    pub entity_id: String,
    pub workflow_id: String,
    pub prompt: String,
    pub count: u32,
}

/// Decision awaiting DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalItem {
    pub session_id: SessionId,
    pub source_action_id: QueueItemId, // Links back to PlayerActionItem
    pub decision_type: DecisionType,
    pub urgency: DecisionUrgency,
    pub npc_name: String,
    pub proposed_dialogue: String,
    pub internal_reasoning: String,
    pub proposed_tools: Vec<ProposedToolInfo>,
    pub retry_count: u32,
    /// Optional challenge suggestion from LLM
    #[serde(default)]
    pub challenge_suggestion: Option<ChallengeSuggestionInfo>,
    /// Optional narrative event suggestion from LLM
    #[serde(default)]
    pub narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
}

/// Challenge suggestion information for DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestionInfo {
    pub challenge_id: String,
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub confidence: String,
    pub reasoning: String,
    /// Target player character ID for skill modifier lookup
    #[serde(default)]
    pub target_pc_id: Option<String>,
}

/// Enhanced challenge suggestion with detailed outcomes and tool receipts
///
/// This structure allows the LLM to suggest a skill challenge with
/// pre-defined outcomes for each result tier, including proposed tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedChallengeSuggestion {
    /// Optional reference to a predefined challenge (None for ad-hoc)
    pub challenge_id: Option<String>,
    /// Name of the challenge (e.g., "Persuasion Check", "Stealth Attempt")
    pub challenge_name: String,
    /// The skill being tested (e.g., "Persuasion", "Stealth", "Athletics")
    pub skill_name: String,
    /// Difficulty display (e.g., "DC 15", "Moderate", "70%")
    pub difficulty_display: String,
    /// What the NPC says before the challenge
    pub npc_reply: String,
    /// Detailed outcomes for each result tier
    pub outcomes: EnhancedOutcomes,
    /// Internal LLM reasoning (shown to DM only)
    pub reasoning: String,
}

/// Outcomes for each challenge result tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedOutcomes {
    /// Outcome for natural 20 or exceptional success (optional)
    #[serde(default)]
    pub critical_success: Option<OutcomeDetail>,
    /// Outcome for meeting or exceeding the DC
    pub success: OutcomeDetail,
    /// Outcome for failing to meet the DC
    pub failure: OutcomeDetail,
    /// Outcome for natural 1 or catastrophic failure (optional)
    #[serde(default)]
    pub critical_failure: Option<OutcomeDetail>,
}

/// Detailed outcome information including narrative and tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDetail {
    /// Narrative flavor text describing what happens
    pub flavor_text: String,
    /// Scene direction (what actions/changes occur)
    pub scene_direction: String,
    /// Tool calls that would be executed for this outcome
    #[serde(default)]
    pub proposed_tools: Vec<ProposedToolInfo>,
}

/// Narrative event suggestion information for DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventSuggestionInfo {
    pub event_id: String,
    pub event_name: String,
    pub description: String,
    pub scene_direction: String,
    pub confidence: String,
    pub reasoning: String,
    pub matched_triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    NPCResponse,
    ToolUsage,
    ChallengeSuggestion,
    SceneTransition,
    /// Challenge outcome pending DM approval (P3.3)
    ChallengeOutcome,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DecisionUrgency {
    Normal = 0,
    AwaitingPlayer = 1,
    SceneCritical = 2,
}

/// Challenge outcome awaiting DM approval (P3.3)
///
/// After a player rolls, the outcome is queued here for DM review.
/// The DM can accept, edit, or request LLM suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeOutcomeApprovalItem {
    /// Unique ID for this resolution
    pub resolution_id: String,
    /// Session where the challenge occurred
    pub session_id: SessionId,
    /// ID of the challenge
    pub challenge_id: String,
    /// Name of the challenge
    pub challenge_name: String,
    /// Description of the challenge (for LLM context)
    #[serde(default)]
    pub challenge_description: String,
    /// Name of the skill required for this challenge (for LLM context)
    #[serde(default)]
    pub skill_name: Option<String>,
    /// ID of the character who rolled
    pub character_id: String,
    /// Name of the character who rolled
    pub character_name: String,
    /// Raw die roll (before modifier)
    pub roll: i32,
    /// Character's skill modifier
    pub modifier: i32,
    /// Total result (roll + modifier)
    pub total: i32,
    /// Determined outcome type (e.g., "Success", "Critical Failure")
    pub outcome_type: String,
    /// The pre-defined outcome description
    pub outcome_description: String,
    /// Triggers that will execute when approved
    pub outcome_triggers: Vec<ProposedToolInfo>,
    /// Roll breakdown string
    #[serde(default)]
    pub roll_breakdown: Option<String>,
    /// When the roll was submitted
    pub timestamp: DateTime<Utc>,
    /// LLM-generated suggestions (if requested)
    #[serde(default)]
    pub suggestions: Option<Vec<String>>,
    /// Whether LLM is currently generating suggestions
    #[serde(default)]
    pub is_generating_suggestions: bool,
}
