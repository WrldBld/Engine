//! World snapshot export implementation
//!
//! This module provides the infrastructure implementation for exporting
//! world snapshots. It implements the `WorldExporterPort` trait defined
//! in the application layer.

use anyhow::Result;
use async_trait::async_trait;

use crate::application::ports::outbound::{
    CharacterData, ExportOptions, LocationData, PlayerWorldSnapshot, SceneData, WorldData,
    WorldExporterPort,
};
use crate::domain::value_objects::{SceneId, WorldId};
use crate::infrastructure::persistence::Neo4jRepository;

/// Load a complete world snapshot for a Player client
///
/// This loads all necessary data from Neo4j and assembles it into a
/// `PlayerWorldSnapshot` suitable for WebSocket transmission.
pub async fn load_world_snapshot(
    world_id: WorldId,
    current_scene_id: Option<SceneId>,
    repository: &Neo4jRepository,
) -> Result<PlayerWorldSnapshot> {
    // Load the world
    let world = repository
        .worlds()
        .get(world_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("World not found: {}", world_id))?;

    // Load all locations
    let locations = repository.locations().list_by_world(world_id).await?;

    // Load all characters
    let characters = repository.characters().list_by_world(world_id).await?;

    // Load all acts and their scenes
    let acts = repository.worlds().get_acts(world_id).await?;
    let mut scenes = Vec::new();
    for act in &acts {
        let act_scenes = repository.scenes().list_by_act(act.id).await?;
        scenes.extend(act_scenes);
    }

    // Convert domain entities to snapshot data structures
    let world_data = WorldData {
        id: world.id.to_string(),
        name: world.name,
        description: world.description,
        rule_system: world.rule_system.into(),
        created_at: world.created_at.to_rfc3339(),
        updated_at: world.updated_at.to_rfc3339(),
    };

    let location_data: Vec<LocationData> = locations
        .into_iter()
        .map(|l| LocationData {
            id: l.id.to_string(),
            name: l.name,
            description: l.description,
            location_type: format!("{:?}", l.location_type),
            backdrop_asset: l.backdrop_asset,
            atmosphere: l.atmosphere,
        })
        .collect();

    let character_data: Vec<CharacterData> = characters
        .into_iter()
        .map(|c| CharacterData {
            id: c.id.to_string(),
            name: c.name,
            description: c.description,
            archetype: format!("{:?}", c.current_archetype),
            sprite_asset: c.sprite_asset,
            portrait_asset: c.portrait_asset,
            is_alive: c.is_alive,
            is_active: c.is_active,
        })
        .collect();

    let scene_data: Vec<SceneData> = scenes
        .iter()
        .map(|s| SceneData {
            id: s.id.to_string(),
            name: s.name.clone(),
            location_id: s.location_id.to_string(),
            time_context: format!("{:?}", s.time_context),
            backdrop_override: s.backdrop_override.clone(),
            featured_characters: s
                .featured_characters
                .iter()
                .map(|c| c.to_string())
                .collect(),
            directorial_notes: s.directorial_notes.clone(),
        })
        .collect();

    // Find the current scene if specified
    let current_scene = current_scene_id.and_then(|scene_id| {
        scenes.iter().find(|s| s.id == scene_id).map(|s| SceneData {
            id: s.id.to_string(),
            name: s.name.clone(),
            location_id: s.location_id.to_string(),
            time_context: format!("{:?}", s.time_context),
            backdrop_override: s.backdrop_override.clone(),
            featured_characters: s
                .featured_characters
                .iter()
                .map(|c| c.to_string())
                .collect(),
            directorial_notes: s.directorial_notes.clone(),
        })
    });

    Ok(PlayerWorldSnapshot {
        world: world_data,
        locations: location_data,
        characters: character_data,
        scenes: scene_data,
        current_scene,
    })
}

/// Builder for creating PlayerWorldSnapshot with additional options
pub struct WorldSnapshotBuilder<'a> {
    world_id: WorldId,
    current_scene_id: Option<SceneId>,
    repository: &'a Neo4jRepository,
    include_inactive_characters: bool,
}

impl<'a> WorldSnapshotBuilder<'a> {
    /// Create a new builder for the given world
    pub fn new(world_id: WorldId, repository: &'a Neo4jRepository) -> Self {
        Self {
            world_id,
            current_scene_id: None,
            repository,
            include_inactive_characters: false,
        }
    }

