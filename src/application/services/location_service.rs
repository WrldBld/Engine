//! Location Service - Application service for location management
//!
//! This service provides use case implementations for creating, updating,
//! and managing locations, including hierarchy and connections.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::{LocationRepositoryPort, WorldRepositoryPort};
use crate::domain::entities::{Location, LocationConnection, LocationType, Region};
use crate::domain::value_objects::{GridMapId, LocationId, RegionId, WorldId};

// Validation constants
const MAX_LOCATION_NAME_LENGTH: usize = 255;
const MAX_LOCATION_DESCRIPTION_LENGTH: usize = 10000;

/// Request to create a new location
#[derive(Debug, Clone)]
pub struct CreateLocationRequest {
    pub world_id: WorldId,
    pub name: String,
    pub description: Option<String>,
    pub location_type: LocationType,
    pub parent_id: Option<LocationId>,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
}

/// Request to update an existing location
#[derive(Debug, Clone)]
pub struct UpdateLocationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub location_type: Option<LocationType>,
    pub backdrop_asset: Option<Option<String>>,
    pub atmosphere: Option<Option<String>>,
}

/// Request to create a connection between locations
#[derive(Debug, Clone)]
pub struct CreateConnectionRequest {
    pub from_location: LocationId,
    pub to_location: LocationId,
    pub connection_type: String,
    pub description: Option<String>,
    pub bidirectional: bool,
    pub travel_time: u32,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

/// Location with all its connections
#[derive(Debug, Clone)]
pub struct LocationWithConnections {
    pub location: Location,
    pub connections: Vec<LocationConnection>,
    pub parent: Option<Location>,
    pub children: Vec<Location>,
    pub regions: Vec<Region>,
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
    /// Create a new location
    async fn create_location(&self, request: CreateLocationRequest) -> Result<Location>;

    /// Get a location by ID
    async fn get_location(&self, id: LocationId) -> Result<Option<Location>>;

    /// Get a location with all its related data (connections, parent, children, regions)
    async fn get_location_with_connections(
        &self,
        id: LocationId,
    ) -> Result<Option<LocationWithConnections>>;

    /// List all locations in a world
    async fn list_locations(&self, world_id: WorldId) -> Result<Vec<Location>>;

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

    // -------------------------------------------------------------------------
    // Hierarchy operations (via edges)
    // -------------------------------------------------------------------------

    /// Set a location's parent (move in hierarchy)
    async fn set_parent(
        &self,
        location_id: LocationId,
        parent_id: Option<LocationId>,
    ) -> Result<()>;

    /// Get a location's children
    async fn get_children(&self, location_id: LocationId) -> Result<Vec<Location>>;

    // -------------------------------------------------------------------------
    // Connection operations (via edges)
    // -------------------------------------------------------------------------

    /// Create a connection between two locations
    async fn create_connection(&self, request: CreateConnectionRequest) -> Result<()>;

    /// Get all connections from a location
    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>>;

    /// Delete a connection between locations
    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()>;

    /// Unlock a connection
    async fn unlock_connection(&self, from: LocationId, to: LocationId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Region operations (via edges)
    // -------------------------------------------------------------------------

    /// Add a region to a location
    async fn add_region(
        &self,
        location_id: LocationId,
        region: Region,
    ) -> Result<()>;

    // -------------------------------------------------------------------------
    // Grid map operations (via edge)
    // -------------------------------------------------------------------------

    /// Set a location's tactical map
    async fn set_grid_map(&self, location_id: LocationId, grid_map_id: GridMapId) -> Result<()>;

    /// Remove a location's tactical map
    async fn remove_grid_map(&self, location_id: LocationId) -> Result<()>;
}

/// Default implementation of LocationService using port abstractions
pub struct LocationServiceImpl {
    world_repository: Arc<dyn WorldRepositoryPort>,
    location_repository: Arc<dyn LocationRepositoryPort>,
}

impl LocationServiceImpl {
    /// Create a new LocationServiceImpl with the given repositories
    pub fn new(
        world_repository: Arc<dyn WorldRepositoryPort>,
        location_repository: Arc<dyn LocationRepositoryPort>,
    ) -> Self {
        Self {
            world_repository,
            location_repository,
        }
    }

    /// Validate a location creation request
    fn validate_create_request(request: &CreateLocationRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            anyhow::bail!("Location name cannot be empty");
        }
        if request.name.len() > MAX_LOCATION_NAME_LENGTH {
            anyhow::bail!("Location name cannot exceed {MAX_LOCATION_NAME_LENGTH} characters");
        }
        if let Some(ref description) = request.description {
            if description.len() > MAX_LOCATION_DESCRIPTION_LENGTH {
                anyhow::bail!("Location description cannot exceed {MAX_LOCATION_DESCRIPTION_LENGTH} characters");
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
            if name.len() > MAX_LOCATION_NAME_LENGTH {
                anyhow::bail!("Location name cannot exceed {MAX_LOCATION_NAME_LENGTH} characters");
            }
        }
        if let Some(ref description) = request.description {
            if description.len() > MAX_LOCATION_DESCRIPTION_LENGTH {
                anyhow::bail!("Location description cannot exceed {MAX_LOCATION_DESCRIPTION_LENGTH} characters");
            }
        }
        Ok(())
    }

