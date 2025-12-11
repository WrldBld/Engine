//! Domain entities - Core business objects with identity

mod character;
mod gallery_asset;
mod generation_batch;
mod grid_map;
mod interaction;
mod location;
mod scene;
mod world;

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
pub use world::{Act, MonomythStage, World};