    /// Set the current scene
    pub fn with_current_scene(mut self, scene_id: SceneId) -> Self {
        self.current_scene_id = Some(scene_id);
        self
    }

    /// Include inactive characters in the snapshot
    pub fn include_inactive_characters(mut self) -> Self {
        self.include_inactive_characters = true;
        self
    }

    /// Build the snapshot
    pub async fn build(self) -> Result<PlayerWorldSnapshot> {
        let mut snapshot =
            load_world_snapshot(self.world_id, self.current_scene_id, self.repository).await?;

        // Filter out inactive characters if not requested
        if !self.include_inactive_characters {
            snapshot.characters.retain(|c| c.is_active);
        }

        Ok(snapshot)
    }
}

/// Implementation of WorldExporterPort using Neo4j repository
pub struct Neo4jWorldExporter {
    repository: Neo4jRepository,
}

impl Neo4jWorldExporter {
    /// Create a new Neo4jWorldExporter with the given repository
    pub fn new(repository: Neo4jRepository) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl WorldExporterPort for Neo4jWorldExporter {
    async fn export_snapshot(&self, world_id: WorldId) -> Result<PlayerWorldSnapshot> {
        WorldSnapshotBuilder::new(world_id, &self.repository)
            .build()
            .await
    }

    async fn export_snapshot_with_options(
        &self,
        world_id: WorldId,
        options: ExportOptions,
    ) -> Result<PlayerWorldSnapshot> {
        let mut builder = WorldSnapshotBuilder::new(world_id, &self.repository);

        if let Some(scene_id) = options.current_scene_id {
            builder = builder.with_current_scene(scene_id);
        }

        if options.include_inactive_characters {
            builder = builder.include_inactive_characters();
        }

        builder.build().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::dto::RuleSystemConfigDto;
    use crate::domain::value_objects::RuleSystemConfig;

    #[test]
    fn test_world_data_serialization() {
        let world = WorldData {
            id: "test-id".to_string(),
            name: "Test World".to_string(),
            description: "A test world".to_string(),
            rule_system: RuleSystemConfigDto::from(RuleSystemConfig::default()),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&world).expect("serialization should succeed");
        assert!(json.contains("Test World"));

        let deserialized: WorldData =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(deserialized.name, "Test World");
    }

    #[test]
    fn test_scene_data_serialization() {
        let scene = SceneData {
            id: "scene-1".to_string(),
            name: "Opening Scene".to_string(),
            location_id: "loc-1".to_string(),
            time_context: "TimeOfDay(Morning)".to_string(),
            backdrop_override: None,
            featured_characters: vec!["char-1".to_string(), "char-2".to_string()],
            directorial_notes: "Set the mood".to_string(),
        };

        let json = serde_json::to_string(&scene).expect("serialization should succeed");
        assert!(json.contains("Opening Scene"));
        assert!(json.contains("char-1"));
    }

    #[test]
    fn test_player_world_snapshot_serialization() {
        let snapshot = PlayerWorldSnapshot {
            world: WorldData {
                id: "world-1".to_string(),
                name: "Fantasy Realm".to_string(),
                description: "A magical world".to_string(),
                rule_system: RuleSystemConfigDto::from(RuleSystemConfig::default()),
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
            },
            locations: vec![LocationData {
                id: "loc-1".to_string(),
                name: "Town Square".to_string(),
                description: "The central plaza".to_string(),
                location_type: "Exterior".to_string(),
                backdrop_asset: Some("town_square.png".to_string()),
                atmosphere: Some("Bustling marketplace atmosphere".to_string()),
            }],
            characters: vec![CharacterData {
                id: "char-1".to_string(),
                name: "Gandalf".to_string(),
                description: "A wise wizard".to_string(),
                archetype: "Mentor".to_string(),
                sprite_asset: None,
                portrait_asset: Some("gandalf_portrait.png".to_string()),
                is_alive: true,
                is_active: true,
            }],
            scenes: vec![],
            current_scene: None,
        };

        let json = serde_json::to_string_pretty(&snapshot).expect("serialization should succeed");
        assert!(json.contains("Fantasy Realm"));
        assert!(json.contains("Town Square"));
        assert!(json.contains("Gandalf"));

        let deserialized: PlayerWorldSnapshot =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(deserialized.world.name, "Fantasy Realm");
        assert_eq!(deserialized.locations.len(), 1);
        assert_eq!(deserialized.characters.len(), 1);
    }
}
