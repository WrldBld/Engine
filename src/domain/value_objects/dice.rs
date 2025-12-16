//! Dice rolling value objects and parsing
//!
//! Supports dice formulas like "1d20+5", "2d6-1", "1d100", etc.
//! Also supports manual result input for physical dice rolls.

use rand::Rng;
use std::fmt;

use super::DiceSystem;

/// Error when parsing a dice formula
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiceParseError {
    /// The formula string is empty
    Empty,
    /// Invalid format - expected XdY or XdY+Z
    InvalidFormat(String),
    /// Dice count must be at least 1
    InvalidDiceCount,
    /// Die size must be at least 2
    InvalidDieSize,
    /// Modifier overflow
    ModifierOverflow,
}

impl fmt::Display for DiceParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty dice formula"),
            Self::InvalidFormat(s) => write!(f, "Invalid dice format: {}", s),
            Self::InvalidDiceCount => write!(f, "Dice count must be at least 1"),
            Self::InvalidDieSize => write!(f, "Die size must be at least 2"),
            Self::ModifierOverflow => write!(f, "Modifier value overflow"),
        }
    }
}

impl std::error::Error for DiceParseError {}

/// A parsed dice formula like "2d6+3"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceFormula {
    /// Number of dice to roll (X in XdY)
    pub dice_count: u8,
    /// Size of each die (Y in XdY)
    pub die_size: u8,
    /// Modifier to add/subtract after rolling (+Z or -Z)
    pub modifier: i32,
}

impl DiceFormula {
    /// Create a new dice formula
    pub fn new(dice_count: u8, die_size: u8, modifier: i32) -> Result<Self, DiceParseError> {
        if dice_count == 0 {
            return Err(DiceParseError::InvalidDiceCount);
        }
        if die_size < 2 {
            return Err(DiceParseError::InvalidDieSize);
        }
        Ok(Self {
            dice_count,
            die_size,
            modifier,
        })
    }

    /// Parse a dice formula string like "1d20+5", "2d6-1", "1d100"
    ///
    /// Supported formats:
    /// - "XdY" - Roll X dice of size Y
    /// - "XdY+Z" - Roll X dice of size Y, add Z
    /// - "XdY-Z" - Roll X dice of size Y, subtract Z
    /// - "dY" - Roll 1 die of size Y (shorthand)
    pub fn parse(input: &str) -> Result<Self, DiceParseError> {
        let input = input.trim().to_lowercase();
        if input.is_empty() {
            return Err(DiceParseError::Empty);
        }

        // Regex pattern: (\d*)d(\d+)([+-]\d+)?
        // But we'll parse manually to avoid regex dependency in domain layer

        // Find 'd' separator
        let d_pos = input.find('d').ok_or_else(|| {
            DiceParseError::InvalidFormat(format!("Missing 'd' separator in '{}'", input))
        })?;

        // Parse dice count (before 'd')
        let dice_count_str = &input[..d_pos];
        let dice_count: u8 = if dice_count_str.is_empty() {
            1 // "d20" means "1d20"
        } else {
            dice_count_str.parse().map_err(|_| {
                DiceParseError::InvalidFormat(format!("Invalid dice count: '{}'", dice_count_str))
            })?
        };

        if dice_count == 0 {
            return Err(DiceParseError::InvalidDiceCount);
        }

        // Parse die size and modifier (after 'd')
        let after_d = &input[d_pos + 1..];

        // Find modifier separator (+ or -)
        let (die_size_str, modifier) = if let Some(plus_pos) = after_d.find('+') {
            let die_str = &after_d[..plus_pos];
            let mod_str = &after_d[plus_pos + 1..];
            let modifier: i32 = mod_str.parse().map_err(|_| {
                DiceParseError::InvalidFormat(format!("Invalid modifier: '+{}'", mod_str))
            })?;
            (die_str, modifier)
        } else if let Some(minus_pos) = after_d.rfind('-') {
            // Use rfind to handle negative numbers correctly
            if minus_pos == 0 {
                // The '-' is at the start, which is invalid for die size
                return Err(DiceParseError::InvalidFormat(format!(
                    "Invalid die size: '{}'",
                    after_d
                )));
            }
            let die_str = &after_d[..minus_pos];
            let mod_str = &after_d[minus_pos + 1..];
            let modifier: i32 = mod_str.parse::<i32>().map_err(|_| {
                DiceParseError::InvalidFormat(format!("Invalid modifier: '-{}'", mod_str))
            })?;
            (die_str, -modifier)
        } else {
            (after_d, 0)
        };

        let die_size: u8 = die_size_str.parse().map_err(|_| {
            DiceParseError::InvalidFormat(format!("Invalid die size: '{}'", die_size_str))
        })?;

        if die_size < 2 {
            return Err(DiceParseError::InvalidDieSize);
        }

        Ok(Self {
            dice_count,
            die_size,
            modifier,
        })
    }

