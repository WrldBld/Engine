//! Location repository implementation for Neo4j

use anyhow::Result;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::domain::entities::{
    BackdropRegion, Location, LocationConnection, LocationType, SpatialRelationship,
};
use crate::domain::value_objects::{GridMapId, LocationId, WorldId};

/// Repository for Location operations
pub struct Neo4jLocationRepository {
    connection: Neo4jConnection,
}

impl Neo4jLocationRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new location
    pub async fn create(&self, location: &Location) -> Result<()> {
        let backdrop_regions_json = serde_json::to_string(&location.backdrop_regions)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (l:Location {
                id: $id,
                world_id: $world_id,
                parent_id: $parent_id,
                name: $name,
                description: $description,
                location_type: $location_type,
                backdrop_asset: $backdrop_asset,
                grid_map_id: $grid_map_id,
                backdrop_regions: $backdrop_regions
            })
            CREATE (w)-[:CONTAINS_LOCATION]->(l)
            RETURN l.id as id",
        )
        .param("id", location.id.to_string())
        .param("world_id", location.world_id.to_string())
        .param(
            "parent_id",
            location
                .parent_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        )
        .param("name", location.name.clone())
        .param("description", location.description.clone())
        .param("location_type", format!("{:?}", location.location_type))
        .param(
            "backdrop_asset",
            location.backdrop_asset.clone().unwrap_or_default(),
        )
        .param(
            "grid_map_id",
            location
                .grid_map_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        )
        .param("backdrop_regions", backdrop_regions_json);

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
        let backdrop_regions_json = serde_json::to_string(&location.backdrop_regions)?;

        let q = query(
            "MATCH (l:Location {id: $id})
            SET l.name = $name,
                l.description = $description,
                l.parent_id = $parent_id,
                l.location_type = $location_type,
                l.backdrop_asset = $backdrop_asset,
                l.grid_map_id = $grid_map_id,
                l.backdrop_regions = $backdrop_regions
            RETURN l.id as id",
        )
        .param("id", location.id.to_string())
        .param("name", location.name.clone())
        .param("description", location.description.clone())
        .param(
            "parent_id",
            location
                .parent_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        )
        .param("location_type", format!("{:?}", location.location_type))
        .param(
            "backdrop_asset",
            location.backdrop_asset.clone().unwrap_or_default(),
        )
        .param(
            "grid_map_id",
            location
                .grid_map_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        )
        .param("backdrop_regions", backdrop_regions_json);

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

    /// Create a connection between two locations
    pub async fn create_connection(&self, connection: &LocationConnection) -> Result<()> {
        let requirements_json = serde_json::to_string(&connection.requirements)?;
        let connection_type_str = format!("{:?}", connection.connection_type);

        let q = query(
            "MATCH (from:Location {id: $from_id})
            MATCH (to:Location {id: $to_id})
            CREATE (from)-[c:CONNECTS_TO {
                connection_type: $connection_type,
                description: $description,
                bidirectional: $bidirectional,
                requirements: $requirements,
                travel_time: $travel_time
            }]->(to)
            RETURN from.id as from_id",
        )
        .param("from_id", connection.from_location.to_string())
        .param("to_id", connection.to_location.to_string())
        .param("connection_type", connection_type_str.clone())
        .param("description", connection.description.clone())
        .param("bidirectional", connection.bidirectional)
        .param("requirements", requirements_json.clone())
        .param("travel_time", connection.travel_time.unwrap_or(0) as i64);

        self.connection.graph().run(q).await?;

        // If bidirectional, create the reverse connection too
        if connection.bidirectional {
            // For bidirectional, reverse the connection type if applicable
            let reverse_type = match connection.connection_type {
                SpatialRelationship::Enters => "Exits",
                SpatialRelationship::Exits => "Enters",
                _ => &connection_type_str,
            };

            let reverse_q = query(
                "MATCH (from:Location {id: $from_id})
                MATCH (to:Location {id: $to_id})
                CREATE (to)-[c:CONNECTS_TO {
                    connection_type: $connection_type,
                    description: $description,
                    bidirectional: $bidirectional,
                    requirements: $requirements,
                    travel_time: $travel_time
                }]->(from)
                RETURN to.id as to_id",
            )
            .param("from_id", connection.from_location.to_string())
            .param("to_id", connection.to_location.to_string())
            .param("connection_type", reverse_type)
            .param("description", connection.description.clone())
            .param("bidirectional", connection.bidirectional)
            .param("requirements", requirements_json)
            .param("travel_time", connection.travel_time.unwrap_or(0) as i64);

            self.connection.graph().run(reverse_q).await?;
        }

        tracing::debug!(
            "Created connection from {} to {}",
            connection.from_location,
            connection.to_location
        );
        Ok(())
    }

    /// Get all connections from a location
    pub async fn get_connections(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<LocationConnection>> {
        let q = query(
            "MATCH (from:Location {id: $id})-[c:CONNECTS_TO]->(to:Location)
            RETURN from.id as from_id, to.id as to_id,
                   c.connection_type as connection_type, c.description as description,
                   c.bidirectional as bidirectional, c.requirements as requirements,
                   c.travel_time as travel_time",
        )
        .param("id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut connections = Vec::new();

        while let Some(row) = result.next().await? {
            connections.push(row_to_connection(row)?);
        }

        Ok(connections)
    }

    /// Delete a connection between locations
    pub async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()> {
        let q = query(
            "MATCH (from:Location {id: $from_id})-[c:CONNECTS_TO]->(to:Location {id: $to_id})
            DELETE c",
        )
        .param("from_id", from.to_string())
        .param("to_id", to.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted connection from {} to {}", from, to);
        Ok(())
    }
}

fn row_to_location(row: Row) -> Result<Location> {
    let node: neo4rs::Node = row.get("l")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let parent_id_str: String = node.get("parent_id").unwrap_or_default();
    let name: String = node.get("name")?;
    let description: String = node.get("description")?;
    let location_type_str: String = node.get("location_type")?;
    let backdrop_asset: String = node.get("backdrop_asset")?;
    let grid_map_id_str: String = node.get("grid_map_id")?;
    let backdrop_regions_json: String = node
        .get("backdrop_regions")
        .unwrap_or_else(|_| "[]".to_string());

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = uuid::Uuid::parse_str(&world_id_str)?;

    let parent_id = if parent_id_str.is_empty() {
        None
    } else {
        uuid::Uuid::parse_str(&parent_id_str)
            .ok()
            .map(LocationId::from_uuid)
    };

    let location_type = match location_type_str.as_str() {
        "Interior" => LocationType::Interior,
        "Exterior" => LocationType::Exterior,
        "Abstract" => LocationType::Abstract,
        _ => LocationType::Interior,
    };

    let grid_map_id = if grid_map_id_str.is_empty() {
        None
    } else {
        uuid::Uuid::parse_str(&grid_map_id_str)
            .ok()
            .map(GridMapId::from_uuid)
    };

    let backdrop_regions: Vec<BackdropRegion> =
        serde_json::from_str(&backdrop_regions_json).unwrap_or_default();

    Ok(Location {
        id: LocationId::from_uuid(id),
        world_id: WorldId::from_uuid(world_id),
        parent_id,
        name,
        description,
        location_type,
        backdrop_asset: if backdrop_asset.is_empty() {
            None
        } else {
            Some(backdrop_asset)
        },
        grid_map_id,
        backdrop_regions,
    })
}

fn row_to_connection(row: Row) -> Result<LocationConnection> {
    let from_id_str: String = row.get("from_id")?;
    let to_id_str: String = row.get("to_id")?;
    let connection_type_str: String = row
        .get("connection_type")
        .unwrap_or_else(|_| "ConnectsTo".to_string());
    let description: String = row.get("description")?;
    let bidirectional: bool = row.get("bidirectional")?;
    let requirements_json: String = row.get("requirements")?;
    let travel_time: i64 = row.get("travel_time")?;

    let from_id = uuid::Uuid::parse_str(&from_id_str)?;
    let to_id = uuid::Uuid::parse_str(&to_id_str)?;
    let requirements = serde_json::from_str(&requirements_json)?;

    let connection_type = match connection_type_str.as_str() {
        "Enters" => SpatialRelationship::Enters,
        "Exits" => SpatialRelationship::Exits,
        "LeadsTo" => SpatialRelationship::LeadsTo,
        "AdjacentTo" => SpatialRelationship::AdjacentTo,
        _ => SpatialRelationship::ConnectsTo,
    };

    Ok(LocationConnection {
        from_location: LocationId::from_uuid(from_id),
        to_location: LocationId::from_uuid(to_id),
        connection_type,
        description,
        bidirectional,
        requirements,
        travel_time: if travel_time == 0 {
            None
        } else {
            Some(travel_time as u32)
        },
    })
}
