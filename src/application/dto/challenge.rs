use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::{
    Challenge, ChallengeOutcomes, ChallengeType, Difficulty, DifficultyDescriptor, Outcome,
    OutcomeTrigger, TriggerCondition, TriggerType,
};
use crate::domain::value_objects::{ChallengeId, SceneId};

// ============================================================================
// DTO enums + mapping
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTypeDto {
    SkillCheck,
    AbilityCheck,
    SavingThrow,
    OpposedCheck,
    ComplexChallenge,
}

impl Default for ChallengeTypeDto {
    fn default() -> Self {
        Self::SkillCheck
    }
}

impl From<ChallengeTypeDto> for ChallengeType {
    fn from(value: ChallengeTypeDto) -> Self {
        match value {
            ChallengeTypeDto::SkillCheck => ChallengeType::SkillCheck,
            ChallengeTypeDto::AbilityCheck => ChallengeType::AbilityCheck,
            ChallengeTypeDto::SavingThrow => ChallengeType::SavingThrow,
            ChallengeTypeDto::OpposedCheck => ChallengeType::OpposedCheck,
            ChallengeTypeDto::ComplexChallenge => ChallengeType::ComplexChallenge,
        }
    }
}

impl From<ChallengeType> for ChallengeTypeDto {
    fn from(value: ChallengeType) -> Self {
        match value {
            ChallengeType::SkillCheck => ChallengeTypeDto::SkillCheck,
            ChallengeType::AbilityCheck => ChallengeTypeDto::AbilityCheck,
            ChallengeType::SavingThrow => ChallengeTypeDto::SavingThrow,
            ChallengeType::OpposedCheck => ChallengeTypeDto::OpposedCheck,
            ChallengeType::ComplexChallenge => ChallengeTypeDto::ComplexChallenge,
        }
    }
}

// ============================================================================
// Request/Response DTOs (moved from HTTP layer)
// ============================================================================

/// Request to create a challenge
#[derive(Debug, Deserialize)]
pub struct CreateChallengeRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub skill_id: String,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub challenge_type: ChallengeTypeDto,
    pub difficulty: DifficultyRequestDto,
    #[serde(default)]
    pub outcomes: OutcomesRequestDto,
    #[serde(default)]
    pub trigger_conditions: Vec<TriggerConditionRequestDto>,
    #[serde(default)]
    pub prerequisite_challenges: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Request to update a challenge