    /// Get the default dice formula for a rule system
    pub fn default_for_system(dice_system: &DiceSystem) -> Self {
        match dice_system {
            DiceSystem::D20 => Self {
                dice_count: 1,
                die_size: 20,
                modifier: 0,
            },
            DiceSystem::D100 => Self {
                dice_count: 1,
                die_size: 100,
                modifier: 0,
            },
            DiceSystem::DicePool { die_type, .. } => Self {
                dice_count: 1,
                die_size: *die_type,
                modifier: 0,
            },
            DiceSystem::Fate => Self {
                // FATE uses 4dF, but we represent as 4d3-8 (each F die is -1/0/+1)
                // Actually, we'll use a special format: 4d3-8 gives range -4 to +4
                dice_count: 4,
                die_size: 3,
                modifier: -8, // Adjusts 4-12 range to -4 to +4
            },
            DiceSystem::Custom(desc) => {
                // Try to parse the custom description, fall back to d20
                if desc.contains("2d6") {
                    Self {
                        dice_count: 2,
                        die_size: 6,
                        modifier: 0,
                    }
                } else if desc.contains("d100") {
                    Self {
                        dice_count: 1,
                        die_size: 100,
                        modifier: 0,
                    }
                } else {
                    Self {
                        dice_count: 1,
                        die_size: 20,
                        modifier: 0,
                    }
                }
            }
        }
    }

    /// Roll the dice and return the result
    pub fn roll(&self) -> DiceRollResult {
        let mut rng = rand::thread_rng();
        let mut individual_rolls = Vec::with_capacity(self.dice_count as usize);

        for _ in 0..self.dice_count {
            let roll = rng.gen_range(1..=self.die_size as i32);
            individual_rolls.push(roll);
        }

        let dice_total: i32 = individual_rolls.iter().sum();
        let total = dice_total + self.modifier;

        DiceRollResult {
            formula: self.clone(),
            individual_rolls,
            dice_total,
            modifier_applied: self.modifier,
            total,
        }
    }

    /// Get the minimum possible roll
    pub fn min_roll(&self) -> i32 {
        self.dice_count as i32 + self.modifier
    }

    /// Get the maximum possible roll
    pub fn max_roll(&self) -> i32 {
        (self.dice_count as i32 * self.die_size as i32) + self.modifier
    }

    /// Format as a display string (e.g., "1d20+5")
    pub fn display(&self) -> String {
        if self.modifier == 0 {
            format!("{}d{}", self.dice_count, self.die_size)
        } else if self.modifier > 0 {
            format!("{}d{}+{}", self.dice_count, self.die_size, self.modifier)
        } else {
            format!("{}d{}{}", self.dice_count, self.die_size, self.modifier)
        }
    }
}

impl fmt::Display for DiceFormula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Result of rolling dice
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceRollResult {
    /// The formula that was rolled
    pub formula: DiceFormula,
    /// Individual die results
    pub individual_rolls: Vec<i32>,
    /// Sum of dice before modifier
    pub dice_total: i32,
    /// Modifier that was applied
    pub modifier_applied: i32,
    /// Final total (dice_total + modifier)
    pub total: i32,
}

impl DiceRollResult {
    /// Create a result from a manual input (no actual dice rolled)
    pub fn from_manual(total: i32) -> Self {
        Self {
            formula: DiceFormula {
                dice_count: 0,
                die_size: 0,
                modifier: 0,
            },
            individual_rolls: vec![],
            dice_total: total,
            modifier_applied: 0,
            total,
        }
    }

    /// Check if this was a manual roll
    pub fn is_manual(&self) -> bool {
        self.formula.dice_count == 0
    }

