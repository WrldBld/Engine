//! Domain entities - Core business objects with identity

mod challenge;
mod character;
mod event_chain;
mod gallery_asset;
mod generation_batch;
mod grid_map;
mod interaction;
mod location;
mod narrative_event;
mod player_character;
mod scene;
mod sheet_template;
mod skill;
mod story_event;
mod workflow_config;
mod world;

pub use challenge::{
    Challenge, ChallengeOutcomes, ChallengeType, Difficulty, DifficultyDescriptor,
    Outcome, OutcomeType, OutcomeTrigger, TriggerCondition, TriggerType,
};
pub use character::{Character, StatBlock};
pub use event_chain::{ChainStatus, EventChain};
pub use gallery_asset::{AssetType, EntityType, GalleryAsset, GenerationMetadata};
pub use generation_batch::{BatchStatus, GenerationBatch, GenerationRequest};
pub use grid_map::GridMap;
pub use interaction::{
    InteractionCondition, InteractionTarget, InteractionTemplate, InteractionType,
};
pub use location::{
    BackdropRegion, ConnectionRequirement, Location, LocationConnection, LocationType, RegionBounds,
    SpatialRelationship,
};
pub use narrative_event::{
    ChainedEvent, EventEffect, EventOutcome, NarrativeEvent, NarrativeTrigger,
    NarrativeTriggerType, OutcomeCondition, TriggerLogic,
};
pub use player_character::PlayerCharacter;
pub use scene::{Scene, SceneCondition, TimeContext};
pub use scene::TimeOfDay;
pub use sheet_template::{
    CharacterSheetData, CharacterSheetTemplate, FieldType, FieldValue, ItemListType,
    SectionLayout, SelectOption, SheetField, SheetSection, SheetTemplateId,
};
pub use skill::{default_skills_for_variant, Skill, SkillCategory};
pub use story_event::{
    ChallengeEventOutcome, CombatEventType, CombatOutcome, DmMarkerType, InfoImportance, InfoType,
    ItemSource, MarkerImportance, StoryEvent, StoryEventType,
};
pub use workflow_config::{
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis,
    WorkflowConfiguration, WorkflowInput, WorkflowSlot,
};
pub use world::{Act, MonomythStage, World};
