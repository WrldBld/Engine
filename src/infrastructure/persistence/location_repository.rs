//! Location repository implementation for Neo4j
//!
//! Uses Neo4j edges for:
//! - CONTAINS_LOCATION: Parent-child hierarchy
//! - CONNECTED_TO: Navigation connections
//! - HAS_REGION: Regions (see region_repository.rs)
//! - HAS_TACTICAL_MAP: Grid maps

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::LocationRepositoryPort;
use crate::domain::entities::{Location, LocationConnection, LocationType, MapBounds, Region};
use crate::domain::value_objects::{GridMapId, LocationId, RegionId, WorldId};

/// Repository for Location operations
pub struct Neo4jLocationRepository {
    connection: Neo4jConnection,
}

impl Neo4jLocationRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    // =========================================================================
    // Core CRUD
    // =========================================================================

    /// Create a new location
    pub async fn create(&self, location: &Location) -> Result<()> {
        // Serialize map_bounds as JSON if present
        let map_bounds_json = location
            .parent_map_bounds
            .as_ref()
            .map(|b| serde_json::json!({
                "x": b.x,
                "y": b.y,
                "width": b.width,
                "height": b.height
            }).to_string())
            .unwrap_or_default();

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (l:Location {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                location_type: $location_type,
                backdrop_asset: $backdrop_asset,
                map_asset: $map_asset,
                parent_map_bounds: $parent_map_bounds,
                default_region_id: $default_region_id,
                atmosphere: $atmosphere
            })
            CREATE (w)-[:CONTAINS_LOCATION]->(l)
            RETURN l.id as id",
        )
        .param("id", location.id.to_string())
        .param("world_id", location.world_id.to_string())
        .param("name", location.name.clone())
        .param("description", location.description.clone())
        .param("location_type", format!("{:?}", location.location_type))
        .param(
            "backdrop_asset",
            location.backdrop_asset.clone().unwrap_or_default(),
        )
        .param(
            "map_asset",
            location.map_asset.clone().unwrap_or_default(),
        )
        .param("parent_map_bounds", map_bounds_json)
        .param(
            "default_region_id",
            location.default_region_id.map(|id| id.to_string()).unwrap_or_default(),
        )
        .param(
            "atmosphere",
            location.atmosphere.clone().unwrap_or_default(),
        );

        self.connection.graph().run(q).await?;
        tracing::debug!("Created location: {}", location.name);
        Ok(())
    }

    /// Get a location by ID
    pub async fn get(&self, id: LocationId) -> Result<Option<Location>> {
        let q = query(
            "MATCH (l:Location {id: $id})
            RETURN l",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_location(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all locations in a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Location>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_LOCATION]->(l:Location)
            RETURN l
            ORDER BY l.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut locations = Vec::new();

        while let Some(row) = result.next().await? {
            locations.push(row_to_location(row)?);
        }

        Ok(locations)
    }

    /// Update a location
    pub async fn update(&self, location: &Location) -> Result<()> {
        // Serialize map_bounds as JSON if present
        let map_bounds_json = location
            .parent_map_bounds
            .as_ref()
            .map(|b| serde_json::json!({
                "x": b.x,
                "y": b.y,
                "width": b.width,
                "height": b.height
            }).to_string())
            .unwrap_or_default();

        let q = query(
            "MATCH (l:Location {id: $id})
            SET l.name = $name,
                l.description = $description,
                l.location_type = $location_type,
                l.backdrop_asset = $backdrop_asset,
                l.map_asset = $map_asset,
                l.parent_map_bounds = $parent_map_bounds,
                l.default_region_id = $default_region_id,
                l.atmosphere = $atmosphere
            RETURN l.id as id",
        )
        .param("id", location.id.to_string())
        .param("name", location.name.clone())
        .param("description", location.description.clone())
        .param("location_type", format!("{:?}", location.location_type))
        .param(
            "backdrop_asset",
            location.backdrop_asset.clone().unwrap_or_default(),
        )
        .param(
            "map_asset",
            location.map_asset.clone().unwrap_or_default(),
        )
        .param("parent_map_bounds", map_bounds_json)
        .param(
            "default_region_id",
            location.default_region_id.map(|id| id.to_string()).unwrap_or_default(),
        )
        .param(
            "atmosphere",
            location.atmosphere.clone().unwrap_or_default(),
        );

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated location: {}", location.name);
        Ok(())
    }

    /// Delete a location
    pub async fn delete(&self, id: LocationId) -> Result<()> {
        let q = query(
            "MATCH (l:Location {id: $id})
            DETACH DELETE l",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted location: {}", id);
        Ok(())
    }

    // =========================================================================
    // Location Hierarchy (CONTAINS_LOCATION edges)
    // =========================================================================

    /// Set a location's parent (creates CONTAINS_LOCATION edge)
    pub async fn set_parent(&self, child_id: LocationId, parent_id: LocationId) -> Result<()> {
        // First remove any existing parent edge
        let remove_q = query(
            "MATCH (parent:Location)-[r:CONTAINS_LOCATION]->(child:Location {id: $child_id})
            DELETE r",
        )
        .param("child_id", child_id.to_string());
        self.connection.graph().run(remove_q).await?;

        // Then create the new parent edge
        let create_q = query(
            "MATCH (parent:Location {id: $parent_id})
            MATCH (child:Location {id: $child_id})
            CREATE (parent)-[:CONTAINS_LOCATION]->(child)
            RETURN parent.id as parent_id",
        )
        .param("parent_id", parent_id.to_string())
        .param("child_id", child_id.to_string());

        self.connection.graph().run(create_q).await?;
        tracing::debug!("Set parent {} for location {}", parent_id, child_id);
        Ok(())
    }

    /// Remove a location's parent (deletes CONTAINS_LOCATION edge from parent)
    pub async fn remove_parent(&self, child_id: LocationId) -> Result<()> {
        let q = query(
            "MATCH (parent:Location)-[r:CONTAINS_LOCATION]->(child:Location {id: $child_id})
            DELETE r",
        )
        .param("child_id", child_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Removed parent for location {}", child_id);
        Ok(())
    }

    /// Get a location's parent
    pub async fn get_parent(&self, location_id: LocationId) -> Result<Option<Location>> {
        let q = query(
            "MATCH (parent:Location)-[:CONTAINS_LOCATION]->(child:Location {id: $id})
            RETURN parent as l",
        )
        .param("id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_location(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get all child locations of a parent
    pub async fn get_children(&self, parent_id: LocationId) -> Result<Vec<Location>> {
        let q = query(
            "MATCH (parent:Location {id: $id})-[:CONTAINS_LOCATION]->(child:Location)
            RETURN child as l
            ORDER BY child.name",
        )
        .param("id", parent_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut locations = Vec::new();

        while let Some(row) = result.next().await? {
            locations.push(row_to_location(row)?);
        }

        Ok(locations)
    }

    // =========================================================================
    // Location Connections (CONNECTED_TO edges)
    // =========================================================================

    /// Create a connection between two locations
    pub async fn create_connection(&self, connection: &LocationConnection) -> Result<()> {
        let q = query(
            "MATCH (from:Location {id: $from_id})
            MATCH (to:Location {id: $to_id})
            CREATE (from)-[:CONNECTED_TO {
                connection_type: $connection_type,
                description: $description,
                bidirectional: $bidirectional,
                travel_time: $travel_time,
                is_locked: $is_locked,
                lock_description: $lock_description
            }]->(to)
            RETURN from.id as from_id",
        )
        .param("from_id", connection.from_location.to_string())
        .param("to_id", connection.to_location.to_string())
        .param("connection_type", connection.connection_type.clone())
        .param(
            "description",
            connection.description.clone().unwrap_or_default(),
        )
        .param("bidirectional", connection.bidirectional)
        .param("travel_time", connection.travel_time as i64)
        .param("is_locked", connection.is_locked)
        .param(
            "lock_description",
            connection.lock_description.clone().unwrap_or_default(),
        );

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Created connection from {} to {}",
            connection.from_location,
            connection.to_location
        );
        Ok(())
    }

    /// Get all connections from a location
    pub async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>> {
        let q = query(
            "MATCH (from:Location {id: $id})-[r:CONNECTED_TO]->(to:Location)
            RETURN from.id as from_id, to.id as to_id, 
                   r.connection_type as connection_type,
                   r.description as description,
                   r.bidirectional as bidirectional,
                   r.travel_time as travel_time,
                   r.is_locked as is_locked,
                   r.lock_description as lock_description",
        )
        .param("id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut connections = Vec::new();

        while let Some(row) = result.next().await? {
            connections.push(row_to_connection(row)?);
        }

        Ok(connections)
    }

    /// Update a connection between two locations
    pub async fn update_connection(&self, connection: &LocationConnection) -> Result<()> {
        let q = query(
            "MATCH (from:Location {id: $from_id})-[r:CONNECTED_TO]->(to:Location {id: $to_id})
            SET r.connection_type = $connection_type,
                r.description = $description,
                r.bidirectional = $bidirectional,
                r.travel_time = $travel_time,
                r.is_locked = $is_locked,
                r.lock_description = $lock_description
            RETURN from.id as from_id",
        )
        .param("from_id", connection.from_location.to_string())
        .param("to_id", connection.to_location.to_string())
        .param("connection_type", connection.connection_type.clone())
        .param(
            "description",
            connection.description.clone().unwrap_or_default(),
        )
        .param("bidirectional", connection.bidirectional)
        .param("travel_time", connection.travel_time as i64)
        .param("is_locked", connection.is_locked)
        .param(
            "lock_description",
            connection.lock_description.clone().unwrap_or_default(),
        );

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Updated connection from {} to {}",
            connection.from_location,
            connection.to_location
        );
        Ok(())
    }

    /// Delete a connection between two locations
    pub async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()> {
        let q = query(
            "MATCH (from:Location {id: $from_id})-[r:CONNECTED_TO]->(to:Location {id: $to_id})
            DELETE r",
        )
        .param("from_id", from.to_string())
        .param("to_id", to.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted connection from {} to {}", from, to);
        Ok(())
    }

    /// Unlock a connection
    pub async fn unlock_connection(&self, from: LocationId, to: LocationId) -> Result<()> {
        let q = query(
            "MATCH (from:Location {id: $from_id})-[r:CONNECTED_TO]->(to:Location {id: $to_id})
            SET r.is_locked = false, r.lock_description = ''
            RETURN from.id as from_id",
        )
        .param("from_id", from.to_string())
        .param("to_id", to.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Unlocked connection from {} to {}", from, to);
        Ok(())
    }

    // =========================================================================
    // Grid Map (HAS_TACTICAL_MAP edge)
    // =========================================================================

    /// Set a location's tactical map
    pub async fn set_grid_map(&self, location_id: LocationId, grid_map_id: GridMapId) -> Result<()> {
        // First remove any existing grid map edge
        let remove_q = query(
            "MATCH (l:Location {id: $location_id})-[r:HAS_TACTICAL_MAP]->()
            DELETE r",
        )
        .param("location_id", location_id.to_string());
        self.connection.graph().run(remove_q).await?;

        // Then create the new edge
        let create_q = query(
            "MATCH (l:Location {id: $location_id})
            MATCH (g:GridMap {id: $grid_map_id})
            CREATE (l)-[:HAS_TACTICAL_MAP]->(g)
            RETURN l.id as location_id",
        )
        .param("location_id", location_id.to_string())
        .param("grid_map_id", grid_map_id.to_string());

        self.connection.graph().run(create_q).await?;
        tracing::debug!("Set grid map {} for location {}", grid_map_id, location_id);
        Ok(())
    }

    /// Remove a location's tactical map
    pub async fn remove_grid_map(&self, location_id: LocationId) -> Result<()> {
        let q = query(
            "MATCH (l:Location {id: $location_id})-[r:HAS_TACTICAL_MAP]->()
            DELETE r",
        )
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Removed grid map from location {}", location_id);
        Ok(())
    }

    /// Get a location's tactical map ID
    pub async fn get_grid_map_id(&self, location_id: LocationId) -> Result<Option<GridMapId>> {
        let q = query(
            "MATCH (l:Location {id: $location_id})-[:HAS_TACTICAL_MAP]->(g:GridMap)
            RETURN g.id as grid_map_id",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let id_str: String = row.get("grid_map_id")?;
            let id = uuid::Uuid::parse_str(&id_str)?;
            Ok(Some(GridMapId::from_uuid(id)))
        } else {
            Ok(None)
        }
    }

    // =========================================================================
    // Regions (HAS_REGION edges)
    // =========================================================================

    /// Create a region within a location
    pub async fn create_region(&self, location_id: LocationId, region: &Region) -> Result<()> {
        // Serialize map_bounds as JSON if present
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
        .param("location_id", location_id.to_string())
        .param("id", region.id.to_string())
        .param("name", region.name.clone())
        .param("description", region.description.clone())
        .param("backdrop_asset", region.backdrop_asset.clone().unwrap_or_default())
        .param("atmosphere", region.atmosphere.clone().unwrap_or_default())
        .param("map_bounds", map_bounds_json)
        .param("is_spawn_point", region.is_spawn_point)
        .param("order", region.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created region {} in location {}", region.id, location_id);
        Ok(())
    }

    /// Get all regions in a location
    pub async fn get_regions(&self, location_id: LocationId) -> Result<Vec<Region>> {
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
}

// ============================================================================
// Row conversion helpers
// ============================================================================

fn row_to_location(row: Row) -> Result<Location> {
    let node: neo4rs::Node = row.get("l")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description")?;
    let location_type_str: String = node.get("location_type")?;
    let backdrop_asset: String = node.get("backdrop_asset").unwrap_or_default();
    let map_asset: String = node.get("map_asset").unwrap_or_default();
    let parent_map_bounds_json: String = node.get("parent_map_bounds").unwrap_or_default();
    let default_region_id_str: String = node.get("default_region_id").unwrap_or_default();
    let atmosphere: String = node.get("atmosphere").unwrap_or_default();

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = uuid::Uuid::parse_str(&world_id_str)?;

    let location_type = match location_type_str.as_str() {
        "Interior" => LocationType::Interior,
        "Exterior" => LocationType::Exterior,
        "Abstract" => LocationType::Abstract,
        _ => LocationType::Interior,
    };

    // Parse parent_map_bounds from JSON
    let parent_map_bounds = if parent_map_bounds_json.is_empty() {
        None
    } else {
        serde_json::from_str::<serde_json::Value>(&parent_map_bounds_json)
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

    // Parse default_region_id
    let default_region_id = if default_region_id_str.is_empty() {
        None
    } else {
        uuid::Uuid::parse_str(&default_region_id_str)
            .ok()
            .map(RegionId::from_uuid)
    };

    Ok(Location {
        id: LocationId::from_uuid(id),
        world_id: WorldId::from_uuid(world_id),
        name,
        description,
        location_type,
        backdrop_asset: if backdrop_asset.is_empty() {
            None
        } else {
            Some(backdrop_asset)
        },
        map_asset: if map_asset.is_empty() {
            None
        } else {
            Some(map_asset)
        },
        parent_map_bounds,
        default_region_id,
        atmosphere: if atmosphere.is_empty() {
            None
        } else {
            Some(atmosphere)
        },
    })
}

fn row_to_connection(row: Row) -> Result<LocationConnection> {
    let from_id_str: String = row.get("from_id")?;
    let to_id_str: String = row.get("to_id")?;
    let connection_type: String = row.get("connection_type")?;
    let description: String = row.get("description").unwrap_or_default();
    let bidirectional: bool = row.get("bidirectional")?;
    let travel_time: i64 = row.get("travel_time")?;
    let is_locked: bool = row.get("is_locked").unwrap_or(false);
    let lock_description: String = row.get("lock_description").unwrap_or_default();

    let from_id = uuid::Uuid::parse_str(&from_id_str)?;
    let to_id = uuid::Uuid::parse_str(&to_id_str)?;

    Ok(LocationConnection {
        from_location: LocationId::from_uuid(from_id),
        to_location: LocationId::from_uuid(to_id),
        connection_type,
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
        bidirectional,
        travel_time: travel_time as u32,
        is_locked,
        lock_description: if lock_description.is_empty() {
            None
        } else {
            Some(lock_description)
        },
    })
}

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

// =============================================================================
// LocationRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl LocationRepositoryPort for Neo4jLocationRepository {
    async fn create(&self, location: &Location) -> Result<()> {
        Neo4jLocationRepository::create(self, location).await
    }

    async fn get(&self, id: LocationId) -> Result<Option<Location>> {
        Neo4jLocationRepository::get(self, id).await
    }

    async fn list(&self, world_id: WorldId) -> Result<Vec<Location>> {
        Neo4jLocationRepository::list_by_world(self, world_id).await
    }

    async fn update(&self, location: &Location) -> Result<()> {
        Neo4jLocationRepository::update(self, location).await
    }

    async fn delete(&self, id: LocationId) -> Result<()> {
        Neo4jLocationRepository::delete(self, id).await
    }

    async fn set_parent(&self, child_id: LocationId, parent_id: LocationId) -> Result<()> {
        Neo4jLocationRepository::set_parent(self, child_id, parent_id).await
    }

    async fn remove_parent(&self, child_id: LocationId) -> Result<()> {
        Neo4jLocationRepository::remove_parent(self, child_id).await
    }

    async fn get_parent(&self, location_id: LocationId) -> Result<Option<Location>> {
        Neo4jLocationRepository::get_parent(self, location_id).await
    }

    async fn get_children(&self, location_id: LocationId) -> Result<Vec<Location>> {
        Neo4jLocationRepository::get_children(self, location_id).await
    }

    async fn create_connection(&self, connection: &LocationConnection) -> Result<()> {
        Neo4jLocationRepository::create_connection(self, connection).await
    }

    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>> {
        Neo4jLocationRepository::get_connections(self, location_id).await
    }

    async fn update_connection(&self, connection: &LocationConnection) -> Result<()> {
        Neo4jLocationRepository::update_connection(self, connection).await
    }

    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()> {
        Neo4jLocationRepository::delete_connection(self, from, to).await
    }

    async fn unlock_connection(&self, from: LocationId, to: LocationId) -> Result<()> {
        Neo4jLocationRepository::unlock_connection(self, from, to).await
    }

    async fn set_grid_map(&self, location_id: LocationId, grid_map_id: GridMapId) -> Result<()> {
        Neo4jLocationRepository::set_grid_map(self, location_id, grid_map_id).await
    }

    async fn remove_grid_map(&self, location_id: LocationId) -> Result<()> {
        Neo4jLocationRepository::remove_grid_map(self, location_id).await
    }

    async fn get_grid_map_id(&self, location_id: LocationId) -> Result<Option<GridMapId>> {
        Neo4jLocationRepository::get_grid_map_id(self, location_id).await
    }

    async fn create_region(&self, location_id: LocationId, region: &Region) -> Result<()> {
        Neo4jLocationRepository::create_region(self, location_id, region).await
    }

    async fn get_regions(&self, location_id: LocationId) -> Result<Vec<Region>> {
        Neo4jLocationRepository::get_regions(self, location_id).await
    }
}
