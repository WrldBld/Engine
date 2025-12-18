//! Region repository implementation for Neo4j
//!
//! Regions are sub-areas within a Location, each with their own backdrop.
//! Think of them as "screens" in a JRPG exploration system.
//!
//! Neo4j relationships:
//! - (Location)-[:HAS_REGION]->(Region) - Containment
//! - (Region)-[:CONNECTED_TO_REGION]->(Region) - Internal navigation
//! - (Region)-[:EXITS_TO_LOCATION {arrival_region_id}]->(Location) - Exit to another location

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::application::dto::parse_archetype;
use crate::application::ports::outbound::RegionRepositoryPort;
use crate::domain::entities::{Character, MapBounds, Region, RegionConnection, RegionExit, StatBlock};
use crate::domain::value_objects::{
    CharacterId, LocationId, RegionFrequency, RegionId, RegionRelationshipType, RegionShift, WorldId,
};

/// Repository for Region operations
pub struct Neo4jRegionRepository {
    connection: Neo4jConnection,
}

impl Neo4jRegionRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    // =========================================================================
    // Core CRUD
    // =========================================================================

    /// Create a new region within a location
    pub async fn create(&self, region: &Region) -> Result<()> {
        let map_bounds_json = region
            .map_bounds
            .as_ref()
            .map(|b| serde_json::json!({
                "x": b.x,
                "y": b.y,
                "width": b.width,
                "height": b.height
            }).to_string())
            .unwrap_or_default();

        let q = query(
            "MATCH (l:Location {id: $location_id})
            CREATE (r:Region {
                id: $id,
                location_id: $location_id,
                name: $name,
                description: $description,
                backdrop_asset: $backdrop_asset,
                atmosphere: $atmosphere,
                map_bounds: $map_bounds,
                is_spawn_point: $is_spawn_point,
                order: $order
            })
            CREATE (l)-[:HAS_REGION]->(r)
            RETURN r.id as id",
        )
        .param("location_id", region.location_id.to_string())
        .param("id", region.id.to_string())
        .param("name", region.name.clone())
        .param("description", region.description.clone())
        .param("backdrop_asset", region.backdrop_asset.clone().unwrap_or_default())
        .param("atmosphere", region.atmosphere.clone().unwrap_or_default())
        .param("map_bounds", map_bounds_json)
        .param("is_spawn_point", region.is_spawn_point)
        .param("order", region.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created region {} in location {}", region.id, region.location_id);
        Ok(())
    }

    /// Get a region by ID
    pub async fn get(&self, id: RegionId) -> Result<Option<Region>> {
        let q = query(
            "MATCH (r:Region {id: $id})
            RETURN r",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_region(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all regions in a location
    pub async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Region>> {
        let q = query(
            "MATCH (l:Location {id: $location_id})-[:HAS_REGION]->(r:Region)
            RETURN r
            ORDER BY r.order",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut regions = Vec::new();

        while let Some(row) = result.next().await? {
            regions.push(row_to_region(row)?);
        }

        Ok(regions)
    }

    /// List all spawn point regions in a world
    pub async fn list_spawn_points(&self, world_id: WorldId) -> Result<Vec<Region>> {
        let q = query(
            "MATCH (w:World {id: $world_id})<-[:BELONGS_TO]-(l:Location)-[:HAS_REGION]->(r:Region)
            WHERE r.is_spawn_point = true
            RETURN r
            ORDER BY l.name, r.order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut regions = Vec::new();

        while let Some(row) = result.next().await? {
            regions.push(row_to_region(row)?);
        }

        Ok(regions)
    }

    /// Update a region
    pub async fn update(&self, region: &Region) -> Result<()> {
        let map_bounds_json = region
            .map_bounds
            .as_ref()
            .map(|b| serde_json::json!({
                "x": b.x,
                "y": b.y,
                "width": b.width,
                "height": b.height
            }).to_string())
            .unwrap_or_default();

        let q = query(
            "MATCH (r:Region {id: $id})
            SET r.name = $name,
                r.description = $description,
                r.backdrop_asset = $backdrop_asset,
                r.atmosphere = $atmosphere,
                r.map_bounds = $map_bounds,
                r.is_spawn_point = $is_spawn_point,
                r.order = $order
            RETURN r.id as id",
        )
        .param("id", region.id.to_string())
        .param("name", region.name.clone())
        .param("description", region.description.clone())
        .param("backdrop_asset", region.backdrop_asset.clone().unwrap_or_default())
        .param("atmosphere", region.atmosphere.clone().unwrap_or_default())
        .param("map_bounds", map_bounds_json)
        .param("is_spawn_point", region.is_spawn_point)
        .param("order", region.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated region {}", region.id);
        Ok(())
    }

    /// Delete a region
    pub async fn delete(&self, id: RegionId) -> Result<()> {
        let q = query(
            "MATCH (r:Region {id: $id})
            DETACH DELETE r",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted region {}", id);
        Ok(())
    }

    // =========================================================================
    // Region Connections (CONNECTED_TO_REGION edges)
    // =========================================================================

    /// Create a connection between two regions
    pub async fn create_connection(&self, connection: &RegionConnection) -> Result<()> {
        let q = query(
            "MATCH (from:Region {id: $from_id})
            MATCH (to:Region {id: $to_id})
            CREATE (from)-[:CONNECTED_TO_REGION {
                description: $description,
                bidirectional: $bidirectional,
                is_locked: $is_locked,
                lock_description: $lock_description
            }]->(to)
            RETURN from.id as from_id",
        )
        .param("from_id", connection.from_region.to_string())
        .param("to_id", connection.to_region.to_string())
        .param("description", connection.description.clone().unwrap_or_default())
        .param("bidirectional", connection.bidirectional)
        .param("is_locked", connection.is_locked)
        .param("lock_description", connection.lock_description.clone().unwrap_or_default());

        self.connection.graph().run(q).await?;

        // If bidirectional, create the reverse edge too
        if connection.bidirectional {
            let reverse_q = query(
                "MATCH (from:Region {id: $from_id})
                MATCH (to:Region {id: $to_id})
                CREATE (to)-[:CONNECTED_TO_REGION {
                    description: $description,
                    bidirectional: $bidirectional,
                    is_locked: $is_locked,
                    lock_description: $lock_description
                }]->(from)
                RETURN to.id as to_id",
            )
            .param("from_id", connection.from_region.to_string())
            .param("to_id", connection.to_region.to_string())
            .param("description", connection.description.clone().unwrap_or_default())
            .param("bidirectional", connection.bidirectional)
            .param("is_locked", connection.is_locked)
            .param("lock_description", connection.lock_description.clone().unwrap_or_default());

            self.connection.graph().run(reverse_q).await?;
        }

        tracing::debug!(
            "Created region connection from {} to {}",
            connection.from_region,
            connection.to_region
        );
        Ok(())
    }

    /// Get all connections from a region
    pub async fn get_connections(&self, region_id: RegionId) -> Result<Vec<RegionConnection>> {
        let q = query(
            "MATCH (from:Region {id: $id})-[rel:CONNECTED_TO_REGION]->(to:Region)
            RETURN from.id as from_id, to.id as to_id,
                   rel.description as description,
                   rel.bidirectional as bidirectional,
                   rel.is_locked as is_locked,
                   rel.lock_description as lock_description",
        )
        .param("id", region_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut connections = Vec::new();

        while let Some(row) = result.next().await? {
            connections.push(row_to_region_connection(row)?);
        }

        Ok(connections)
    }

    /// Delete a connection between regions
    pub async fn delete_connection(&self, from: RegionId, to: RegionId) -> Result<()> {
        let q = query(
            "MATCH (from:Region {id: $from_id})-[rel:CONNECTED_TO_REGION]->(to:Region {id: $to_id})
            DELETE rel",
        )
        .param("from_id", from.to_string())
        .param("to_id", to.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted region connection from {} to {}", from, to);
        Ok(())
    }

    /// Unlock a connection between regions
    pub async fn unlock_connection(&self, from: RegionId, to: RegionId) -> Result<()> {
        let q = query(
            "MATCH (from:Region {id: $from_id})-[rel:CONNECTED_TO_REGION]->(to:Region {id: $to_id})
            SET rel.is_locked = false
            RETURN from.id as from_id",
        )
        .param("from_id", from.to_string())
        .param("to_id", to.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Unlocked region connection from {} to {}", from, to);
        Ok(())
    }

    // =========================================================================
    // Region Exits (EXITS_TO_LOCATION edges)
    // =========================================================================

    /// Create an exit from a region to another location
    pub async fn create_exit(&self, exit: &RegionExit) -> Result<()> {
        let q = query(
            "MATCH (from:Region {id: $from_id})
            MATCH (to:Location {id: $to_location_id})
            CREATE (from)-[:EXITS_TO_LOCATION {
                arrival_region_id: $arrival_region_id,
                description: $description,
                bidirectional: $bidirectional
            }]->(to)
            RETURN from.id as from_id",
        )
        .param("from_id", exit.from_region.to_string())
        .param("to_location_id", exit.to_location.to_string())
        .param("arrival_region_id", exit.arrival_region_id.to_string())
        .param("description", exit.description.clone().unwrap_or_default())
        .param("bidirectional", exit.bidirectional);

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Created exit from region {} to location {}",
            exit.from_region,
            exit.to_location
        );
        Ok(())
    }

    /// Get all exits from a region
    pub async fn get_exits(&self, region_id: RegionId) -> Result<Vec<RegionExit>> {
        let q = query(
            "MATCH (from:Region {id: $id})-[rel:EXITS_TO_LOCATION]->(to:Location)
            RETURN from.id as from_id, to.id as to_location_id,
                   rel.arrival_region_id as arrival_region_id,
                   rel.description as description,
                   rel.bidirectional as bidirectional",
        )
        .param("id", region_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut exits = Vec::new();

        while let Some(row) = result.next().await? {
            exits.push(row_to_region_exit(row)?);
        }

        Ok(exits)
    }

    /// Delete an exit from a region to a location
    pub async fn delete_exit(&self, from_region: RegionId, to_location: LocationId) -> Result<()> {
        let q = query(
            "MATCH (from:Region {id: $from_id})-[rel:EXITS_TO_LOCATION]->(to:Location {id: $to_id})
            DELETE rel",
        )
        .param("from_id", from_region.to_string())
        .param("to_id", to_location.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Deleted exit from region {} to location {}",
            from_region,
            to_location
        );
        Ok(())
    }
}

// =============================================================================
// Row conversion helpers
// =============================================================================

fn row_to_region(row: Row) -> Result<Region> {
    let node: neo4rs::Node = row.get("r")?;

    let id_str: String = node.get("id")?;
    let location_id_str: String = node.get("location_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let backdrop_asset: String = node.get("backdrop_asset").unwrap_or_default();
    let atmosphere: String = node.get("atmosphere").unwrap_or_default();
    let map_bounds_json: String = node.get("map_bounds").unwrap_or_default();
    let is_spawn_point: bool = node.get("is_spawn_point").unwrap_or(false);
    let order: i64 = node.get("order").unwrap_or(0);

    let id = uuid::Uuid::parse_str(&id_str)?;
    let location_id = uuid::Uuid::parse_str(&location_id_str)?;

    // Parse map_bounds from JSON
    let map_bounds = if map_bounds_json.is_empty() {
        None
    } else {
        serde_json::from_str::<serde_json::Value>(&map_bounds_json)
            .ok()
            .and_then(|v| {
                Some(MapBounds {
                    x: v.get("x")?.as_u64()? as u32,
                    y: v.get("y")?.as_u64()? as u32,
                    width: v.get("width")?.as_u64()? as u32,
                    height: v.get("height")?.as_u64()? as u32,
                })
            })
    };

    Ok(Region {
        id: RegionId::from_uuid(id),
        location_id: LocationId::from_uuid(location_id),
        name,
        description,
        backdrop_asset: if backdrop_asset.is_empty() { None } else { Some(backdrop_asset) },
        atmosphere: if atmosphere.is_empty() { None } else { Some(atmosphere) },
        map_bounds,
        is_spawn_point,
        order: order as u32,
    })
}

fn row_to_region_connection(row: Row) -> Result<RegionConnection> {
    let from_id_str: String = row.get("from_id")?;
    let to_id_str: String = row.get("to_id")?;
    let description: String = row.get("description").unwrap_or_default();
    let bidirectional: bool = row.get("bidirectional").unwrap_or(true);
    let is_locked: bool = row.get("is_locked").unwrap_or(false);
    let lock_description: String = row.get("lock_description").unwrap_or_default();

    let from_id = uuid::Uuid::parse_str(&from_id_str)?;
    let to_id = uuid::Uuid::parse_str(&to_id_str)?;

    Ok(RegionConnection {
        from_region: RegionId::from_uuid(from_id),
        to_region: RegionId::from_uuid(to_id),
        description: if description.is_empty() { None } else { Some(description) },
        bidirectional,
        is_locked,
        lock_description: if lock_description.is_empty() { None } else { Some(lock_description) },
    })
}

fn row_to_region_exit(row: Row) -> Result<RegionExit> {
    let from_id_str: String = row.get("from_id")?;
    let to_location_id_str: String = row.get("to_location_id")?;
    let arrival_region_id_str: String = row.get("arrival_region_id")?;
    let description: String = row.get("description").unwrap_or_default();
    let bidirectional: bool = row.get("bidirectional").unwrap_or(true);

    let from_id = uuid::Uuid::parse_str(&from_id_str)?;
    let to_location_id = uuid::Uuid::parse_str(&to_location_id_str)?;
    let arrival_region_id = uuid::Uuid::parse_str(&arrival_region_id_str)?;

    Ok(RegionExit {
        from_region: RegionId::from_uuid(from_id),
        to_location: LocationId::from_uuid(to_location_id),
        arrival_region_id: RegionId::from_uuid(arrival_region_id),
        description: if description.is_empty() { None } else { Some(description) },
        bidirectional,
    })
}

// =============================================================================
// RegionRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl RegionRepositoryPort for Neo4jRegionRepository {
    async fn get(&self, id: RegionId) -> Result<Option<Region>> {
        Neo4jRegionRepository::get(self, id).await
    }

    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Region>> {
        Neo4jRegionRepository::list_by_location(self, location_id).await
    }

    async fn list_spawn_points(&self, world_id: WorldId) -> Result<Vec<Region>> {
        Neo4jRegionRepository::list_spawn_points(self, world_id).await
    }

    async fn get_npcs_related_to_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>> {
        let q = query(
            "MATCH (c:Character)-[r]->(reg:Region {id: $region_id})
            WHERE type(r) IN ['HOME_REGION', 'WORKS_AT_REGION', 'FREQUENTS_REGION', 'AVOIDS_REGION']
            RETURN c, type(r) as rel_type, r.shift as shift, r.frequency as frequency, r.reason as reason",
        )
        .param("region_id", region_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut npcs = Vec::new();

        while let Some(row) = result.next().await? {
            // Extract relationship data first (before consuming row for character)
            let rel_type: String = row.get("rel_type")?;
            let shift_str: String = row.get("shift").unwrap_or_default();
            let freq_str: String = row.get("frequency").unwrap_or_default();
            let reason: String = row.get("reason").unwrap_or_default();

            let relationship_type = match rel_type.as_str() {
                "HOME_REGION" => RegionRelationshipType::Home,
                "WORKS_AT_REGION" => {
                    let shift = shift_str.parse().unwrap_or(RegionShift::Always);
                    RegionRelationshipType::WorksAt { shift }
                }
                "FREQUENTS_REGION" => {
                    let frequency = freq_str.parse().unwrap_or(RegionFrequency::Sometimes);
                    RegionRelationshipType::Frequents { frequency }
                }
                "AVOIDS_REGION" => RegionRelationshipType::Avoids { reason },
                _ => continue,
            };

            let character = row_to_character_for_presence(row)?;
            npcs.push((character, relationship_type));
        }

        Ok(npcs)
    }
}

/// Convert a Neo4j row to a Character (simplified for presence queries)
fn row_to_character_for_presence(row: Row) -> Result<Character> {
    let node: neo4rs::Node = row.get("c")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let sprite_asset: Option<String> = node.get("sprite_asset").ok();
    let portrait_asset: Option<String> = node.get("portrait_asset").ok();
    let base_archetype_str: String = node.get("base_archetype").unwrap_or_default();
    let current_archetype_str: String = node.get("current_archetype").unwrap_or_default();
    let is_alive: bool = node.get("is_alive").unwrap_or(true);
    let is_active: bool = node.get("is_active").unwrap_or(true);

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = uuid::Uuid::parse_str(&world_id_str)?;

    let base_archetype = parse_archetype(&base_archetype_str);
    let current_archetype = if current_archetype_str.is_empty() {
        base_archetype
    } else {
        parse_archetype(&current_archetype_str)
    };

    Ok(Character {
        id: CharacterId::from_uuid(id),
        world_id: WorldId::from_uuid(world_id),
        name,
        description,
        sprite_asset: if sprite_asset.as_ref().map_or(true, |s| s.is_empty()) {
            None
        } else {
            sprite_asset
        },
        portrait_asset: if portrait_asset.as_ref().map_or(true, |s| s.is_empty()) {
            None
        } else {
            portrait_asset
        },
        base_archetype,
        current_archetype,
        archetype_history: vec![],
        stats: StatBlock::default(),
        is_alive,
        is_active,
    })
}
