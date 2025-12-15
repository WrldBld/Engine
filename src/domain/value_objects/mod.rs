//! Value objects - Immutable objects defined by their attributes

mod approval;
mod archetype;
mod directorial;
mod game_tools;
mod ids;
mod llm_context;
mod relationship;
mod rule_system;
mod want;

pub use approval::{ApprovalDecision, ProposedToolInfo};
pub use archetype::{ArchetypeChange, CampbellArchetype};
pub use directorial::{DirectorialNotes};
pub use game_tools::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};
pub use ids::*;
pub use llm_context::{
    ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext, ConversationTurn,
    GamePromptRequest, PlayerActionContext, SceneContext,
};
pub use relationship::{Relationship, RelationshipEvent, RelationshipType};
pub use relationship::{FamilyRelation};
pub use rule_system::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition, SuccessComparison,
};

pub use want::{ActantTarget, Want};
