//! Challenge entity - Skill checks, ability checks, and other game challenges
//!
//! Challenges can be attached to scenes and triggered either manually by the DM
//! or suggested by the LLM when trigger conditions are met.
//!
//! ## Graph-First Design (Phase 0.E)
//!
//! Challenges use Neo4j edges for relationships:
//! - `(Challenge)-[:REQUIRES_SKILL]->(Skill)` - Skill tested by this challenge
//! - `(Challenge)-[:TIED_TO_SCENE]->(Scene)` - Scene this challenge appears in
//! - `(Challenge)-[:REQUIRES_COMPLETION_OF {success_required}]->(Challenge)` - Prerequisites
//! - `(Challenge)-[:AVAILABLE_AT {always_available, time_restriction}]->(Location)` - Location availability
//! - `(Challenge)-[:ON_SUCCESS_UNLOCKS]->(Location)` - Location unlocked on success
//!
//! The embedded fields `scene_id`, `skill_id`, and `prerequisite_challenges` are
//! DEPRECATED and kept only for backward compatibility during migration.

use crate::domain::value_objects::{ChallengeId, LocationId, SceneId, SkillId, WorldId};

/// A challenge that can be triggered during gameplay
///
/// ## Graph Relationships (via repository edge methods)
/// - `REQUIRES_SKILL` -> Skill: The skill tested by this challenge
/// - `TIED_TO_SCENE` -> Scene: Scene this challenge appears in (optional)
/// - `REQUIRES_COMPLETION_OF` -> Challenge: Prerequisite challenges
/// - `AVAILABLE_AT` -> Location: Locations where this challenge is available
/// - `ON_SUCCESS_UNLOCKS` -> Location: Locations unlocked on success
#[derive(Debug, Clone)]
pub struct Challenge {
    pub id: ChallengeId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub challenge_type: ChallengeType,
    pub difficulty: Difficulty,
    pub outcomes: ChallengeOutcomes,
    /// Conditions that trigger LLM to suggest this challenge (non-entity triggers stored as JSON)
    pub trigger_conditions: Vec<TriggerCondition>,
    /// Whether this challenge can currently be triggered
    pub active: bool,
    /// Display order in challenge library
    pub order: u32,
    /// Whether the DM favorited this challenge
    pub is_favorite: bool,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl Challenge {
    /// Create a new challenge.
    ///
    /// Note: The skill relationship should be set via `ChallengeRepositoryPort::set_required_skill()`
    /// after creating the challenge.
    pub fn new(
        world_id: WorldId,
        name: impl Into<String>,
        difficulty: Difficulty,
    ) -> Self {
        Self {
            id: ChallengeId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            challenge_type: ChallengeType::SkillCheck,
            difficulty,
            outcomes: ChallengeOutcomes::default(),
            trigger_conditions: Vec::new(),
            active: true,
            order: 0,
            is_favorite: false,
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_challenge_type(mut self, challenge_type: ChallengeType) -> Self {
        self.challenge_type = challenge_type;
        self
    }

    pub fn with_outcomes(mut self, outcomes: ChallengeOutcomes) -> Self {
        self.outcomes = outcomes;
        self
    }

    pub fn with_trigger(mut self, condition: TriggerCondition) -> Self {
        self.trigger_conditions.push(condition);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Check if a trigger condition matches some player action/context
    pub fn matches_trigger(&self, action: &str, context: &str) -> bool {
        self.trigger_conditions.iter().any(|tc| tc.matches(action, context))
    }
}

/// Types of challenges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeType {
    /// Standard skill check against difficulty
    SkillCheck,
    /// Raw attribute/ability check (no skill proficiency)
    AbilityCheck,
    /// Reactive defense check
    SavingThrow,
    /// Contest against another entity's skill
    OpposedCheck,
    /// Multi-roll challenge requiring accumulated successes
    ComplexChallenge,
}

impl Default for ChallengeType {
    fn default() -> Self {
        Self::SkillCheck
    }
}

impl ChallengeType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SkillCheck => "Skill Check",
            Self::AbilityCheck => "Ability Check",
            Self::SavingThrow => "Saving Throw",
            Self::OpposedCheck => "Opposed Check",
            Self::ComplexChallenge => "Complex Challenge",
        }
    }
}

/// Challenge difficulty representation
#[derive(Debug, Clone, PartialEq)]
pub enum Difficulty {
    /// D20-style: roll + modifier >= DC
    DC(u32),
    /// D100-style: roll <= percentage target
    Percentage(u32),
    /// Narrative systems: descriptive difficulty
    Descriptor(DifficultyDescriptor),
    /// Opposed roll: compare to opponent's roll
    Opposed,
    /// Custom difficulty with notes
    Custom(String),
}

