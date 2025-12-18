//! World Exporter Port - Interface for exporting world snapshots
//!
//! This port abstracts the world export functionality, allowing the
//! application layer to request world snapshots without depending
//! on the concrete infrastructure implementation.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::application::dto::RuleSystemConfigDto;
use crate::domain::value_objects::{SceneId, WorldId};

/// Simplified world snapshot for Player clients
///
/// Contains the essential data needed by the Player to render the game world.
/// This is sent when a client joins a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerWorldSnapshot {
    /// The world metadata
    pub world: WorldData,
    /// All locations in the world
    pub locations: Vec<LocationData>,
    /// All characters in the world
    pub characters: Vec<CharacterData>,
    /// All scenes in the world
    pub scenes: Vec<SceneData>,
    /// The current active scene (if any)
    pub current_scene: Option<SceneData>,
}

/// World metadata for Player clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_system: RuleSystemConfigDto,
    pub created_at: String,
    pub updated_at: String,
}

/// Location data for Player clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub location_type: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    // Note: parent_id is now derived from CONTAINS_LOCATION edges, not stored here
}

/// Character data for Player clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub archetype: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_alive: bool,
    pub is_active: bool,
}

/// Scene data for Player clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneData {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub time_context: String,
    pub backdrop_override: Option<String>,
    pub featured_characters: Vec<String>,
    pub directorial_notes: String,
}

/// Options for exporting a world snapshot
#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    /// The current scene to mark as active
    pub current_scene_id: Option<SceneId>,
    /// Whether to include inactive characters
    pub include_inactive_characters: bool,
}

/// World Exporter port trait
///
/// This trait defines the interface for exporting world snapshots.
/// Infrastructure adapters implement this to provide the actual
/// data loading and transformation logic.
#[async_trait]
pub trait WorldExporterPort: Send + Sync {
    /// Export a world snapshot with default options
    async fn export_snapshot(&self, world_id: WorldId) -> Result<PlayerWorldSnapshot>;

    /// Export a world snapshot with custom options
    async fn export_snapshot_with_options(
        &self,
        world_id: WorldId,
        options: ExportOptions,
    ) -> Result<PlayerWorldSnapshot>;
}
