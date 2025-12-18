//! Domain entities - Core business objects with identity

mod challenge;
mod character;
mod event_chain;
mod gallery_asset;
mod generation_batch;
mod goal;
mod grid_map;
mod interaction;
mod item;
mod location;
mod narrative_event;
mod observation;
mod region;
mod player_character;
mod scene;
mod sheet_template;
mod skill;
mod story_event;
mod want;
mod workflow_config;
mod world;

pub use challenge::{
    Challenge, ChallengeLocationAvailability, ChallengeOutcomes, ChallengePrerequisite,
    ChallengeType, ChallengeUnlock, Difficulty, DifficultyDescriptor, Outcome, OutcomeType,
    OutcomeTrigger, TriggerCondition, TriggerType,
};
pub use character::{Character, StatBlock};
pub use event_chain::{ChainStatus, EventChain};
pub use gallery_asset::{AssetType, EntityType, GalleryAsset, GenerationMetadata};
pub use generation_batch::{BatchStatus, GenerationBatch, GenerationRequest};
pub use goal::Goal;
pub use grid_map::GridMap;
pub use interaction::{
    InteractionCondition, InteractionRequirement, InteractionTarget, InteractionTargetType,
    InteractionTemplate, InteractionType,
};
pub use item::{AcquisitionMethod, FrequencyLevel, InventoryItem, Item};
pub use location::{Location, LocationConnection, LocationType};
pub use region::{MapBounds, Region, RegionConnection, RegionExit};
pub use narrative_event::{
    ChainedEvent, EventChainMembership, EventEffect, EventOutcome, FeaturedNpc, NarrativeEvent,
    NarrativeTrigger, NarrativeTriggerType, OutcomeCondition, TriggerContext, TriggerEvaluation,
    TriggerLogic,
};
pub use observation::{NpcObservation, ObservationSummary, ObservationType};
pub use player_character::PlayerCharacter;
pub use scene::{Scene, SceneCharacter, SceneCharacterRole, SceneCondition, TimeContext, TimeOfDay};
pub use sheet_template::{
    CharacterSheetData, CharacterSheetTemplate, FieldType, FieldValue, ItemListType,
    SectionLayout, SelectOption, SheetField, SheetSection, SheetTemplateId,
};
pub use skill::{default_skills_for_variant, Skill, SkillCategory};
pub use story_event::{
    ChallengeEventOutcome, CombatEventType, CombatOutcome, DmMarkerType, InfoImportance, InfoType,
    InvolvedCharacter, ItemSource, MarkerImportance, StoryEvent, StoryEventType,
};
pub use want::{ActantialRole, ActantialView, CharacterWant, Want, WantTargetType};
pub use workflow_config::{
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis,
    WorkflowConfiguration, WorkflowInput, WorkflowSlot,
};
pub use world::{Act, MonomythStage, World};