impl Default for Difficulty {
    fn default() -> Self {
        Self::DC(10)
    }
}

impl Difficulty {
    /// Get a human-readable description
    pub fn display(&self) -> String {
        match self {
            Self::DC(dc) => format!("DC {}", dc),
            Self::Percentage(p) => format!("{}%", p),
            Self::Descriptor(d) => d.display_name().to_string(),
            Self::Opposed => "Opposed".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// Standard D20 difficulty presets
    pub fn d20_easy() -> Self { Self::DC(10) }
    pub fn d20_medium() -> Self { Self::DC(15) }
    pub fn d20_hard() -> Self { Self::DC(20) }
    pub fn d20_very_hard() -> Self { Self::DC(25) }

    /// D100 difficulty presets (based on typical skill values)
    pub fn d100_regular() -> Self { Self::Percentage(100) }
    pub fn d100_hard() -> Self { Self::Percentage(50) }
    pub fn d100_extreme() -> Self { Self::Percentage(20) }
}

/// Descriptive difficulty for narrative systems
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifficultyDescriptor {
    Trivial,
    Easy,
    Routine,
    Moderate,
    Challenging,
    Hard,
    VeryHard,
    Extreme,
    Impossible,
    // PbtA-style
    Risky,
    Desperate,
}

impl DifficultyDescriptor {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Trivial => "Trivial",
            Self::Easy => "Easy",
            Self::Routine => "Routine",
            Self::Moderate => "Moderate",
            Self::Challenging => "Challenging",
            Self::Hard => "Hard",
            Self::VeryHard => "Very Hard",
            Self::Extreme => "Extreme",
            Self::Impossible => "Impossible",
            Self::Risky => "Risky",
            Self::Desperate => "Desperate",
        }
    }
}

/// Outcomes for a challenge
#[derive(Debug, Clone, Default)]
pub struct ChallengeOutcomes {
    pub success: Outcome,
    pub failure: Outcome,
    /// For narrative systems or "meet DC exactly" results
    pub partial: Option<Outcome>,
    /// Natural 20 or roll of 01 on d100
    pub critical_success: Option<Outcome>,
    /// Natural 1 or fumble roll
    pub critical_failure: Option<Outcome>,
}

impl ChallengeOutcomes {
    pub fn simple(success: impl Into<String>, failure: impl Into<String>) -> Self {
        Self {
            success: Outcome::new(success),
            failure: Outcome::new(failure),
            partial: None,
            critical_success: None,
            critical_failure: None,
        }
    }

    pub fn with_partial(mut self, partial: impl Into<String>) -> Self {
        self.partial = Some(Outcome::new(partial));
        self
    }

    pub fn with_critical_success(mut self, critical: impl Into<String>) -> Self {
        self.critical_success = Some(Outcome::new(critical));
        self
    }

    pub fn with_critical_failure(mut self, critical: impl Into<String>) -> Self {
        self.critical_failure = Some(Outcome::new(critical));
        self
    }
}

/// A single outcome with narrative text and triggered effects
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Narrative description shown to players
    pub description: String,
    /// Effects that trigger when this outcome occurs
    pub triggers: Vec<OutcomeTrigger>,
}

impl Outcome {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            triggers: Vec::new(),
        }
    }

    pub fn with_trigger(mut self, trigger: OutcomeTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }
}

/// Effects triggered by challenge outcomes
#[derive(Debug, Clone)]
pub enum OutcomeTrigger {
    /// Reveal hidden information to the player
    RevealInformation {
        info: String,
        /// Whether to add to journal/notes
        persist: bool,
    },
    /// Enable another challenge (unlock prerequisite)
    EnableChallenge {
        challenge_id: ChallengeId,
    },
    /// Disable a challenge (remove from available)
    DisableChallenge {
        challenge_id: ChallengeId,
    },
    /// Modify a character stat (HP, Sanity, etc.)
    ModifyCharacterStat {
        stat: String,
        modifier: i32,
    },
    /// Trigger a scene transition
    TriggerScene {
        scene_id: SceneId,
    },
    /// Add an item to inventory
    GiveItem {
        item_name: String,
        item_description: Option<String>,
    },
    /// Custom trigger with free-text description
    Custom {
        description: String,
    },
}

impl OutcomeTrigger {
    pub fn reveal(info: impl Into<String>) -> Self {
        Self::RevealInformation {
            info: info.into(),
            persist: false,
        }
    }

    pub fn reveal_persistent(info: impl Into<String>) -> Self {
        Self::RevealInformation {
            info: info.into(),
            persist: true,
        }
    }

    pub fn enable(challenge_id: ChallengeId) -> Self {
        Self::EnableChallenge { challenge_id }
    }