#[derive(Debug, Deserialize)]
pub struct UpdateChallengeRequestDto {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub skill_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub challenge_type: Option<ChallengeTypeDto>,
    #[serde(default)]
    pub difficulty: Option<DifficultyRequestDto>,
    #[serde(default)]
    pub outcomes: Option<OutcomesRequestDto>,
    #[serde(default)]
    pub trigger_conditions: Option<Vec<TriggerConditionRequestDto>>,
    #[serde(default)]
    pub prerequisite_challenges: Option<Vec<String>>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub order: Option<u32>,
    #[serde(default)]
    pub is_favorite: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Difficulty request variants
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DifficultyRequestDto {
    Dc { value: u32 },
    Percentage { value: u32 },
    Descriptor { value: String },
    Opposed,
    Custom { value: String },
}

impl From<DifficultyRequestDto> for Difficulty {
    fn from(req: DifficultyRequestDto) -> Self {
        match req {
            DifficultyRequestDto::Dc { value } => Difficulty::DC(value),
            DifficultyRequestDto::Percentage { value } => Difficulty::Percentage(value),
            DifficultyRequestDto::Descriptor { value } => {
                let descriptor = match value.to_lowercase().as_str() {
                    "trivial" => DifficultyDescriptor::Trivial,
                    "easy" => DifficultyDescriptor::Easy,
                    "routine" => DifficultyDescriptor::Routine,
                    "moderate" => DifficultyDescriptor::Moderate,
                    "challenging" => DifficultyDescriptor::Challenging,
                    "hard" => DifficultyDescriptor::Hard,
                    "very_hard" | "veryhard" => DifficultyDescriptor::VeryHard,
                    "extreme" => DifficultyDescriptor::Extreme,
                    "impossible" => DifficultyDescriptor::Impossible,
                    "risky" => DifficultyDescriptor::Risky,
                    "desperate" => DifficultyDescriptor::Desperate,
                    _ => DifficultyDescriptor::Moderate,
                };
                Difficulty::Descriptor(descriptor)
            }
            DifficultyRequestDto::Opposed => Difficulty::Opposed,
            DifficultyRequestDto::Custom { value } => Difficulty::Custom(value),
        }
    }
}

impl From<Difficulty> for DifficultyRequestDto {
    fn from(d: Difficulty) -> Self {
        match d {
            Difficulty::DC(v) => DifficultyRequestDto::Dc { value: v },
            Difficulty::Percentage(v) => DifficultyRequestDto::Percentage { value: v },
            Difficulty::Descriptor(d) => DifficultyRequestDto::Descriptor {
                value: format!("{:?}", d).to_lowercase(),
            },
            Difficulty::Opposed => DifficultyRequestDto::Opposed,
            Difficulty::Custom(s) => DifficultyRequestDto::Custom { value: s },
        }
    }
}

/// Outcomes request
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OutcomesRequestDto {
    pub success: OutcomeRequestDto,
    pub failure: OutcomeRequestDto,
    #[serde(default)]
    pub partial: Option<OutcomeRequestDto>,
    #[serde(default)]
    pub critical_success: Option<OutcomeRequestDto>,
    #[serde(default)]
    pub critical_failure: Option<OutcomeRequestDto>,
}

impl From<OutcomesRequestDto> for ChallengeOutcomes {
    fn from(req: OutcomesRequestDto) -> Self {
        Self {
            success: req.success.into(),
            failure: req.failure.into(),
            partial: req.partial.map(Into::into),
            critical_success: req.critical_success.map(Into::into),
            critical_failure: req.critical_failure.map(Into::into),
        }
    }
}

impl From<ChallengeOutcomes> for OutcomesRequestDto {
    fn from(o: ChallengeOutcomes) -> Self {
        Self {
            success: o.success.into(),
            failure: o.failure.into(),
            partial: o.partial.map(Into::into),
            critical_success: o.critical_success.map(Into::into),
            critical_failure: o.critical_failure.map(Into::into),
        }
    }
}

/// Single outcome request
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OutcomeRequestDto {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub triggers: Vec<OutcomeTriggerRequestDto>,
}

impl From<OutcomeRequestDto> for Outcome {
    fn from(req: OutcomeRequestDto) -> Self {
        Self {
            description: req.description,
            triggers: req.triggers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Outcome> for OutcomeRequestDto {
    fn from(o: Outcome) -> Self {
        Self {
            description: o.description,
            triggers: o.triggers.into_iter().map(Into::into).collect(),
        }
    }
}

/// Outcome trigger request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutcomeTriggerRequestDto {
    RevealInformation { info: String, persist: bool },
    EnableChallenge { challenge_id: String },
    DisableChallenge { challenge_id: String },
    ModifyCharacterStat { stat: String, modifier: i32 },
    TriggerScene { scene_id: String },
    GiveItem { item_name: String, item_description: Option<String> },
    Custom { description: String },
}

impl From<OutcomeTriggerRequestDto> for OutcomeTrigger {
    fn from(req: OutcomeTriggerRequestDto) -> Self {
        match req {
            OutcomeTriggerRequestDto::RevealInformation { info, persist } => {
                OutcomeTrigger::RevealInformation { info, persist }
            }
            OutcomeTriggerRequestDto::EnableChallenge { challenge_id } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                OutcomeTrigger::EnableChallenge { challenge_id: id }
            }
            OutcomeTriggerRequestDto::DisableChallenge { challenge_id } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                OutcomeTrigger::DisableChallenge { challenge_id: id }
            }
            OutcomeTriggerRequestDto::ModifyCharacterStat { stat, modifier } => {
                OutcomeTrigger::ModifyCharacterStat { stat, modifier }
            }
            OutcomeTriggerRequestDto::TriggerScene { scene_id } => {
                let id = Uuid::parse_str(&scene_id)
                    .map(SceneId::from_uuid)
                    .unwrap_or_else(|_| SceneId::new());
                OutcomeTrigger::TriggerScene { scene_id: id }
            }
            OutcomeTriggerRequestDto::GiveItem {
                item_name,
                item_description,
            } => OutcomeTrigger::GiveItem {
                item_name,
                item_description,
            },
            OutcomeTriggerRequestDto::Custom { description } => {
                OutcomeTrigger::Custom { description }
            }
        }
    }
}

impl From<OutcomeTrigger> for OutcomeTriggerRequestDto {
    fn from(t: OutcomeTrigger) -> Self {
        match t {
            OutcomeTrigger::RevealInformation { info, persist } => {
                OutcomeTriggerRequestDto::RevealInformation { info, persist }
            }
            OutcomeTrigger::EnableChallenge { challenge_id } => {
                OutcomeTriggerRequestDto::EnableChallenge {
                    challenge_id: challenge_id.to_string(),
                }
            }
            OutcomeTrigger::DisableChallenge { challenge_id } => {
                OutcomeTriggerRequestDto::DisableChallenge {
                    challenge_id: challenge_id.to_string(),
                }
            }
            OutcomeTrigger::ModifyCharacterStat { stat, modifier } => {
                OutcomeTriggerRequestDto::ModifyCharacterStat { stat, modifier }
            }
            OutcomeTrigger::TriggerScene { scene_id } => OutcomeTriggerRequestDto::TriggerScene {
                scene_id: scene_id.to_string(),
            },
            OutcomeTrigger::GiveItem {
                item_name,
                item_description,
            } => OutcomeTriggerRequestDto::GiveItem {
                item_name,
                item_description,
            },
            OutcomeTrigger::Custom { description } => OutcomeTriggerRequestDto::Custom { description },
        }
    }
}

/// Trigger condition request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TriggerConditionRequestDto {
    pub condition_type: TriggerTypeRequestDto,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

impl From<TriggerConditionRequestDto> for TriggerCondition {
    fn from(req: TriggerConditionRequestDto) -> Self {
        let mut tc = TriggerCondition::new(req.condition_type.into(), req.description);
        if req.required {
            tc = tc.required();
        }
        tc
    }
}

impl From<TriggerCondition> for TriggerConditionRequestDto {
    fn from(tc: TriggerCondition) -> Self {
        Self {
            condition_type: tc.condition_type.into(),
            description: tc.description,
            required: tc.required,
        }
    }
}

/// Trigger type request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerTypeRequestDto {
    ObjectInteraction { keywords: Vec<String> },
    EnterArea { keywords: Vec<String> },
    DialogueTopic { keywords: Vec<String> },
    ChallengeComplete {
        challenge_id: String,
        requires_success: Option<bool>,
    },
    TimeBased { turns: u32 },
    NpcPresent { keywords: Vec<String> },
    Custom { description: String },
}

