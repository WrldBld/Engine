use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition, SuccessComparison,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSystemTypeDto {
    D20,
    D100,
    Narrative,
    Custom,
}

impl From<RuleSystemType> for RuleSystemTypeDto {
    fn from(value: RuleSystemType) -> Self {
        match value {
            RuleSystemType::D20 => Self::D20,
            RuleSystemType::D100 => Self::D100,
            RuleSystemType::Narrative => Self::Narrative,
            RuleSystemType::Custom => Self::Custom,
        }
    }
}

impl From<RuleSystemTypeDto> for RuleSystemType {
    fn from(value: RuleSystemTypeDto) -> Self {
        match value {
            RuleSystemTypeDto::D20 => Self::D20,
            RuleSystemTypeDto::D100 => Self::D100,
            RuleSystemTypeDto::Narrative => Self::Narrative,
            RuleSystemTypeDto::Custom => Self::Custom,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSystemVariantDto {
    Dnd5e,
    Pathfinder2e,
    GenericD20,
    CallOfCthulhu7e,
    RuneQuest,
    GenericD100,
    KidsOnBikes,
    FateCore,
    PoweredByApocalypse,
    Custom(String),
}

impl From<RuleSystemVariant> for RuleSystemVariantDto {
    fn from(value: RuleSystemVariant) -> Self {
        match value {
            RuleSystemVariant::Dnd5e => Self::Dnd5e,
            RuleSystemVariant::Pathfinder2e => Self::Pathfinder2e,
            RuleSystemVariant::GenericD20 => Self::GenericD20,
            RuleSystemVariant::CallOfCthulhu7e => Self::CallOfCthulhu7e,
            RuleSystemVariant::RuneQuest => Self::RuneQuest,
            RuleSystemVariant::GenericD100 => Self::GenericD100,
            RuleSystemVariant::KidsOnBikes => Self::KidsOnBikes,
            RuleSystemVariant::FateCore => Self::FateCore,
            RuleSystemVariant::PoweredByApocalypse => Self::PoweredByApocalypse,
            RuleSystemVariant::Custom(s) => Self::Custom(s),
        }
    }
}

impl From<RuleSystemVariantDto> for RuleSystemVariant {
    fn from(value: RuleSystemVariantDto) -> Self {
        match value {
            RuleSystemVariantDto::Dnd5e => Self::Dnd5e,
            RuleSystemVariantDto::Pathfinder2e => Self::Pathfinder2e,
            RuleSystemVariantDto::GenericD20 => Self::GenericD20,
            RuleSystemVariantDto::CallOfCthulhu7e => Self::CallOfCthulhu7e,
            RuleSystemVariantDto::RuneQuest => Self::RuneQuest,
            RuleSystemVariantDto::GenericD100 => Self::GenericD100,
            RuleSystemVariantDto::KidsOnBikes => Self::KidsOnBikes,
            RuleSystemVariantDto::FateCore => Self::FateCore,
            RuleSystemVariantDto::PoweredByApocalypse => Self::PoweredByApocalypse,
            RuleSystemVariantDto::Custom(s) => Self::Custom(s),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuccessComparisonDto {
    GreaterOrEqual,
    LessOrEqual,
    Narrative,
}

impl From<SuccessComparison> for SuccessComparisonDto {
    fn from(value: SuccessComparison) -> Self {
        match value {
            SuccessComparison::GreaterOrEqual => Self::GreaterOrEqual,
            SuccessComparison::LessOrEqual => Self::LessOrEqual,
            SuccessComparison::Narrative => Self::Narrative,
        }
    }
}

impl From<SuccessComparisonDto> for SuccessComparison {
    fn from(value: SuccessComparisonDto) -> Self {
        match value {
            SuccessComparisonDto::GreaterOrEqual => Self::GreaterOrEqual,
            SuccessComparisonDto::LessOrEqual => Self::LessOrEqual,
            SuccessComparisonDto::Narrative => Self::Narrative,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatDefinitionDto {
    pub name: String,
    pub abbreviation: String,
    pub min_value: i32,
    pub max_value: i32,
    pub default_value: i32,
}

impl From<StatDefinition> for StatDefinitionDto {
    fn from(value: StatDefinition) -> Self {
        Self {
            name: value.name,
            abbreviation: value.abbreviation,
            min_value: value.min_value,
            max_value: value.max_value,
            default_value: value.default_value,
        }
    }
}

impl From<StatDefinitionDto> for StatDefinition {
    fn from(value: StatDefinitionDto) -> Self {
        Self {
            name: value.name,
            abbreviation: value.abbreviation,
            min_value: value.min_value,
            max_value: value.max_value,
            default_value: value.default_value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiceSystemDto {
    D20,
    D100,
    DicePool { die_type: u8, success_threshold: u8 },
    Fate,
    Custom(String),
}

impl From<DiceSystem> for DiceSystemDto {
    fn from(value: DiceSystem) -> Self {
        match value {
            DiceSystem::D20 => Self::D20,
            DiceSystem::D100 => Self::D100,
            DiceSystem::DicePool {
                die_type,
                success_threshold,
            } => Self::DicePool {
                die_type,
                success_threshold,
            },
            DiceSystem::Fate => Self::Fate,
            DiceSystem::Custom(s) => Self::Custom(s),
        }
    }
}

impl From<DiceSystemDto> for DiceSystem {
    fn from(value: DiceSystemDto) -> Self {
        match value {
            DiceSystemDto::D20 => Self::D20,
            DiceSystemDto::D100 => Self::D100,
            DiceSystemDto::DicePool {
                die_type,
                success_threshold,
            } => Self::DicePool {
                die_type,
                success_threshold,
            },
            DiceSystemDto::Fate => Self::Fate,
            DiceSystemDto::Custom(s) => Self::Custom(s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleSystemConfigDto {
    pub name: String,
    pub description: String,
    pub system_type: RuleSystemTypeDto,
    pub variant: RuleSystemVariantDto,
    pub stat_definitions: Vec<StatDefinitionDto>,
    pub dice_system: DiceSystemDto,
    pub success_comparison: SuccessComparisonDto,
    pub skill_check_formula: String,
}

impl From<RuleSystemConfig> for RuleSystemConfigDto {
    fn from(value: RuleSystemConfig) -> Self {
        Self {
            name: value.name,
            description: value.description,
            system_type: value.system_type.into(),
            variant: value.variant.into(),
            stat_definitions: value
                .stat_definitions
                .into_iter()
                .map(StatDefinitionDto::from)
                .collect(),
            dice_system: value.dice_system.into(),
            success_comparison: value.success_comparison.into(),
            skill_check_formula: value.skill_check_formula,
        }
    }
}

impl From<RuleSystemConfigDto> for RuleSystemConfig {
    fn from(value: RuleSystemConfigDto) -> Self {
        Self {
            name: value.name,
            description: value.description,
            system_type: value.system_type.into(),
            variant: value.variant.into(),
            stat_definitions: value
                .stat_definitions
                .into_iter()
                .map(StatDefinition::from)
                .collect(),
            dice_system: value.dice_system.into(),
            success_comparison: value.success_comparison.into(),
            skill_check_formula: value.skill_check_formula,
        }
    }
}

