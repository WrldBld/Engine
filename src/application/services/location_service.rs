//! Location Service - Application service for location management
//!
//! This service provides use case implementations for creating, updating,
//! and managing locations, including hierarchy and connections.

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, info, instrument};

use crate::domain::entities::{
    BackdropRegion, Location, LocationConnection, LocationType, SpatialRelationship,
};
use crate::domain::value_objects::{GridMapId, LocationId, WorldId};
use crate::infrastructure::persistence::Neo4jRepository;

/// Request to create a new location
#[derive(Debug, Clone)]
pub struct CreateLocationRequest {
    pub world_id: WorldId,
    pub name: String,
    pub description: Option<String>,
    pub location_type: LocationType,
    pub parent_id: Option<LocationId>,
    pub backdrop_asset: Option<String>,
    pub grid_map_id: Option<GridMapId>,
    pub backdrop_regions: Vec<BackdropRegion>,
}

/// Request to update an existing location
#[derive(Debug, Clone)]
pub struct UpdateLocationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub location_type: Option<LocationType>,
    pub parent_id: Option<Option<LocationId>>,
    pub backdrop_asset: Option<String>,
    pub grid_map_id: Option<Option<GridMapId>>,
}

/// Request to create a connection between locations
#[derive(Debug, Clone)]
pub struct CreateConnectionRequest {
    pub from_location: LocationId,
    pub to_location: LocationId,
    pub connection_type: SpatialRelationship,
    pub description: Option<String>,
    pub bidirectional: bool,
    pub travel_time: Option<u32>,
}

/// Location with all its connections
#[derive(Debug, Clone)]
pub struct LocationWithConnections {
    pub location: Location,
    pub connections: Vec<LocationConnection>,
}

/// Location hierarchy node
#[derive(Debug, Clone)]
pub struct LocationHierarchy {
    pub location: Location,
    pub children: Vec<LocationHierarchy>,
}

/// Location service trait defining the application use cases
#[async_trait]
pub trait LocationService: Send + Sync {
    /// Create a new location with optional hierarchy support
    async fn create_location(&self, request: CreateLocationRequest) -> Result<Location>;

    /// Get a location by ID
    async fn get_location(&self, id: LocationId) -> Result<Option<Location>>;

    /// Get a location with all its connections
    async fn get_location_with_connections(
        &self,
        id: LocationId,
    ) -> Result<Option<LocationWithConnections>>;

    /// List all locations in a world
    async fn list_locations(&self, world_id: WorldId) -> Result<Vec<Location>>;

    /// List child locations of a parent
    async fn list_children(&self, parent_id: LocationId) -> Result<Vec<Location>>;

    /// Get the location hierarchy for a world (tree structure)
    async fn get_location_hierarchy(&self, world_id: WorldId) -> Result<Vec<LocationHierarchy>>;

    /// Update a location
    async fn update_location(
        &self,
        id: LocationId,
        request: UpdateLocationRequest,
    ) -> Result<Location>;

    /// Delete a location
    async fn delete_location(&self, id: LocationId) -> Result<()>;

    /// Create a connection between two locations
    async fn create_connection(&self, request: CreateConnectionRequest) -> Result<()>;

    /// Delete a connection between locations
    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()>;

    /// Get all connections from a location
    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>>;

    /// Add a backdrop region to a location
    async fn add_backdrop_region(
        &self,
        location_id: LocationId,
        region: BackdropRegion,
    ) -> Result<Location>;

    /// Remove a backdrop region from a location
    async fn remove_backdrop_region(
        &self,
        location_id: LocationId,
        region_id: String,
    ) -> Result<Location>;

    /// Update backdrop asset for a location
    async fn update_backdrop(&self, location_id: LocationId, asset_path: String) -> Result<Location>;

    /// Set the parent of a location (move in hierarchy)
    async fn set_parent(
        &self,
        location_id: LocationId,
        parent_id: Option<LocationId>,
    ) -> Result<Location>;
}