impl From<TriggerTypeRequestDto> for TriggerType {
    fn from(req: TriggerTypeRequestDto) -> Self {
        match req {
            TriggerTypeRequestDto::ObjectInteraction { keywords } => {
                TriggerType::ObjectInteraction { keywords }
            }
            TriggerTypeRequestDto::EnterArea { keywords } => {
                TriggerType::EnterArea { area_keywords: keywords }
            }
            TriggerTypeRequestDto::DialogueTopic { keywords } => {
                TriggerType::DialogueTopic { topic_keywords: keywords }
            }
            TriggerTypeRequestDto::ChallengeComplete {
                challenge_id,
                requires_success,
            } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                TriggerType::ChallengeComplete {
                    challenge_id: id,
                    requires_success,
                }
            }
            TriggerTypeRequestDto::TimeBased { turns } => TriggerType::TimeBased { turns },
            TriggerTypeRequestDto::NpcPresent { keywords } => {
                TriggerType::NpcPresent { npc_keywords: keywords }
            }
            TriggerTypeRequestDto::Custom { description } => TriggerType::Custom { description },
        }
    }
}

impl From<TriggerType> for TriggerTypeRequestDto {
    fn from(t: TriggerType) -> Self {
        match t {
            TriggerType::ObjectInteraction { keywords } => {
                TriggerTypeRequestDto::ObjectInteraction { keywords }
            }
            TriggerType::EnterArea { area_keywords } => {
                TriggerTypeRequestDto::EnterArea { keywords: area_keywords }
            }
            TriggerType::DialogueTopic { topic_keywords } => {
                TriggerTypeRequestDto::DialogueTopic { keywords: topic_keywords }
            }
            TriggerType::ChallengeComplete {
                challenge_id,
                requires_success,
            } => TriggerTypeRequestDto::ChallengeComplete {
                challenge_id: challenge_id.to_string(),
                requires_success,
            },
            TriggerType::TimeBased { turns } => TriggerTypeRequestDto::TimeBased { turns },
            TriggerType::NpcPresent { npc_keywords } => {
                TriggerTypeRequestDto::NpcPresent { keywords: npc_keywords }
            }
            TriggerType::Custom { description } => TriggerTypeRequestDto::Custom { description },
        }
    }
}