    pub fn disable(challenge_id: ChallengeId) -> Self {
        Self::DisableChallenge { challenge_id }
    }

    pub fn modify_stat(stat: impl Into<String>, modifier: i32) -> Self {
        Self::ModifyCharacterStat {
            stat: stat.into(),
            modifier,
        }
    }

    pub fn scene(scene_id: SceneId) -> Self {
        Self::TriggerScene { scene_id }
    }
}

/// Condition that triggers LLM to suggest a challenge
#[derive(Debug, Clone)]
pub struct TriggerCondition {
    pub condition_type: TriggerType,
    /// Human-readable description for DM reference
    pub description: String,
    /// Whether this condition alone is sufficient (AND vs OR logic)
    pub required: bool,
}

impl TriggerCondition {
    pub fn new(condition_type: TriggerType, description: impl Into<String>) -> Self {
        Self {
            condition_type,
            description: description.into(),
            required: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Check if this condition matches the given action/context
    pub fn matches(&self, action: &str, context: &str) -> bool {
        self.condition_type.matches(action, context)
    }
}

/// Types of trigger conditions
#[derive(Debug, Clone)]
pub enum TriggerType {
    /// Player interacts with specific object
    ObjectInteraction {
        /// Object keywords to match
        keywords: Vec<String>,
    },
    /// Player enters specific area/location
    EnterArea {
        area_keywords: Vec<String>,
    },
    /// Player discusses specific topic
    DialogueTopic {
        topic_keywords: Vec<String>,
    },
    /// Another challenge completed (success or failure)
    ChallengeComplete {
        challenge_id: ChallengeId,
        /// None = either, Some(true) = success only, Some(false) = failure only
        requires_success: Option<bool>,
    },
    /// Time-based trigger (after N turns/exchanges)
    TimeBased {
        turns: u32,
    },
    /// NPC present in scene
    NpcPresent {
        npc_keywords: Vec<String>,
    },
    /// Free-text condition for LLM interpretation
    Custom {
        description: String,
    },
}

impl TriggerType {
    /// Check if this trigger type matches the given action/context
    pub fn matches(&self, action: &str, context: &str) -> bool {
        let action_lower = action.to_lowercase();
        let context_lower = context.to_lowercase();

        match self {
            Self::ObjectInteraction { keywords } => {
                keywords.iter().any(|k| {
                    let k_lower = k.to_lowercase();
                    action_lower.contains(&k_lower) || context_lower.contains(&k_lower)
                })
            }
            Self::EnterArea { area_keywords } => {
                area_keywords.iter().any(|k| {
                    let k_lower = k.to_lowercase();
                    action_lower.contains(&k_lower) || context_lower.contains(&k_lower)
                })
            }
            Self::DialogueTopic { topic_keywords } => {
                topic_keywords.iter().any(|k| {
                    let k_lower = k.to_lowercase();
                    action_lower.contains(&k_lower) || context_lower.contains(&k_lower)
                })
            }
            Self::Custom { description } => {
                // Custom triggers rely on LLM interpretation
                // This basic implementation checks for keyword overlap
                let desc_lower = description.to_lowercase();
                let desc_words: Vec<&str> = desc_lower.split_whitespace().collect();
                desc_words.iter().filter(|w| w.len() > 3).any(|w| {
                    action_lower.contains(*w) || context_lower.contains(*w)
                })
            }
            // These require external state to evaluate
            Self::ChallengeComplete { .. } | Self::TimeBased { .. } | Self::NpcPresent { .. } => {
                false
            }
        }
    }

    pub fn object(keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::ObjectInteraction {
            keywords: keywords.into_iter().map(|k| k.into()).collect(),
        }
    }

    pub fn area(keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::EnterArea {
            area_keywords: keywords.into_iter().map(|k| k.into()).collect(),
        }
    }

    pub fn topic(keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::DialogueTopic {
            topic_keywords: keywords.into_iter().map(|k| k.into()).collect(),
        }
    }

    pub fn after_challenge(challenge_id: ChallengeId) -> Self {
        Self::ChallengeComplete {
            challenge_id,
            requires_success: None,
        }
    }

    pub fn after_challenge_success(challenge_id: ChallengeId) -> Self {
        Self::ChallengeComplete {
            challenge_id,
            requires_success: Some(true),
        }
    }

    pub fn custom(description: impl Into<String>) -> Self {
        Self::Custom {
            description: description.into(),
        }
    }
}

/// Result of a challenge resolution
#[derive(Debug, Clone)]
pub struct ChallengeResult {
    pub challenge_id: ChallengeId,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub outcome_type: OutcomeType,
    pub outcome: Outcome,
    /// For complex challenges: progress toward required successes
    pub accumulated_successes: Option<u32>,
    pub accumulated_failures: Option<u32>,
}

/// Type of outcome achieved
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeType {
    CriticalSuccess,
    Success,
    Partial,
    Failure,
    CriticalFailure,
}

impl OutcomeType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CriticalSuccess => "Critical Success!",
            Self::Success => "Success",
            Self::Partial => "Partial Success",
            Self::Failure => "Failure",
            Self::CriticalFailure => "Critical Failure!",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::CriticalSuccess | Self::Success | Self::Partial)
    }
}