/// Default implementation of LocationService using Neo4j repository
pub struct LocationServiceImpl {
    repository: Neo4jRepository,
}

impl LocationServiceImpl {
    /// Create a new LocationServiceImpl with the given repository
    pub fn new(repository: Neo4jRepository) -> Self {
        Self { repository }
    }

    /// Validate a location creation request
    fn validate_create_request(request: &CreateLocationRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            anyhow::bail!("Location name cannot be empty");
        }
        if request.name.len() > 255 {
            anyhow::bail!("Location name cannot exceed 255 characters");
        }
        if let Some(ref description) = request.description {
            if description.len() > 10000 {
                anyhow::bail!("Location description cannot exceed 10000 characters");
            }
        }
        Ok(())
    }

    /// Validate a location update request
    fn validate_update_request(request: &UpdateLocationRequest) -> Result<()> {
        if let Some(ref name) = request.name {
            if name.trim().is_empty() {
                anyhow::bail!("Location name cannot be empty");
            }
            if name.len() > 255 {
                anyhow::bail!("Location name cannot exceed 255 characters");
            }
        }
        if let Some(ref description) = request.description {
            if description.len() > 10000 {
                anyhow::bail!("Location description cannot exceed 10000 characters");
            }
        }
        Ok(())
    }

    /// Build hierarchy tree from flat list of locations
    fn build_hierarchy(locations: Vec<Location>) -> Vec<LocationHierarchy> {
        use std::collections::HashMap;

        // Group locations by parent_id
        let mut children_map: HashMap<Option<LocationId>, Vec<Location>> = HashMap::new();
        for location in locations {
            children_map
                .entry(location.parent_id)
                .or_default()
                .push(location);
        }

        // Recursive function to build tree
        fn build_tree(
            parent_id: Option<LocationId>,
            children_map: &HashMap<Option<LocationId>, Vec<Location>>,
        ) -> Vec<LocationHierarchy> {
            children_map
                .get(&parent_id)
                .map(|children| {
                    children
                        .iter()
                        .map(|location| LocationHierarchy {
                            location: location.clone(),
                            children: build_tree(Some(location.id), children_map),
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        // Start with root locations (no parent)
        build_tree(None, &children_map)
    }
}

#[async_trait]
impl LocationService for LocationServiceImpl {
    #[instrument(skip(self), fields(world_id = %request.world_id, name = %request.name))]
    async fn create_location(&self, request: CreateLocationRequest) -> Result<Location> {
        Self::validate_create_request(&request)?;

        // Verify the world exists
        let _ = self
            .repository
            .worlds()
            .get(request.world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", request.world_id))?;

        // Verify parent exists if specified
        if let Some(parent_id) = request.parent_id {
            let _ = self
                .repository
                .locations()
                .get(parent_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Parent location not found: {}", parent_id))?;
        }

        let mut location = Location::new(request.world_id, &request.name, request.location_type);

        if let Some(description) = request.description {
            location = location.with_description(description);
        }
        if let Some(parent_id) = request.parent_id {
            location = location.with_parent(parent_id);
        }
        if let Some(backdrop) = request.backdrop_asset {
            location = location.with_backdrop(backdrop);
        }
        if let Some(grid_map_id) = request.grid_map_id {
            location = location.with_grid_map(grid_map_id);
        }
        for region in request.backdrop_regions {
            location = location.with_backdrop_region(region);
        }

        self.repository
            .locations()
            .create(&location)
            .await
            .context("Failed to create location in repository")?;

        info!(
            location_id = %location.id,
            location_type = ?location.location_type,
            "Created location: {} in world {}",
            location.name,
            request.world_id
        );
        Ok(location)
    }

    #[instrument(skip(self))]
    async fn get_location(&self, id: LocationId) -> Result<Option<Location>> {
        debug!(location_id = %id, "Fetching location");
        self.repository
            .locations()
            .get(id)
            .await
            .context("Failed to get location from repository")
    }

    #[instrument(skip(self))]
    async fn get_location_with_connections(
        &self,
        id: LocationId,
    ) -> Result<Option<LocationWithConnections>> {
        debug!(location_id = %id, "Fetching location with connections");

        let location = match self.repository.locations().get(id).await? {
            Some(l) => l,
            None => return Ok(None),
        };

        let connections = self
            .repository
            .locations()
            .get_connections(id)
            .await
            .context("Failed to get connections for location")?;

        Ok(Some(LocationWithConnections {
            location,
            connections,
        }))
    }

    #[instrument(skip(self))]
    async fn list_locations(&self, world_id: WorldId) -> Result<Vec<Location>> {
        debug!(world_id = %world_id, "Listing locations in world");
        self.repository
            .locations()
            .list_by_world(world_id)
            .await
            .context("Failed to list locations from repository")
    }

    #[instrument(skip(self))]
    async fn list_children(&self, parent_id: LocationId) -> Result<Vec<Location>> {
        debug!(parent_id = %parent_id, "Listing child locations");

        // Get the parent to find its world
        let parent = self
            .repository
            .locations()
            .get(parent_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Parent location not found: {}", parent_id))?;

        let all_locations = self
            .repository
            .locations()
            .list_by_world(parent.world_id)
            .await?;

        Ok(all_locations
            .into_iter()
            .filter(|l| l.parent_id == Some(parent_id))
            .collect())
    }

    #[instrument(skip(self))]
    async fn get_location_hierarchy(&self, world_id: WorldId) -> Result<Vec<LocationHierarchy>> {
        debug!(world_id = %world_id, "Building location hierarchy");

        let locations = self
            .repository
            .locations()
            .list_by_world(world_id)
            .await
            .context("Failed to list locations for hierarchy")?;

        Ok(Self::build_hierarchy(locations))
    }

    #[instrument(skip(self), fields(location_id = %id))]
    async fn update_location(
        &self,
        id: LocationId,
        request: UpdateLocationRequest,
    ) -> Result<Location> {
        Self::validate_update_request(&request)?;

        let mut location = self
            .repository
            .locations()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", id))?;

        if let Some(name) = request.name {
            location.name = name;
        }
        if let Some(description) = request.description {
            location.description = description;
        }
        if let Some(location_type) = request.location_type {
            location.location_type = location_type;
        }
        if let Some(parent_id) = request.parent_id {
            // Verify new parent exists if Some
            if let Some(pid) = parent_id {
                // Prevent circular references
                if pid == id {
                    anyhow::bail!("Location cannot be its own parent");
                }
                let _ = self
                    .repository
                    .locations()
                    .get(pid)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Parent location not found: {}", pid))?;
            }
            location.parent_id = parent_id;
        }
        if request.backdrop_asset.is_some() {
            location.backdrop_asset = request.backdrop_asset;
        }
        if let Some(grid_map_id) = request.grid_map_id {
            location.grid_map_id = grid_map_id;
        }

        self.repository
            .locations()
            .update(&location)
            .await
            .context("Failed to update location in repository")?;

        info!(location_id = %id, "Updated location: {}", location.name);
        Ok(location)
    }

    #[instrument(skip(self))]
    async fn delete_location(&self, id: LocationId) -> Result<()> {
        let location = self
            .repository
            .locations()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", id))?;

        // Check for child locations
        let all_locations = self
            .repository
            .locations()
            .list_by_world(location.world_id)
            .await?;

        let has_children = all_locations.iter().any(|l| l.parent_id == Some(id));
        if has_children {
            anyhow::bail!(
                "Cannot delete location '{}' because it has child locations. Delete children first.",
                location.name
            );
        }

        self.repository
            .locations()
            .delete(id)
            .await
            .context("Failed to delete location from repository")?;

        info!(location_id = %id, "Deleted location: {}", location.name);
        Ok(())
    }

    #[instrument(skip(self), fields(from = %request.from_location, to = %request.to_location))]
    async fn create_connection(&self, request: CreateConnectionRequest) -> Result<()> {
        // Verify both locations exist
        let from = self
            .repository
            .locations()
            .get(request.from_location)
            .await?
            .ok_or_else(|| anyhow::anyhow!("From location not found: {}", request.from_location))?;

        let to = self
            .repository
            .locations()
            .get(request.to_location)
            .await?
            .ok_or_else(|| anyhow::anyhow!("To location not found: {}", request.to_location))?;

        // Verify locations are in the same world
        if from.world_id != to.world_id {
            anyhow::bail!("Cannot create connection between locations in different worlds");
        }

        // Prevent self-connections
        if request.from_location == request.to_location {
            anyhow::bail!("Cannot create connection from a location to itself");
        }

        let mut connection = LocationConnection::new(request.from_location, request.to_location)
            .with_connection_type(request.connection_type);

        if let Some(description) = request.description {
            connection = connection.with_description(description);
        }

        connection.bidirectional = request.bidirectional;
        connection.travel_time = request.travel_time;

        self.repository
            .locations()
            .create_connection(&connection)
            .await
            .context("Failed to create connection in repository")?;

        info!(
            from = %request.from_location,
            to = %request.to_location,
            connection_type = ?request.connection_type,
            "Created connection from '{}' to '{}'",
            from.name,
            to.name
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()> {
        self.repository
            .locations()
            .delete_connection(from, to)
            .await
            .context("Failed to delete connection from repository")?;

        info!(from = %from, to = %to, "Deleted connection");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>> {
        debug!(location_id = %location_id, "Getting connections for location");
        self.repository
            .locations()
            .get_connections(location_id)
            .await
            .context("Failed to get connections from repository")
    }

    #[instrument(skip(self, region), fields(location_id = %location_id))]
    async fn add_backdrop_region(
        &self,
        location_id: LocationId,
        region: BackdropRegion,
    ) -> Result<Location> {
        let mut location = self
            .repository
            .locations()
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        location.backdrop_regions.push(region.clone());

        self.repository
            .locations()
            .update(&location)
            .await
            .context("Failed to add backdrop region to location")?;

        debug!(
            location_id = %location_id,
            region_id = %region.id,
            "Added backdrop region to location: {}",
            location.name
        );
        Ok(location)
    }

    #[instrument(skip(self), fields(location_id = %location_id, region_id = %region_id))]
    async fn remove_backdrop_region(
        &self,
        location_id: LocationId,
        region_id: String,
    ) -> Result<Location> {
        let mut location = self
            .repository
            .locations()
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        if let Some(pos) = location
            .backdrop_regions
            .iter()
            .position(|r| r.id == region_id)
        {
            location.backdrop_regions.remove(pos);

            self.repository
                .locations()
                .update(&location)
                .await
                .context("Failed to remove backdrop region from location")?;

            debug!(
                location_id = %location_id,
                region_id = %region_id,
                "Removed backdrop region from location: {}",
                location.name
            );
        }

        Ok(location)
    }

    #[instrument(skip(self), fields(location_id = %location_id))]
    async fn update_backdrop(&self, location_id: LocationId, asset_path: String) -> Result<Location> {
        let mut location = self
            .repository
            .locations()
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        location.backdrop_asset = Some(asset_path);

        self.repository
            .locations()
            .update(&location)
            .await
            .context("Failed to update location backdrop")?;

        debug!(location_id = %location_id, "Updated backdrop for location: {}", location.name);
        Ok(location)
    }

    #[instrument(skip(self), fields(location_id = %location_id))]
    async fn set_parent(
        &self,
        location_id: LocationId,
        parent_id: Option<LocationId>,
    ) -> Result<Location> {
        let mut location = self
            .repository
            .locations()
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        // Verify new parent exists and prevent circular references
        if let Some(pid) = parent_id {
            if pid == location_id {
                anyhow::bail!("Location cannot be its own parent");
            }

            let parent = self
                .repository
                .locations()
                .get(pid)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Parent location not found: {}", pid))?;

            // Verify same world
            if parent.world_id != location.world_id {
                anyhow::bail!("Parent location must be in the same world");
            }

            // Check for circular reference (parent's ancestors should not include this location)
            let mut current_parent_id = Some(pid);
            while let Some(cpid) = current_parent_id {
                if cpid == location_id {
                    anyhow::bail!("Cannot set parent: would create circular reference");
                }
                if let Some(ancestor) = self.repository.locations().get(cpid).await? {
                    current_parent_id = ancestor.parent_id;
                } else {
                    break;
                }
            }
        }

        location.parent_id = parent_id;

        self.repository
            .locations()
            .update(&location)
            .await
            .context("Failed to update location parent")?;

        info!(
            location_id = %location_id,
            parent_id = ?parent_id,
            "Updated parent for location: {}",
            location.name
        );
        Ok(location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_location_request_validation() {
        // Empty name should fail
        let request = CreateLocationRequest {
            world_id: WorldId::new(),
            name: "".to_string(),
            description: None,
            location_type: LocationType::Interior,
            parent_id: None,
            backdrop_asset: None,
            grid_map_id: None,
            backdrop_regions: vec![],
        };
        assert!(LocationServiceImpl::validate_create_request(&request).is_err());

        // Valid request should pass
        let request = CreateLocationRequest {
            world_id: WorldId::new(),
            name: "Tavern".to_string(),
            description: Some("A cozy tavern".to_string()),
            location_type: LocationType::Interior,
            parent_id: None,
            backdrop_asset: None,
            grid_map_id: None,
            backdrop_regions: vec![],
        };
        assert!(LocationServiceImpl::validate_create_request(&request).is_ok());
    }

    #[test]
    fn test_build_hierarchy() {
        let world_id = WorldId::new();
        let root_id = LocationId::new();
        let child1_id = LocationId::new();
        let child2_id = LocationId::new();
        let grandchild_id = LocationId::new();

        let locations = vec![
            Location {
                id: root_id,
                world_id,
                parent_id: None,
                name: "Root".to_string(),
                description: String::new(),
                location_type: LocationType::Exterior,
                backdrop_asset: None,
                grid_map_id: None,
                backdrop_regions: vec![],
            },
            Location {
                id: child1_id,
                world_id,
                parent_id: Some(root_id),
                name: "Child1".to_string(),
                description: String::new(),
                location_type: LocationType::Interior,
                backdrop_asset: None,
                grid_map_id: None,
                backdrop_regions: vec![],
            },
            Location {
                id: child2_id,
                world_id,
                parent_id: Some(root_id),
                name: "Child2".to_string(),
                description: String::new(),
                location_type: LocationType::Interior,
                backdrop_asset: None,
                grid_map_id: None,
                backdrop_regions: vec![],
            },
            Location {
                id: grandchild_id,
                world_id,
                parent_id: Some(child1_id),
                name: "Grandchild".to_string(),
                description: String::new(),
                location_type: LocationType::Interior,
                backdrop_asset: None,
                grid_map_id: None,
                backdrop_regions: vec![],
            },
        ];

        let hierarchy = LocationServiceImpl::build_hierarchy(locations);

        assert_eq!(hierarchy.len(), 1); // One root
        assert_eq!(hierarchy[0].location.name, "Root");
        assert_eq!(hierarchy[0].children.len(), 2); // Two children

        // Find Child1 and verify it has the grandchild
        let child1 = hierarchy[0]
            .children
            .iter()
            .find(|h| h.location.name == "Child1")
            .unwrap();
        assert_eq!(child1.children.len(), 1);
        assert_eq!(child1.children[0].location.name, "Grandchild");
    }
}
