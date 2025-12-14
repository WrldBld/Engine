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
    pub scene_id: Option<String>,
    pub name: String,
    pub description: String,
    pub challenge_type: ChallengeTypeDto,
    pub skill_id: String,
    pub difficulty: DifficultyRequestDto,
    pub outcomes: OutcomesRequestDto,
    pub trigger_conditions: Vec<TriggerConditionRequestDto>,
    pub prerequisite_challenges: Vec<String>,
    pub active: bool,
    pub order: u32,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

impl From<Challenge> for ChallengeResponseDto {
    fn from(c: Challenge) -> Self {
        Self {
            id: c.id.to_string(),
            world_id: c.world_id.to_string(),
            scene_id: c.scene_id.map(|s| s.to_string()),
            name: c.name,
            description: c.description,
            challenge_type: c.challenge_type.into(),
            skill_id: c.skill_id.to_string(),
            difficulty: c.difficulty.into(),
            outcomes: c.outcomes.into(),
            trigger_conditions: c.trigger_conditions.into_iter().map(Into::into).collect(),
            prerequisite_challenges: c
                .prerequisite_challenges
                .iter()
                .map(|id| id.to_string())
                .collect(),
            active: c.active,
            order: c.order,
            is_favorite: c.is_favorite,
            tags: c.tags,
        }
    }
}