/// Settings for complex (multi-roll) challenges
#[derive(Debug, Clone)]
pub struct ComplexChallengeSettings {
    /// Number of successes required to complete
    pub required_successes: u32,
    /// Number of failures before challenge is failed (0 = unlimited)
    pub max_failures: u32,
    /// Whether different skills can be used for each roll
    pub flexible_skills: bool,
    /// Skills allowed if flexible_skills is true
    pub allowed_skills: Vec<SkillId>,
}

impl Default for ComplexChallengeSettings {
    fn default() -> Self {
        Self {
            required_successes: 3,
            max_failures: 3,
            flexible_skills: false,
            allowed_skills: Vec::new(),
        }
    }
}

// =============================================================================
// Edge Support Structs (Graph-First Design)
// =============================================================================

/// Data for REQUIRES_COMPLETION_OF edge between challenges
#[derive(Debug, Clone)]
pub struct ChallengePrerequisite {
    /// The prerequisite challenge ID
    pub challenge_id: ChallengeId,
    /// Whether success is required (true = must succeed, false = just attempted)
    pub success_required: bool,
}

impl ChallengePrerequisite {
    pub fn new(challenge_id: ChallengeId) -> Self {
        Self {
            challenge_id,
            success_required: false,
        }
    }

    pub fn requiring_success(challenge_id: ChallengeId) -> Self {
        Self {
            challenge_id,
            success_required: true,
        }
    }
}

/// Data for AVAILABLE_AT edge between Challenge and Location
#[derive(Debug, Clone)]
pub struct ChallengeLocationAvailability {
    /// The location where this challenge is available
    pub location_id: LocationId,
    /// Whether the challenge is always available at this location
    pub always_available: bool,
    /// Time restriction (if any): "Morning", "Afternoon", "Evening", "Night"
    pub time_restriction: Option<String>,
}

impl ChallengeLocationAvailability {
    pub fn new(location_id: LocationId) -> Self {
        Self {
            location_id,
            always_available: true,
            time_restriction: None,
        }
    }

    pub fn with_time_restriction(mut self, time: impl Into<String>) -> Self {
        self.time_restriction = Some(time.into());
        self.always_available = false;
        self
    }
}

/// Data for ON_SUCCESS_UNLOCKS edge between Challenge and Location
#[derive(Debug, Clone)]
pub struct ChallengeUnlock {
    /// The location that gets unlocked on successful completion
    pub location_id: LocationId,
}

impl ChallengeUnlock {
    pub fn new(location_id: LocationId) -> Self {
        Self { location_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_creation() {
        let world_id = WorldId::new();

        let challenge = Challenge::new(world_id, "Investigate the Statue", Difficulty::d20_medium())
            .with_description("Examine the ancient statue for hidden compartments")
            .with_outcomes(ChallengeOutcomes::simple(
                "You find a hidden mechanism in the statue's base",
                "The statue appears to be solid stone"
            ));

        assert_eq!(challenge.name, "Investigate the Statue");
        assert!(challenge.active);
        assert_eq!(challenge.outcomes.success.description, "You find a hidden mechanism in the statue's base");
    }

    #[test]
    fn test_trigger_condition_matching() {
        let trigger = TriggerCondition::new(
            TriggerType::object(["statue", "ancient", "stone"]),
            "When player examines the statue"
        );

        assert!(trigger.matches("I want to examine the statue", ""));
        assert!(trigger.matches("look at", "there is an ancient monument here"));
        assert!(!trigger.matches("I walk away", "there is a door"));
    }

    #[test]
    fn test_difficulty_display() {
        assert_eq!(Difficulty::DC(15).display(), "DC 15");
        assert_eq!(Difficulty::Percentage(45).display(), "45%");
        assert_eq!(Difficulty::Descriptor(DifficultyDescriptor::Hard).display(), "Hard");
    }

    #[test]
    fn test_outcome_triggers() {
        let outcome = Outcome::new("You discover a secret passage!")
            .with_trigger(OutcomeTrigger::reveal_persistent("Map of the catacombs"))
            .with_trigger(OutcomeTrigger::enable(ChallengeId::new()));

        assert_eq!(outcome.triggers.len(), 2);
    }
}