    /// Build hierarchy tree by querying parent/child edges
    async fn build_hierarchy_from_repo(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<LocationHierarchy>> {
        // Get all locations in the world
        let locations = self.location_repository.list(world_id).await?;

        // Build a map of location_id -> Location
        let location_map: std::collections::HashMap<LocationId, Location> = locations
            .iter()
            .cloned()
            .map(|l| (l.id, l))
            .collect();

        // Build parent -> children map by querying each location's children
        let mut children_map: std::collections::HashMap<LocationId, Vec<Location>> =
            std::collections::HashMap::new();
        let mut root_locations = Vec::new();

        for location in &locations {
            let parent = self.location_repository.get_parent(location.id).await?;
            if let Some(parent_loc) = parent {
                children_map
                    .entry(parent_loc.id)
                    .or_default()
                    .push(location.clone());
            } else {
                root_locations.push(location.clone());
            }
        }

        // Recursive function to build tree
        fn build_tree(
            location: Location,
            children_map: &std::collections::HashMap<LocationId, Vec<Location>>,
        ) -> LocationHierarchy {
            let children = children_map
                .get(&location.id)
                .map(|kids| {
                    kids.iter()
                        .map(|child| build_tree(child.clone(), children_map))
                        .collect()
                })
                .unwrap_or_default();

            LocationHierarchy { location, children }
        }

        Ok(root_locations
            .into_iter()
            .map(|loc| build_tree(loc, &children_map))
            .collect())
    }
}

#[async_trait]
impl LocationService for LocationServiceImpl {
    #[instrument(skip(self), fields(world_id = %request.world_id, name = %request.name))]
    async fn create_location(&self, request: CreateLocationRequest) -> Result<Location> {
        Self::validate_create_request(&request)?;

        // Verify the world exists
        let _ = self
            .world_repository
            .get(request.world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", request.world_id))?;

        // Verify parent exists if specified
        if let Some(parent_id) = request.parent_id {
            let _ = self
                .location_repository
                .get(parent_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Parent location not found: {}", parent_id))?;
        }

        let mut location = Location::new(request.world_id, &request.name, request.location_type);

        if let Some(description) = request.description {
            location = location.with_description(description);
        }
        if let Some(backdrop) = request.backdrop_asset {
            location = location.with_backdrop(backdrop);
        }
        if let Some(atmosphere) = request.atmosphere {
            location = location.with_atmosphere(atmosphere);
        }

        // Create the location node
        self.location_repository
            .create(&location)
            .await
            .context("Failed to create location in repository")?;

        // Set parent if specified (creates CONTAINS_LOCATION edge)
        if let Some(parent_id) = request.parent_id {
            self.location_repository
                .set_parent(location.id, parent_id)
                .await
                .context("Failed to set location parent")?;
        }

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
        self.location_repository
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

        let location = match self.location_repository.get(id).await? {
            Some(l) => l,
            None => return Ok(None),
        };

        let connections = self
            .location_repository
            .get_connections(id)
            .await
            .context("Failed to get connections for location")?;

        let parent = self
            .location_repository
            .get_parent(id)
            .await
            .context("Failed to get parent location")?;

        let children = self
            .location_repository
            .get_children(id)
            .await
            .context("Failed to get child locations")?;

        let regions = self
            .location_repository
            .get_regions(id)
            .await
            .context("Failed to get backdrop regions")?;

        Ok(Some(LocationWithConnections {
            location,
            connections,
            parent,
            children,
            regions,
        }))
    }

    #[instrument(skip(self))]
    async fn list_locations(&self, world_id: WorldId) -> Result<Vec<Location>> {
        debug!(world_id = %world_id, "Listing locations in world");
        self.location_repository
            .list(world_id)
            .await
            .context("Failed to list locations from repository")
    }

    #[instrument(skip(self))]
    async fn get_location_hierarchy(&self, world_id: WorldId) -> Result<Vec<LocationHierarchy>> {
        debug!(world_id = %world_id, "Building location hierarchy");
        self.build_hierarchy_from_repo(world_id).await
    }

    #[instrument(skip(self), fields(location_id = %id))]
    async fn update_location(
        &self,
        id: LocationId,
        request: UpdateLocationRequest,
    ) -> Result<Location> {
        Self::validate_update_request(&request)?;

        let mut location = self
            .location_repository
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
        if let Some(backdrop_asset) = request.backdrop_asset {
            location.backdrop_asset = backdrop_asset;
        }
        if let Some(atmosphere) = request.atmosphere {
            location.atmosphere = atmosphere;
        }

        self.location_repository
            .update(&location)
            .await
            .context("Failed to update location in repository")?;

        info!(location_id = %id, "Updated location: {}", location.name);
        Ok(location)
    }

    #[instrument(skip(self))]
    async fn delete_location(&self, id: LocationId) -> Result<()> {
        let location = self
            .location_repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", id))?;

        // Check for child locations
        let children = self.location_repository.get_children(id).await?;
        if !children.is_empty() {
            anyhow::bail!(
                "Cannot delete location '{}' because it has {} child location(s). Delete children first.",
                location.name,
                children.len()
            );
        }

        self.location_repository
            .delete(id)
            .await
            .context("Failed to delete location from repository")?;

        info!(location_id = %id, "Deleted location: {}", location.name);
        Ok(())
    }

    #[instrument(skip(self), fields(location_id = %location_id))]
    async fn set_parent(
        &self,
        location_id: LocationId,
        parent_id: Option<LocationId>,
    ) -> Result<()> {
        let location = self
            .location_repository
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        if let Some(pid) = parent_id {
            // Prevent self-reference
            if pid == location_id {
                anyhow::bail!("Location cannot be its own parent");
            }

            let parent = self
                .location_repository
                .get(pid)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Parent location not found: {}", pid))?;

            // Verify same world
            if parent.world_id != location.world_id {
                anyhow::bail!("Parent location must be in the same world");
            }

            // Check for circular reference
            let mut current_parent_id = Some(pid);
            while let Some(cpid) = current_parent_id {
                if cpid == location_id {
                    anyhow::bail!("Cannot set parent: would create circular reference");
                }
                current_parent_id = self
                    .location_repository
                    .get_parent(cpid)
                    .await?
                    .map(|p| p.id);
            }

            self.location_repository
                .set_parent(location_id, pid)
                .await
                .context("Failed to set location parent")?;
        } else {
            self.location_repository
                .remove_parent(location_id)
                .await
                .context("Failed to remove location parent")?;
        }

        info!(
            location_id = %location_id,
            parent_id = ?parent_id,
            "Updated parent for location: {}",
            location.name
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_children(&self, location_id: LocationId) -> Result<Vec<Location>> {
        self.location_repository
            .get_children(location_id)
            .await
            .context("Failed to get child locations")
    }

    #[instrument(skip(self), fields(from = %request.from_location, to = %request.to_location))]
    async fn create_connection(&self, request: CreateConnectionRequest) -> Result<()> {
        // Verify both locations exist
        let from = self
            .location_repository
            .get(request.from_location)
            .await?
            .ok_or_else(|| anyhow::anyhow!("From location not found: {}", request.from_location))?;

        let to = self
            .location_repository
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

        let mut connection =
            LocationConnection::new(request.from_location, request.to_location, &request.connection_type);

        if let Some(description) = request.description {
            connection = connection.with_description(description);
        }
        connection.bidirectional = request.bidirectional;
        connection.travel_time = request.travel_time;
        if request.is_locked {
            if let Some(lock_desc) = request.lock_description {
                connection = connection.locked(lock_desc);
            } else {
                connection.is_locked = true;
            }
        }

        self.location_repository
            .create_connection(&connection)
            .await
            .context("Failed to create connection in repository")?;

        info!(
            from = %request.from_location,
            to = %request.to_location,
            connection_type = %request.connection_type,
            "Created connection from '{}' to '{}'",
            from.name,
            to.name
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>> {
        debug!(location_id = %location_id, "Getting connections for location");
        self.location_repository
            .get_connections(location_id)
            .await
            .context("Failed to get connections from repository")
    }

    #[instrument(skip(self))]
    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()> {
        self.location_repository
            .delete_connection(from, to)
            .await
            .context("Failed to delete connection from repository")?;

        info!(from = %from, to = %to, "Deleted connection");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn unlock_connection(&self, from: LocationId, to: LocationId) -> Result<()> {
        self.location_repository
            .unlock_connection(from, to)
            .await
            .context("Failed to unlock connection")?;

        info!(from = %from, to = %to, "Unlocked connection");
        Ok(())
    }

    #[instrument(skip(self, region), fields(location_id = %location_id))]
    async fn add_region(
        &self,
        location_id: LocationId,
        region: Region,
    ) -> Result<()> {
        // Verify location exists
        let location = self
            .location_repository
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        self.location_repository
            .create_region(location_id, &region)
            .await
            .context("Failed to add region")?;

        debug!(
            location_id = %location_id,
            region_id = %region.id,
            "Added region to location: {}",
            location.name
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn set_grid_map(&self, location_id: LocationId, grid_map_id: GridMapId) -> Result<()> {
        // Verify location exists
        let _ = self
            .location_repository
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        self.location_repository
            .set_grid_map(location_id, grid_map_id)
            .await
            .context("Failed to set grid map")?;

        info!(location_id = %location_id, grid_map_id = %grid_map_id, "Set grid map for location");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_grid_map(&self, location_id: LocationId) -> Result<()> {
        self.location_repository
            .remove_grid_map(location_id)
            .await
            .context("Failed to remove grid map")?;

        info!(location_id = %location_id, "Removed grid map from location");
        Ok(())
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
            atmosphere: None,
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
            atmosphere: None,
        };
        assert!(LocationServiceImpl::validate_create_request(&request).is_ok());
    }
}
