//! System-agnostic rule configuration

use serde::{Deserialize, Serialize};

/// Configuration for a game's rule system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSystemConfig {
    pub name: String,
    pub stat_definitions: Vec<StatDefinition>,
    pub dice_system: DiceSystem,
    pub skill_check_formula: String,
}

impl Default for RuleSystemConfig {
    fn default() -> Self {
        Self {
            name: "Default System".to_string(),
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 20, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 20, 10),
                StatDefinition::new("Constitution", "CON", 1, 20, 10),
                StatDefinition::new("Intelligence", "INT", 1, 20, 10),
                StatDefinition::new("Wisdom", "WIS", 1, 20, 10),
                StatDefinition::new("Charisma", "CHA", 1, 20, 10),
            ],
            dice_system: DiceSystem::D20,
            skill_check_formula: "1d20 + modifier".to_string(),
        }
    }
}

/// Definition of a character stat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatDefinition {
    pub name: String,
    pub abbreviation: String,
    pub min_value: i32,
    pub max_value: i32,
    pub default_value: i32,
}

impl StatDefinition {
    pub fn new(
        name: impl Into<String>,
        abbreviation: impl Into<String>,
        min_value: i32,
        max_value: i32,
        default_value: i32,
    ) -> Self {
        Self {
            name: name.into(),
            abbreviation: abbreviation.into(),
            min_value,
            max_value,
            default_value,
        }
    }
}

/// The dice system used for resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiceSystem {
    /// Classic d20 system (D&D, Pathfinder)
    D20,
    /// Percentile system (Call of Cthulhu)
    D100,
    /// Dice pool system (World of Darkness)
    DicePool { die_type: u8, success_threshold: u8 },
    /// FATE/Fudge dice
    Fate,
    /// Custom dice expression
    Custom(String),
}
