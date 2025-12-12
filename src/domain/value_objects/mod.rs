//! Value objects - Immutable objects defined by their attributes

mod archetype;
mod directorial;
mod game_tools;
mod ids;
mod relationship;
mod rule_system;
mod want;

pub use archetype::{ArchetypeChange, CampbellArchetype};
pub use directorial::{DirectorialNotes, NpcMotivation, PacingGuidance, ToneGuidance};
pub use game_tools::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};
pub use ids::*;
pub use relationship::{Relationship, RelationshipEvent, RelationshipType};
pub use rule_system::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition,
    SuccessComparison,
};
pub use want::{ActantTarget, Want};
