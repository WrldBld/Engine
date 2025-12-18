//! JSON exporter for world data
//!
//! Exports complete world snapshots that can be consumed by the Player.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::application::dto::RuleSystemConfigDto;
use crate::domain::value_objects::WorldId;
use crate::infrastructure::persistence::Neo4jRepository;

/// Complete snapshot of a world for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// Metadata about this snapshot
    pub metadata: SnapshotMetadata,
    /// The world itself
    pub world: WorldData,
    /// All acts in the world
    pub acts: Vec<ActData>,
    /// All scenes in the world
    pub scenes: Vec<SceneData>,
    /// All characters in the world
    pub characters: Vec<CharacterData>,
    /// All locations in the world
    pub locations: Vec<LocationData>,
    /// All relationships between characters
    pub relationships: Vec<RelationshipData>,
    /// Location connections (graph edges)
    pub connections: Vec<ConnectionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub version: String,
    pub exported_at: String,
    pub engine_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_system: RuleSystemConfigDto,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActData {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub stage: String,
    pub description: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneData {
    pub id: String,
    pub act_id: String,
    pub name: String,
    pub location_id: String,
    pub time_context: String,
    pub backdrop_override: Option<String>,
    pub featured_characters: Vec<String>,
    pub directorial_notes: String,
    pub entry_conditions: Vec<String>,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub base_archetype: String,
    pub current_archetype: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_alive: bool,
    pub is_active: bool,
    pub stats: serde_json::Value,
    // NOTE: Wants are now stored as separate nodes with HAS_WANT edges
    // They are not embedded in the character export for now
    // TODO: Add wants export via graph traversal in Phase 0.H
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationData {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub location_type: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    // Note: parent_id, grid_map_id, and backdrop_regions are now edges in Neo4j
    // They can be reconstructed from separate queries if needed for export
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionData {
    pub id: String,
    pub location_id: String,
    pub name: String,
    pub description: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub map_bounds: Option<MapBoundsData>,
    pub is_spawn_point: bool,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBoundsData {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipData {
    pub id: String,
    pub from_character_id: String,
    pub to_character_id: String,
    pub relationship_type: String,
    pub sentiment: f32,
    pub known_to_player: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionData {
    pub from_location_id: String,
    pub to_location_id: String,
    pub connection_type: String,
    pub description: Option<String>,
    pub bidirectional: bool,
    pub travel_time: u32,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

/// JSON exporter for creating world snapshots
pub struct JsonExporter {
    repository: Neo4jRepository,
}

impl JsonExporter {
    pub fn new(repository: Neo4jRepository) -> Self {
        Self { repository }
    }

    /// Export a complete world snapshot
    pub async fn export_world(&self, world_id: WorldId) -> Result<WorldSnapshot> {
        // Get the world
        let world = self
            .repository
            .worlds()
            .get(world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found"))?;

        // Get all acts
        let acts = self.repository.worlds().get_acts(world_id).await?;

        // Get all scenes for all acts
        let mut scenes = Vec::new();
        for act in &acts {
            let act_scenes = self.repository.scenes().list_by_act(act.id).await?;
            scenes.extend(act_scenes);
        }

        // Get all characters
        let characters = self.repository.characters().list_by_world(world_id).await?;

        // Get all locations
        let locations = self.repository.locations().list_by_world(world_id).await?;

        // Get all relationships (social network)
        let social_network = self
            .repository
            .relationships()
            .get_social_network(world_id)
            .await?;

        // Get all location connections
        let mut connections = Vec::new();
        for location in &locations {
            let loc_connections = self
                .repository
                .locations()
                .get_connections(location.id)
                .await?;
            for conn in loc_connections {
                connections.push(ConnectionData {
                    from_location_id: conn.from_location.to_string(),
                    to_location_id: conn.to_location.to_string(),
                    connection_type: conn.connection_type.clone(),
                    description: conn.description.clone(),
                    bidirectional: conn.bidirectional,
                    travel_time: conn.travel_time,
                    is_locked: conn.is_locked,
                    lock_description: conn.lock_description.clone(),
                });
            }
        }

        // Deduplicate connections (bidirectional creates two entries)
        connections.sort_by(|a, b| {
            (&a.from_location_id, &a.to_location_id).cmp(&(&b.from_location_id, &b.to_location_id))
        });
        connections.dedup_by(|a, b| {
            a.from_location_id == b.from_location_id && a.to_location_id == b.to_location_id
        });

        // Build the snapshot
        let snapshot = WorldSnapshot {
            metadata: SnapshotMetadata {
                version: "1.0".to_string(),
                exported_at: chrono::Utc::now().to_rfc3339(),
                engine_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            world: WorldData {
                id: world.id.to_string(),
                name: world.name,
                description: world.description,
                rule_system: RuleSystemConfigDto::from(world.rule_system),
                created_at: world.created_at.to_rfc3339(),
                updated_at: world.updated_at.to_rfc3339(),
            },
            acts: acts
                .into_iter()
                .map(|a| ActData {
                    id: a.id.to_string(),
                    world_id: a.world_id.to_string(),
                    name: a.name,
                    stage: format!("{:?}", a.stage),
                    description: a.description,
                    order: a.order,
                })
                .collect(),
            scenes: scenes
                .into_iter()
                .map(|s| SceneData {
                    id: s.id.to_string(),
                    act_id: s.act_id.to_string(),
                    name: s.name,
                    location_id: s.location_id.to_string(),
                    time_context: format!("{:?}", s.time_context),
                    backdrop_override: s.backdrop_override,
                    featured_characters: s
                        .featured_characters
                        .iter()
                        .map(|c| c.to_string())
                        .collect(),
                    directorial_notes: s.directorial_notes,
                    entry_conditions: s
                        .entry_conditions
                        .iter()
                        .map(|c| format!("{:?}", c))
                        .collect(),
                    order: s.order,
                })
                .collect(),
            characters: characters
                .into_iter()
                .map(|c| CharacterData {
                    id: c.id.to_string(),
                    world_id: c.world_id.to_string(),
                    name: c.name,
                    description: c.description,
                    base_archetype: format!("{:?}", c.base_archetype),
                    current_archetype: format!("{:?}", c.current_archetype),
                    sprite_asset: c.sprite_asset,
                    portrait_asset: c.portrait_asset,
                    is_alive: c.is_alive,
                    is_active: c.is_active,
                    stats: serde_json::json!({
                        "stats": c.stats.stats,
                        "current_hp": c.stats.current_hp,
                        "max_hp": c.stats.max_hp,
                    }),
                })
                .collect(),
            locations: locations
                .into_iter()
                .map(|l| LocationData {
                    id: l.id.to_string(),
                    world_id: l.world_id.to_string(),
                    name: l.name,
                    description: l.description,
                    location_type: format!("{:?}", l.location_type),
                    backdrop_asset: l.backdrop_asset,
                    atmosphere: l.atmosphere,
                })
                .collect(),
            relationships: social_network
                .relationships
                .into_iter()
                .map(|e| RelationshipData {
                    id: format!("{}-{}", e.from_id, e.to_id), // Generated ID for edge
                    from_character_id: e.from_id,
                    to_character_id: e.to_id,
                    relationship_type: e.relationship_type,
                    sentiment: e.sentiment,
                    known_to_player: true, // Default to known, SocialEdge doesn't track this
                })
                .collect(),
            connections,
        };

        Ok(snapshot)
    }

    /// Export world to JSON string
    pub async fn export_to_json(&self, world_id: WorldId) -> Result<String> {
        let snapshot = self.export_world(world_id).await?;
        Ok(serde_json::to_string_pretty(&snapshot)?)
    }

    /// Export world to compressed JSON (minified)
    pub async fn export_to_json_compressed(&self, world_id: WorldId) -> Result<String> {
        let snapshot = self.export_world(world_id).await?;
        Ok(serde_json::to_string(&snapshot)?)
    }
}