/// Challenge response
#[derive(Debug, Serialize)]
pub struct ChallengeResponseDto {
    pub id: String,
    pub world_id: String,
    /// Scene this challenge is tied to (from TIED_TO_SCENE edge)
    pub scene_id: Option<String>,
    pub name: String,
    pub description: String,
    pub challenge_type: ChallengeTypeDto,
    /// Skill required for this challenge (from REQUIRES_SKILL edge)
    pub skill_id: Option<String>,
    pub difficulty: DifficultyRequestDto,
    pub outcomes: OutcomesRequestDto,
    pub trigger_conditions: Vec<TriggerConditionRequestDto>,
    /// Prerequisite challenges (from REQUIRES_COMPLETION_OF edges)
    pub prerequisite_challenges: Vec<String>,
    pub active: bool,
    pub order: u32,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

impl ChallengeResponseDto {
    /// Create a response DTO from a Challenge and its edge data
    pub fn from_challenge_with_edges(
        challenge: Challenge,
        skill_id: Option<String>,
        scene_id: Option<String>,
        prerequisite_ids: Vec<String>,
    ) -> Self {
        Self {
            id: challenge.id.to_string(),
            world_id: challenge.world_id.to_string(),
            scene_id,
            name: challenge.name,
            description: challenge.description,
            challenge_type: challenge.challenge_type.into(),
            skill_id,
            difficulty: challenge.difficulty.into(),
            outcomes: challenge.outcomes.into(),
            trigger_conditions: challenge.trigger_conditions.into_iter().map(Into::into).collect(),
            prerequisite_challenges: prerequisite_ids,
            active: challenge.active,
            order: challenge.order,
            is_favorite: challenge.is_favorite,
            tags: challenge.tags,
        }
    }

    /// Create a minimal response without edge data (for list views where edge data isn't needed)
    pub fn from_challenge_minimal(challenge: Challenge) -> Self {
        Self {
            id: challenge.id.to_string(),
            world_id: challenge.world_id.to_string(),
            scene_id: None,
            name: challenge.name,
            description: challenge.description,
            challenge_type: challenge.challenge_type.into(),
            skill_id: None,
            difficulty: challenge.difficulty.into(),
            outcomes: challenge.outcomes.into(),
            trigger_conditions: challenge.trigger_conditions.into_iter().map(Into::into).collect(),
            prerequisite_challenges: Vec::new(),
            active: challenge.active,
            order: challenge.order,
            is_favorite: challenge.is_favorite,
            tags: challenge.tags,
        }
    }
}

// ============================================================================
// Ad-hoc Challenge DTOs
// ============================================================================

/// Ad-hoc challenge outcomes provided by the DM (simple string descriptions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdHocOutcomesDto {
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

// ============================================================================
// Challenge Outcome Approval DTOs (P3.3)
// ============================================================================

/// Pending challenge resolution awaiting DM approval
///
/// After a player rolls, this structure holds the resolution details
/// until the DM approves, edits, or requests an alternative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChallengeResolutionDto {
    /// Unique ID for this resolution (for tracking in approval queue)
    pub resolution_id: String,
    /// ID of the challenge being resolved
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
    /// The pre-defined outcome description from the challenge
    pub outcome_description: String,
    /// Triggers that will execute when this outcome is applied
    pub outcome_triggers: Vec<OutcomeTriggerRequestDto>,
    /// Roll breakdown string (e.g., "1d20(15) + 3 = 18")
    #[serde(default)]
    pub roll_breakdown: Option<String>,
    /// Individual die rolls
    #[serde(default)]
    pub individual_rolls: Option<Vec<i32>>,
    /// When the roll was submitted
    pub timestamp: String,
}

