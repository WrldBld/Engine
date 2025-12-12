//! Domain entities - Core business objects with identity

mod challenge;
mod character;
mod gallery_asset;
mod generation_batch;
mod grid_map;
mod interaction;
mod location;
mod scene;
mod sheet_template;
mod skill;
mod workflow_config;
mod world;

pub use challenge::{
    Challenge, ChallengeOutcomes, ChallengeResult, ChallengeType, ComplexChallengeSettings,
    Difficulty, DifficultyDescriptor, Outcome, OutcomeTrigger, OutcomeType, TriggerCondition,
    TriggerType,
};
pub use character::{Character, StatBlock};
pub use gallery_asset::{AssetType, EntityType, GalleryAsset, GenerationMetadata};
pub use generation_batch::{BatchSelection, BatchStatus, GenerationBatch, GenerationRequest};
pub use grid_map::{GridMap, TerrainType, Tile};
pub use interaction::{
    InteractionCondition, InteractionTarget, InteractionTemplate, InteractionType,
};
pub use location::{
    BackdropRegion, Location, LocationConnection, LocationType, RegionBounds, SpatialRelationship,
};
pub use scene::{Scene, SceneCondition, TimeContext};
pub use sheet_template::{
    CharacterSheetData, CharacterSheetTemplate, FieldType, FieldValue, ItemListType,
    SectionLayout, SelectOption, SheetField, SheetSection, SheetTemplateId,
};
pub use skill::{default_skills_for_variant, Skill, SkillCategory};
pub use workflow_config::{
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis,
    WorkflowConfiguration, WorkflowInput, WorkflowSlot,
};
pub use world::{Act, MonomythStage, World};
