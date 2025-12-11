//! Value objects - Immutable objects defined by their attributes

mod archetype;
mod directorial;
mod ids;
mod relationship;
mod rule_system;
mod want;

pub use archetype::{ArchetypeChange, CampbellArchetype};
pub use directorial::{DirectorialNotes, NpcMotivation, PacingGuidance, ToneGuidance};
pub use ids::*;
pub use relationship::{Relationship, RelationshipEvent, RelationshipType};
pub use rule_system::{DiceSystem, RuleSystemConfig, StatDefinition};
pub use want::{ActantTarget, Want};
