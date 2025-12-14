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

// ============================================================================
// Catalog/lookup DTOs (used by rule system routes)
// ============================================================================

/// Summary of a preset for browsing.
#[derive(Debug, Serialize)]
pub struct RuleSystemPresetSummaryDto {
    pub variant: RuleSystemVariantDto,
    pub name: String,
    pub description: String,
}

/// Summary of a rule system type for browsing.
#[derive(Debug, Serialize)]
pub struct RuleSystemSummaryDto {
    pub system_type: RuleSystemTypeDto,
    pub name: String,
    pub description: String,
    pub dice_notation: String,
    pub presets: Vec<RuleSystemPresetSummaryDto>,
}

/// Full preset details.
#[derive(Debug, Serialize)]
pub struct RuleSystemPresetDetailsDto {
    pub variant: RuleSystemVariantDto,
    pub config: RuleSystemConfigDto,
}

/// Details about a rule system type.
#[derive(Debug, Serialize)]
pub struct RuleSystemTypeDetailsDto {
    pub system_type: RuleSystemTypeDto,
    pub name: String,
    pub description: String,
    pub dice_notation: String,
    pub presets: Vec<RuleSystemPresetSummaryDto>,
}

pub fn parse_system_type(s: &str) -> Result<RuleSystemType, String> {
    match s.to_lowercase().as_str() {
        "d20" => Ok(RuleSystemType::D20),
        "d100" => Ok(RuleSystemType::D100),
        "narrative" => Ok(RuleSystemType::Narrative),
        "custom" => Ok(RuleSystemType::Custom),
        _ => Err(format!(
            "Unknown rule system type: {}. Valid types: d20, d100, narrative, custom",
            s
        )),
    }
}

pub fn parse_variant(s: &str) -> Result<RuleSystemVariant, String> {
    match s.to_lowercase().replace("-", "_").as_str() {
        "dnd5e" | "dnd_5e" => Ok(RuleSystemVariant::Dnd5e),
        "pathfinder2e" | "pathfinder_2e" => Ok(RuleSystemVariant::Pathfinder2e),
        "generic_d20" | "genericd20" => Ok(RuleSystemVariant::GenericD20),
        "coc7e" | "coc_7e" | "callofcthulhu7e" | "call_of_cthulhu_7e" => {
            Ok(RuleSystemVariant::CallOfCthulhu7e)
        }
        "runequest" | "rune_quest" => Ok(RuleSystemVariant::RuneQuest),
        "generic_d100" | "genericd100" => Ok(RuleSystemVariant::GenericD100),
        "kidsonbikes" | "kids_on_bikes" => Ok(RuleSystemVariant::KidsOnBikes),
        "fatecore" | "fate_core" | "fate" => Ok(RuleSystemVariant::FateCore),
        "pbta" | "poweredbyapocalypse" | "powered_by_apocalypse" => {
            Ok(RuleSystemVariant::PoweredByApocalypse)
        }
        _ => Err(format!(
            "Unknown variant: {}. Valid variants: dnd5e, pathfinder2e, generic_d20, coc7e, runequest, generic_d100, kidsonbikes, fatecore, pbta",
            s
        )),
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