    /// Format as a breakdown string (e.g., "1d20(14) + 5 = 19" or "Manual: 18")
    pub fn breakdown(&self) -> String {
        if self.is_manual() {
            format!("Manual: {}", self.total)
        } else if self.individual_rolls.len() == 1 {
            // Single die
            let roll = self.individual_rolls[0];
            if self.modifier_applied == 0 {
                format!("{}({}) = {}", self.formula.display(), roll, self.total)
            } else if self.modifier_applied > 0 {
                format!(
                    "{}d{}({}) + {} = {}",
                    self.formula.dice_count,
                    self.formula.die_size,
                    roll,
                    self.modifier_applied,
                    self.total
                )
            } else {
                format!(
                    "{}d{}({}) - {} = {}",
                    self.formula.dice_count,
                    self.formula.die_size,
                    roll,
                    -self.modifier_applied,
                    self.total
                )
            }
        } else {
            // Multiple dice
            let rolls_str: Vec<String> = self.individual_rolls.iter().map(|r| r.to_string()).collect();
            if self.modifier_applied == 0 {
                format!(
                    "{}[{}] = {}",
                    self.formula.display(),
                    rolls_str.join(", "),
                    self.total
                )
            } else if self.modifier_applied > 0 {
                format!(
                    "{}d{}[{}] + {} = {}",
                    self.formula.dice_count,
                    self.formula.die_size,
                    rolls_str.join(", "),
                    self.modifier_applied,
                    self.total
                )
            } else {
                format!(
                    "{}d{}[{}] - {} = {}",
                    self.formula.dice_count,
                    self.formula.die_size,
                    rolls_str.join(", "),
                    -self.modifier_applied,
                    self.total
                )
            }
        }
    }

    /// Check if this is a natural 20 (for d20 systems)
    pub fn is_natural_20(&self) -> bool {
        self.formula.die_size == 20
            && self.formula.dice_count == 1
            && self.individual_rolls.first() == Some(&20)
    }

    /// Check if this is a natural 1 (for d20 systems)
    pub fn is_natural_1(&self) -> bool {
        self.formula.die_size == 20
            && self.formula.dice_count == 1
            && self.individual_rolls.first() == Some(&1)
    }
}

/// Input for a dice roll - either a formula to roll or a manual result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiceRollInput {
    /// Roll dice using a formula string like "1d20+5"
    Formula(String),
    /// Use a manual result (physical dice roll)
    ManualResult(i32),
}

impl DiceRollInput {
    /// Resolve the input to a roll result
    pub fn resolve(&self) -> Result<DiceRollResult, DiceParseError> {
        match self {
            Self::Formula(formula_str) => {
                let formula = DiceFormula::parse(formula_str)?;
                Ok(formula.roll())
            }
            Self::ManualResult(total) => Ok(DiceRollResult::from_manual(*total)),
        }
    }

    /// Resolve with an additional modifier (from character skills)
    pub fn resolve_with_modifier(&self, skill_modifier: i32) -> Result<DiceRollResult, DiceParseError> {
        match self {
            Self::Formula(formula_str) => {
                let mut formula = DiceFormula::parse(formula_str)?;
                formula.modifier += skill_modifier;
                Ok(formula.roll())
            }
            Self::ManualResult(total) => {
                // For manual results, the player already factored in their modifier
                Ok(DiceRollResult::from_manual(*total))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_d20() {
        let formula = DiceFormula::parse("1d20").unwrap();
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 20);
        assert_eq!(formula.modifier, 0);
    }

    #[test]
    fn test_parse_shorthand_d20() {
        let formula = DiceFormula::parse("d20").unwrap();
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 20);
        assert_eq!(formula.modifier, 0);
    }

    #[test]
    fn test_parse_with_positive_modifier() {
        let formula = DiceFormula::parse("1d20+5").unwrap();
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 20);
        assert_eq!(formula.modifier, 5);
    }

