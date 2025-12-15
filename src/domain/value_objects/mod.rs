//! Value objects - Immutable objects defined by their attributes

mod archetype;
mod directorial;
mod game_tools;
mod ids;
mod queue_items;
mod relationship;
mod rule_system;
mod want;

pub use archetype::{ArchetypeChange, CampbellArchetype};
pub use directorial::{DirectorialNotes};
pub use game_tools::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};
pub use ids::*;
pub use relationship::{Relationship, RelationshipEvent, RelationshipType};
pub use relationship::{FamilyRelation};
pub use rule_system::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition, SuccessComparison,
};
pub use queue_items::{
    ApprovalDecision, ApprovalItem, AssetGenerationItem, DMAction, DMActionItem, DecisionType,
    DecisionUrgency, LLMRequestItem, LLMRequestType, PlayerActionItem,
};

pub use want::{ActantTarget, Want};
