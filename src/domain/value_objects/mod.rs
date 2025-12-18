//! Value objects - Immutable objects defined by their attributes

mod approval;
mod archetype;
mod comfyui_config;
mod context_budget;
mod dice;
mod directorial;
mod game_time;
mod game_tools;
mod ids;
mod llm_context;
mod region;
mod relationship;
mod rule_system;
mod settings;

pub use approval::{ApprovalDecision, ProposedToolInfo};
pub use game_time::{GameTime, TimeOfDay};
pub use archetype::{ArchetypeChange, CampbellArchetype};
pub use comfyui_config::ComfyUIConfig;
pub use context_budget::{
    AssembledContext, CategoryContext, ContextBudgetConfig, ContextCategory,
    TokenCountMethod, TokenCounter, count_tokens, exceeds_token_budget,
};
pub use dice::DiceRollInput;
pub use directorial::{DirectorialNotes};
pub use game_tools::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};
pub use ids::*;
pub use llm_context::{
    ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext, ConversationTurn,
    GamePromptRequest, PlayerActionContext, SceneContext,
};
pub use region::{RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift};
pub use relationship::{FamilyRelation, Relationship, RelationshipEvent, RelationshipType};
pub use rule_system::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition, SuccessComparison,
};
pub use settings::{AppSettings, SettingsFieldMetadata, settings_metadata};

// NOTE: Want has been promoted to an entity (domain/entities/want.rs)
// ActantTarget is no longer used - targets are now Neo4j edges