    #[test]
    fn test_parse_with_negative_modifier() {
        let formula = DiceFormula::parse("1d20-3").unwrap();
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 20);
        assert_eq!(formula.modifier, -3);
    }

    #[test]
    fn test_parse_multiple_dice() {
        let formula = DiceFormula::parse("2d6+3").unwrap();
        assert_eq!(formula.dice_count, 2);
        assert_eq!(formula.die_size, 6);
        assert_eq!(formula.modifier, 3);
    }

    #[test]
    fn test_parse_d100() {
        let formula = DiceFormula::parse("1d100").unwrap();
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 100);
        assert_eq!(formula.modifier, 0);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let formula = DiceFormula::parse("1D20+5").unwrap();
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 20);
        assert_eq!(formula.modifier, 5);
    }

    #[test]
    fn test_parse_with_whitespace() {
        let formula = DiceFormula::parse("  1d20+5  ").unwrap();
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 20);
        assert_eq!(formula.modifier, 5);
    }

    #[test]
    fn test_parse_empty() {
        assert!(matches!(DiceFormula::parse(""), Err(DiceParseError::Empty)));
    }

    #[test]
    fn test_parse_invalid_no_d() {
        assert!(matches!(
            DiceFormula::parse("20"),
            Err(DiceParseError::InvalidFormat(_))
        ));
    }

    #[test]
    fn test_parse_invalid_zero_dice() {
        assert!(matches!(
            DiceFormula::parse("0d20"),
            Err(DiceParseError::InvalidDiceCount)
        ));
    }

    #[test]
    fn test_parse_invalid_die_size() {
        assert!(matches!(
            DiceFormula::parse("1d1"),
            Err(DiceParseError::InvalidDieSize)
        ));
    }

    #[test]
    fn test_roll_range() {
        let formula = DiceFormula::parse("1d20").unwrap();
        for _ in 0..100 {
            let result = formula.roll();
            assert!(result.total >= 1 && result.total <= 20);
        }
    }

    #[test]
    fn test_roll_with_modifier() {
        let formula = DiceFormula::parse("1d20+5").unwrap();
        for _ in 0..100 {
            let result = formula.roll();
            assert!(result.total >= 6 && result.total <= 25);
            assert_eq!(result.modifier_applied, 5);
        }
    }

    #[test]
    fn test_breakdown_single_die() {
        let result = DiceRollResult {
            formula: DiceFormula::new(1, 20, 5).unwrap(),
            individual_rolls: vec![14],
            dice_total: 14,
            modifier_applied: 5,
            total: 19,
        };
        assert_eq!(result.breakdown(), "1d20(14) + 5 = 19");
    }

    #[test]
    fn test_breakdown_multiple_dice() {
        let result = DiceRollResult {
            formula: DiceFormula::new(2, 6, 3).unwrap(),
            individual_rolls: vec![4, 5],
            dice_total: 9,
            modifier_applied: 3,
            total: 12,
        };
        assert_eq!(result.breakdown(), "2d6[4, 5] + 3 = 12");
    }

    #[test]
    fn test_breakdown_manual() {
        let result = DiceRollResult::from_manual(18);
        assert_eq!(result.breakdown(), "Manual: 18");
        assert!(result.is_manual());
    }

    #[test]
    fn test_natural_20() {
        let result = DiceRollResult {
            formula: DiceFormula::new(1, 20, 0).unwrap(),
            individual_rolls: vec![20],
            dice_total: 20,
            modifier_applied: 0,
            total: 20,
        };
        assert!(result.is_natural_20());
        assert!(!result.is_natural_1());
    }

    #[test]
    fn test_natural_1() {
        let result = DiceRollResult {
            formula: DiceFormula::new(1, 20, 0).unwrap(),
            individual_rolls: vec![1],
            dice_total: 1,
            modifier_applied: 0,
            total: 1,
        };
        assert!(result.is_natural_1());
        assert!(!result.is_natural_20());
    }

    #[test]
    fn test_dice_roll_input_formula() {
        let input = DiceRollInput::Formula("1d20+5".to_string());
        let result = input.resolve().unwrap();
        assert!(!result.is_manual());
        assert!(result.total >= 6 && result.total <= 25);
    }

    #[test]
    fn test_dice_roll_input_manual() {
        let input = DiceRollInput::ManualResult(18);
        let result = input.resolve().unwrap();
        assert!(result.is_manual());
        assert_eq!(result.total, 18);
    }

    #[test]
    fn test_resolve_with_modifier() {
        let input = DiceRollInput::Formula("1d20".to_string());
        let result = input.resolve_with_modifier(5).unwrap();
        assert!(result.total >= 6 && result.total <= 25);
        assert_eq!(result.modifier_applied, 5);
    }

    #[test]
    fn test_default_for_d20_system() {
        let formula = DiceFormula::default_for_system(&DiceSystem::D20);
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 20);
        assert_eq!(formula.modifier, 0);
    }

    #[test]
    fn test_default_for_d100_system() {
        let formula = DiceFormula::default_for_system(&DiceSystem::D100);
        assert_eq!(formula.dice_count, 1);
        assert_eq!(formula.die_size, 100);
        assert_eq!(formula.modifier, 0);
    }

    #[test]
    fn test_display() {
        assert_eq!(DiceFormula::new(1, 20, 0).unwrap().display(), "1d20");
        assert_eq!(DiceFormula::new(1, 20, 5).unwrap().display(), "1d20+5");
        assert_eq!(DiceFormula::new(1, 20, -3).unwrap().display(), "1d20-3");
        assert_eq!(DiceFormula::new(2, 6, 3).unwrap().display(), "2d6+3");
    }
}
