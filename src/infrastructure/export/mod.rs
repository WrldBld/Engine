//! World export functionality
//!
//! This module provides export capabilities for worlds,
//! allowing them to be serialized to JSON for the Player to consume.
//!
//! Two export formats are available:
//! - [`JsonExporter`] / [`WorldSnapshot`]: Full export with all data for archival/backup
//! - [`PlayerWorldSnapshot`]: Streamlined snapshot for real-time Player client transmission

mod json_exporter;
mod world_snapshot;

pub use json_exporter::{JsonExporter, WorldSnapshot};
pub use world_snapshot::{
    load_world_snapshot, CharacterData as PlayerCharacterData, LocationData as PlayerLocationData,
    PlayerWorldSnapshot, SceneData as PlayerSceneData, WorldData as PlayerWorldData,
    WorldSnapshotBuilder,
};