/// DM's decision on a challenge outcome
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ChallengeOutcomeDecision {
    /// Accept the outcome as-is
    Accept,
    /// Edit the outcome description
    Edit {
        /// The modified outcome text
        modified_description: String,
    },
    /// Request LLM to suggest alternative outcomes
    Suggest {
        /// Optional guidance for the LLM
        #[serde(default)]
        guidance: Option<String>,
    },
}

/// Request to approve/modify a challenge outcome
#[derive(Debug, Clone, Deserialize)]
pub struct ChallengeOutcomeApprovalRequest {
    /// ID of the pending resolution
    pub resolution_id: String,
    /// The DM's decision
    #[serde(flatten)]
    pub decision: ChallengeOutcomeDecision,
}

/// Request for LLM outcome suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeSuggestionRequest {
    /// Challenge ID (for context lookup)
    pub challenge_id: String,
    /// Name of the challenge
    pub challenge_name: String,
    /// Description of the challenge situation
    pub challenge_description: String,
    /// Skill being tested
    pub skill_name: String,
    /// Outcome tier to generate suggestions for
    pub outcome_type: String,
    /// Roll context (e.g., "rolled 15 + 3 = 18 vs DC 15")
    pub roll_context: String,
    /// Optional DM guidance for generation
    #[serde(default)]
    pub guidance: Option<String>,
    /// Narrative context for continuity
    #[serde(default)]
    pub narrative_context: Option<String>,
}

/// Response containing LLM-generated outcome suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeSuggestionResponse {
    /// Resolution ID this applies to
    pub resolution_id: String,
    /// Alternative outcome descriptions
    pub suggestions: Vec<String>,
}

// ============================================================================
// Challenge Notification DTOs (for application layer → infrastructure)
// ============================================================================

/// Notification that a challenge has been resolved
///
/// This DTO is serialized and sent via the session port. The infrastructure
/// layer can map this to its own message format (e.g., WebSocket ServerMessage).
#[derive(Debug, Clone, Serialize)]
pub struct ChallengeResolvedNotification {
    /// Message type discriminator for WebSocket routing
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub challenge_id: String,
    pub challenge_name: String,
    pub character_name: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub outcome: String,
    pub outcome_description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roll_breakdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub individual_rolls: Option<Vec<i32>>,
}

impl ChallengeResolvedNotification {
    pub fn new(
        challenge_id: String,
        challenge_name: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome: String,
        outcome_description: String,
        roll_breakdown: Option<String>,
        individual_rolls: Option<Vec<i32>>,
    ) -> Self {
        Self {
            message_type: "ChallengeResolved",
            challenge_id,
            challenge_name,
            character_name,
            roll,
            modifier,
            total,
            outcome,
            outcome_description,
            roll_breakdown,
            individual_rolls,
        }
    }
}

/// Notification that a challenge roll was submitted and is awaiting DM approval
#[derive(Debug, Clone, Serialize)]
pub struct ChallengeRollSubmittedNotification {
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub challenge_id: String,
    pub challenge_name: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub outcome_type: String,
    pub status: String,
}

