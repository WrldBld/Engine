//! System-agnostic rule configuration
//!
//! Supports multiple TTRPG systems through presets and customization.

/// The type of rule system (determines dice mechanics and success calculation)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSystemType {
    /// Roll d20 + modifier vs DC (D&D, Pathfinder)
    D20,
    /// Roll d100 under skill value (Call of Cthulhu, RuneQuest)
    D100,
    /// Fiction-first with descriptive outcomes (Kids on Bikes, FATE, PbtA)
    Narrative,
    /// User-defined dice mechanics
    Custom,
}

impl Default for RuleSystemType {
    fn default() -> Self {
        Self::D20
    }
}

/// Known presets for rule systems
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleSystemVariant {
    // D20 variants
    Dnd5e,
    Pathfinder2e,
    GenericD20,
    // D100 variants
    CallOfCthulhu7e,
    RuneQuest,
    GenericD100,
    // Narrative variants
    KidsOnBikes,
    FateCore,
    PoweredByApocalypse,
    // Custom
    Custom(String),
}

impl Default for RuleSystemVariant {
    fn default() -> Self {
        Self::GenericD20
    }
}

impl RuleSystemVariant {
    /// Get the display name for this variant
    pub fn display_name(&self) -> &str {
        match self {
            Self::Dnd5e => "D&D 5th Edition",
            Self::Pathfinder2e => "Pathfinder 2nd Edition",
            Self::GenericD20 => "Generic D20 System",
            Self::CallOfCthulhu7e => "Call of Cthulhu 7th Edition",
            Self::RuneQuest => "RuneQuest",
            Self::GenericD100 => "Generic D100 System",
            Self::KidsOnBikes => "Kids on Bikes",
            Self::FateCore => "FATE Core",
            Self::PoweredByApocalypse => "Powered by the Apocalypse",
            Self::Custom(name) => name,
        }
    }

    /// Get the rule system type for this variant
    pub fn system_type(&self) -> RuleSystemType {
        match self {
            Self::Dnd5e | Self::Pathfinder2e | Self::GenericD20 => RuleSystemType::D20,
            Self::CallOfCthulhu7e | Self::RuneQuest | Self::GenericD100 => RuleSystemType::D100,
            Self::KidsOnBikes | Self::FateCore | Self::PoweredByApocalypse => RuleSystemType::Narrative,
            Self::Custom(_) => RuleSystemType::Custom,
        }
    }

    /// Get all variants for a given system type
    pub fn variants_for_type(system_type: RuleSystemType) -> Vec<Self> {
        match system_type {
            RuleSystemType::D20 => vec![Self::Dnd5e, Self::Pathfinder2e, Self::GenericD20],
            RuleSystemType::D100 => vec![Self::CallOfCthulhu7e, Self::RuneQuest, Self::GenericD100],
            RuleSystemType::Narrative => vec![Self::KidsOnBikes, Self::FateCore, Self::PoweredByApocalypse],
            RuleSystemType::Custom => vec![],
        }
    }
}

/// Configuration for a game's rule system
#[derive(Debug, Clone)]
pub struct RuleSystemConfig {
    /// Display name for this configuration
    pub name: String,
    /// Description of how the system works
    pub description: String,
    /// The type of rule system
    pub system_type: RuleSystemType,
    /// The specific variant/preset used
    pub variant: RuleSystemVariant,
    /// Character stat definitions
    pub stat_definitions: Vec<StatDefinition>,
    /// The dice system used for resolution
    pub dice_system: DiceSystem,
    /// How success/failure is determined
    pub success_comparison: SuccessComparison,
    /// Formula for skill checks (display only)
    pub skill_check_formula: String,
}

impl Default for RuleSystemConfig {
    fn default() -> Self {
        Self::from_variant(RuleSystemVariant::GenericD20)
    }
}

impl RuleSystemConfig {
    /// Create a configuration from a preset variant
    pub fn from_variant(variant: RuleSystemVariant) -> Self {
        match variant {
            RuleSystemVariant::Dnd5e => Self::dnd_5e(),
            RuleSystemVariant::Pathfinder2e => Self::pathfinder_2e(),
            RuleSystemVariant::GenericD20 => Self::generic_d20(),
            RuleSystemVariant::CallOfCthulhu7e => Self::call_of_cthulhu_7e(),
            RuleSystemVariant::RuneQuest => Self::runequest(),
            RuleSystemVariant::GenericD100 => Self::generic_d100(),
            RuleSystemVariant::KidsOnBikes => Self::kids_on_bikes(),
            RuleSystemVariant::FateCore => Self::fate_core(),
            RuleSystemVariant::PoweredByApocalypse => Self::powered_by_apocalypse(),
            RuleSystemVariant::Custom(name) => Self::custom(name),
        }
    }