impl ChallengeRollSubmittedNotification {
    pub fn new(
        challenge_id: String,
        challenge_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
    ) -> Self {
        Self {
            message_type: "ChallengeRollSubmitted",
            challenge_id,
            challenge_name,
            roll,
            modifier,
            total,
            outcome_type,
            status: "awaiting_dm_approval".to_string(),
        }
    }
}

/// Notification for DM that a challenge outcome is pending approval
#[derive(Debug, Clone, Serialize)]
pub struct ChallengeOutcomePendingNotification {
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub resolution_id: String,
    pub challenge_id: String,
    pub challenge_name: String,
    pub character_id: String,
    pub character_name: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub outcome_type: String,
    pub outcome_description: String,
    pub outcome_triggers: Vec<crate::domain::value_objects::ProposedToolInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roll_breakdown: Option<String>,
}

impl ChallengeOutcomePendingNotification {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resolution_id: String,
        challenge_id: String,
        challenge_name: String,
        character_id: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
        outcome_description: String,
        outcome_triggers: Vec<crate::domain::value_objects::ProposedToolInfo>,
        roll_breakdown: Option<String>,
    ) -> Self {
        Self {
            message_type: "ChallengeOutcomePending",
            resolution_id,
            challenge_id,
            challenge_name,
            character_id,
            character_name,
            roll,
            modifier,
            total,
            outcome_type,
            outcome_description,
            outcome_triggers,
            roll_breakdown,
        }
    }
}

/// Notification that LLM-generated suggestions are ready
#[derive(Debug, Clone, Serialize)]
pub struct OutcomeSuggestionReadyNotification {
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub resolution_id: String,
    pub suggestions: Vec<String>,
}

impl OutcomeSuggestionReadyNotification {
    pub fn new(resolution_id: String, suggestions: Vec<String>) -> Self {
        Self {
            message_type: "OutcomeSuggestionReady",
            resolution_id,
            suggestions,
        }
    }
}

// ============================================================================
// Outcome Branch DTOs (Phase 22C)
// ============================================================================

/// A single outcome branch option for DM selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeBranchDto {
    /// Unique identifier for this branch (for selection)
    pub id: String,
    /// Short title/summary of this outcome
    pub title: String,
    /// Full narrative description
    pub description: String,
    /// Optional mechanical effects or triggers
    #[serde(default)]
    pub effects: Vec<String>,
}

impl OutcomeBranchDto {
    pub fn new(id: impl Into<String>, title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            effects: Vec::new(),
        }
    }

    pub fn with_effects(mut self, effects: Vec<String>) -> Self {
        self.effects = effects;
        self
    }
}

/// Notification that outcome branches are ready for DM selection
#[derive(Debug, Clone, Serialize)]
pub struct OutcomeBranchesReadyNotification {
    #[serde(rename = "type")]
    pub message_type: &'static str,
    /// Resolution ID this applies to
    pub resolution_id: String,
    /// The outcome tier these branches are for (e.g., "Success", "Critical Failure")
    pub outcome_type: String,
    /// Available outcome branches to choose from
    pub branches: Vec<OutcomeBranchDto>,
}

impl OutcomeBranchesReadyNotification {
    pub fn new(resolution_id: String, outcome_type: String, branches: Vec<OutcomeBranchDto>) -> Self {
        Self {
            message_type: "OutcomeBranchesReady",
            resolution_id,
            outcome_type,
            branches,
        }
    }
}

/// DM's selection of an outcome branch
#[derive(Debug, Clone, Deserialize)]
pub struct OutcomeBranchSelectionRequest {
    /// Resolution ID
    pub resolution_id: String,
    /// ID of the selected branch
    pub branch_id: String,
    /// Optional modifications to the branch description
    #[serde(default)]
    pub modified_description: Option<String>,
}

/// Response containing LLM-generated outcome branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeBranchResponse {
    /// Resolution ID this applies to
    pub resolution_id: String,
    /// Outcome tier
    pub outcome_type: String,
    /// Generated branches
    pub branches: Vec<OutcomeBranchDto>,
}