    /// D&D 5th Edition preset
    pub fn dnd_5e() -> Self {
        Self {
            name: "D&D 5th Edition".to_string(),
            system_type: RuleSystemType::D20,
            variant: RuleSystemVariant::Dnd5e,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 20, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 20, 10),
                StatDefinition::new("Constitution", "CON", 1, 20, 10),
                StatDefinition::new("Intelligence", "INT", 1, 20, 10),
                StatDefinition::new("Wisdom", "WIS", 1, 20, 10),
                StatDefinition::new("Charisma", "CHA", 1, 20, 10),
            ],
            dice_system: DiceSystem::D20,
            success_comparison: SuccessComparison::GreaterOrEqual,
            skill_check_formula: "1d20 + ability modifier + proficiency (if proficient)".to_string(),
            description: "Roll d20, add modifiers. Meet or beat the DC to succeed.".to_string(),
        }
    }

    /// Pathfinder 2e preset
    pub fn pathfinder_2e() -> Self {
        Self {
            name: "Pathfinder 2nd Edition".to_string(),
            system_type: RuleSystemType::D20,
            variant: RuleSystemVariant::Pathfinder2e,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 20, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 20, 10),
                StatDefinition::new("Constitution", "CON", 1, 20, 10),
                StatDefinition::new("Intelligence", "INT", 1, 20, 10),
                StatDefinition::new("Wisdom", "WIS", 1, 20, 10),
                StatDefinition::new("Charisma", "CHA", 1, 20, 10),
            ],
            dice_system: DiceSystem::D20,
            success_comparison: SuccessComparison::GreaterOrEqual,
            skill_check_formula: "1d20 + modifier vs DC (4 degrees of success)".to_string(),
            description: "Roll d20 + modifier. Crit success on DC+10, crit fail on DC-10.".to_string(),
        }
    }

    /// Generic D20 preset
    pub fn generic_d20() -> Self {
        Self {
            name: "Generic D20 System".to_string(),
            system_type: RuleSystemType::D20,
            variant: RuleSystemVariant::GenericD20,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 20, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 20, 10),
                StatDefinition::new("Constitution", "CON", 1, 20, 10),
                StatDefinition::new("Intelligence", "INT", 1, 20, 10),
                StatDefinition::new("Wisdom", "WIS", 1, 20, 10),
                StatDefinition::new("Charisma", "CHA", 1, 20, 10),
            ],
            dice_system: DiceSystem::D20,
            success_comparison: SuccessComparison::GreaterOrEqual,
            skill_check_formula: "1d20 + modifier vs DC".to_string(),
            description: "Roll d20, add modifiers. Meet or beat the DC to succeed.".to_string(),
        }
    }

    /// Call of Cthulhu 7e preset
    pub fn call_of_cthulhu_7e() -> Self {
        Self {
            name: "Call of Cthulhu 7th Edition".to_string(),
            system_type: RuleSystemType::D100,
            variant: RuleSystemVariant::CallOfCthulhu7e,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 100, 50),
                StatDefinition::new("Constitution", "CON", 1, 100, 50),
                StatDefinition::new("Size", "SIZ", 1, 100, 50),
                StatDefinition::new("Dexterity", "DEX", 1, 100, 50),
                StatDefinition::new("Appearance", "APP", 1, 100, 50),
                StatDefinition::new("Intelligence", "INT", 1, 100, 50),
                StatDefinition::new("Power", "POW", 1, 100, 50),
                StatDefinition::new("Education", "EDU", 1, 100, 50),
                StatDefinition::new("Luck", "LCK", 1, 100, 50),
            ],
            dice_system: DiceSystem::D100,
            success_comparison: SuccessComparison::LessOrEqual,
            skill_check_formula: "Roll d100 ≤ skill value".to_string(),
            description: "Roll d100. Regular success ≤ skill, Hard ≤ half, Extreme ≤ fifth.".to_string(),
        }
    }

    /// RuneQuest preset
    pub fn runequest() -> Self {
        Self {
            name: "RuneQuest".to_string(),
            system_type: RuleSystemType::D100,
            variant: RuleSystemVariant::RuneQuest,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 21, 10),
                StatDefinition::new("Constitution", "CON", 1, 21, 10),
                StatDefinition::new("Size", "SIZ", 1, 21, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 21, 10),
                StatDefinition::new("Intelligence", "INT", 1, 21, 10),
                StatDefinition::new("Power", "POW", 1, 21, 10),
                StatDefinition::new("Charisma", "CHA", 1, 21, 10),
            ],
            dice_system: DiceSystem::D100,
            success_comparison: SuccessComparison::LessOrEqual,
            skill_check_formula: "Roll d100 ≤ skill value".to_string(),
            description: "Roll d100 under skill. Critical on 1/20th, special on 1/5th.".to_string(),
        }
    }

    /// Generic D100 preset
    pub fn generic_d100() -> Self {
        Self {
            name: "Generic D100 System".to_string(),
            system_type: RuleSystemType::D100,
            variant: RuleSystemVariant::GenericD100,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 100, 50),
                StatDefinition::new("Dexterity", "DEX", 1, 100, 50),
                StatDefinition::new("Constitution", "CON", 1, 100, 50),
                StatDefinition::new("Intelligence", "INT", 1, 100, 50),
                StatDefinition::new("Wisdom", "WIS", 1, 100, 50),
                StatDefinition::new("Charisma", "CHA", 1, 100, 50),
            ],
            dice_system: DiceSystem::D100,
            success_comparison: SuccessComparison::LessOrEqual,
            skill_check_formula: "Roll d100 ≤ skill value".to_string(),
            description: "Roll d100 and compare to skill value. Lower is better.".to_string(),
        }
    }

    /// Kids on Bikes preset
    pub fn kids_on_bikes() -> Self {
        Self {
            name: "Kids on Bikes".to_string(),
            system_type: RuleSystemType::Narrative,
            variant: RuleSystemVariant::KidsOnBikes,
            stat_definitions: vec![
                StatDefinition::new("Brains", "BRN", 4, 20, 8),
                StatDefinition::new("Brawn", "BRW", 4, 20, 8),
                StatDefinition::new("Fight", "FGT", 4, 20, 8),
                StatDefinition::new("Flight", "FLT", 4, 20, 8),
                StatDefinition::new("Charm", "CHM", 4, 20, 8),
                StatDefinition::new("Grit", "GRT", 4, 20, 8),
            ],
            dice_system: DiceSystem::Custom("Variable die (d4-d20)".to_string()),
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "Roll stat die vs difficulty (1-6 scale)".to_string(),
            description: "Roll your stat die. Higher stat = bigger die. Narrative outcomes.".to_string(),
        }
    }

    /// FATE Core preset
    pub fn fate_core() -> Self {
        Self {
            name: "FATE Core".to_string(),
            system_type: RuleSystemType::Narrative,
            variant: RuleSystemVariant::FateCore,
            stat_definitions: vec![
                StatDefinition::new("Careful", "CAR", 0, 4, 1),
                StatDefinition::new("Clever", "CLV", 0, 4, 1),
                StatDefinition::new("Flashy", "FLS", 0, 4, 1),
                StatDefinition::new("Forceful", "FRC", 0, 4, 1),
                StatDefinition::new("Quick", "QCK", 0, 4, 1),
                StatDefinition::new("Sneaky", "SNK", 0, 4, 1),
            ],
            dice_system: DiceSystem::Fate,
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "4dF + approach vs difficulty ladder".to_string(),
            description: "Roll 4 Fate dice (+/-/blank) + approach. Compare to ladder.".to_string(),
        }
    }

    /// Powered by the Apocalypse preset
    pub fn powered_by_apocalypse() -> Self {
        Self {
            name: "Powered by the Apocalypse".to_string(),
            system_type: RuleSystemType::Narrative,
            variant: RuleSystemVariant::PoweredByApocalypse,
            stat_definitions: vec![
                StatDefinition::new("Cool", "COL", -2, 3, 0),
                StatDefinition::new("Hard", "HRD", -2, 3, 0),
                StatDefinition::new("Hot", "HOT", -2, 3, 0),
                StatDefinition::new("Sharp", "SHP", -2, 3, 0),
                StatDefinition::new("Weird", "WRD", -2, 3, 0),
            ],
            dice_system: DiceSystem::Custom("2d6".to_string()),
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "2d6 + stat: 10+ full success, 7-9 partial, 6- miss".to_string(),
            description: "Roll 2d6 + stat. 10+ success, 7-9 success with cost, 6- trouble.".to_string(),
        }
    }

    /// Custom system (blank slate)
    pub fn custom(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_type: RuleSystemType::Custom,
            variant: RuleSystemVariant::Custom("Custom".to_string()),
            stat_definitions: vec![],
            dice_system: DiceSystem::Custom("Custom".to_string()),
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "Custom resolution".to_string(),
            description: "A custom rule system. Define your own stats and mechanics.".to_string(),
        }
    }
}

/// How success is determined
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessComparison {
    /// Roll must be >= target (D20 systems)
    GreaterOrEqual,
    /// Roll must be <= target (D100 systems)
    LessOrEqual,
    /// Success is determined narratively
    Narrative,
}

/// Definition of a character stat
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
