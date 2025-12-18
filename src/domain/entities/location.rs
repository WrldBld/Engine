//! Location entity - Physical or conceptual places in the world
//!
//! Locations form a hierarchy via CONTAINS_LOCATION edges in Neo4j.
//! Connections between locations use CONNECTED_TO edges.
//! Regions are separate nodes with HAS_REGION edges (see region.rs).

use crate::domain::value_objects::{LocationId, RegionId, WorldId};
use super::region::MapBounds;

/// A location in the world
///
/// Locations form a hierarchy via Neo4j edges:
/// - Parent/child: `(parent)-[:CONTAINS_LOCATION]->(child)`
/// - Navigation: `(from)-[:CONNECTED_TO]->(to)`
/// - Regions: `(location)-[:HAS_REGION]->(region)`
/// - Grid map: `(location)-[:HAS_TACTICAL_MAP]->(map)`
#[derive(Debug, Clone)]
pub struct Location {
    pub id: LocationId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub location_type: LocationType,
    
    // Visual assets
    /// Path to the default backdrop image asset (used if entering without specific region)
    pub backdrop_asset: Option<String>,
    /// Path to the top-down map image for navigation between regions
    pub map_asset: Option<String>,
    
    // Position on parent location's map (if this location is nested)
    /// Bounds defining where this location appears on its parent's map
    pub parent_map_bounds: Option<MapBounds>,
    
    // Default entry point
    /// Default region to place players when arriving without a specific region target
    pub default_region_id: Option<RegionId>,
    
    /// Sensory/emotional description of the location's atmosphere
    pub atmosphere: Option<String>,
}

impl Location {
    pub fn new(world_id: WorldId, name: impl Into<String>, location_type: LocationType) -> Self {
        Self {
            id: LocationId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            location_type,
            backdrop_asset: None,
            map_asset: None,
            parent_map_bounds: None,
            default_region_id: None,
            atmosphere: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_backdrop(mut self, asset_path: impl Into<String>) -> Self {
        self.backdrop_asset = Some(asset_path.into());
        self
    }

    pub fn with_map(mut self, asset_path: impl Into<String>) -> Self {
        self.map_asset = Some(asset_path.into());
        self
    }

    pub fn with_parent_map_bounds(mut self, bounds: MapBounds) -> Self {
        self.parent_map_bounds = Some(bounds);
        self
    }

    pub fn with_default_region(mut self, region_id: RegionId) -> Self {
        self.default_region_id = Some(region_id);
        self
    }

    pub fn with_atmosphere(mut self, atmosphere: impl Into<String>) -> Self {
        self.atmosphere = Some(atmosphere.into());
        self
    }

    /// Check if a pixel position is within this location's parent map bounds
    pub fn contains_point_on_parent_map(&self, x: u32, y: u32) -> bool {
        if let Some(bounds) = &self.parent_map_bounds {
            bounds.contains(x, y)
        } else {
            false
        }
    }
}

/// The type of location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocationType {
    /// Indoor location (tavern, dungeon room, etc.)
    Interior,
    /// Outdoor location (forest, city street, etc.)
    Exterior,
    /// Abstract or metaphysical location (dreamscape, etc.)
    Abstract,
}

/// A connection between two locations
///
/// Stored as a `CONNECTED_TO` edge in Neo4j with properties.
#[derive(Debug, Clone)]
pub struct LocationConnection {
    pub from_location: LocationId,
    pub to_location: LocationId,
    /// Type of connection (Door, Path, Stairs, Portal, etc.)
    pub connection_type: String,
    /// Description of the path/transition
    pub description: Option<String>,
    /// Whether this connection works both ways
    pub bidirectional: bool,
    /// Travel time in game-time units (0 = instant)
    pub travel_time: u32,
    /// Whether this connection is currently locked
    pub is_locked: bool,
    /// Description of what's needed to unlock (if locked)
    pub lock_description: Option<String>,
}

impl LocationConnection {
    pub fn new(from: LocationId, to: LocationId, connection_type: impl Into<String>) -> Self {
        Self {
            from_location: from,
            to_location: to,
            connection_type: connection_type.into(),
            description: None,
            bidirectional: true,
            travel_time: 0,
            is_locked: false,
            lock_description: None,
        }
    }

    /// Create a door connection
    pub fn door(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, "Door")
    }

    /// Create a path/road connection
    pub fn path(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, "Path")
    }

    /// Create a stairs connection
    pub fn stairs(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, "Stairs")
    }

    /// Create a portal/magical connection
    pub fn portal(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, "Portal")
    }

    pub fn one_way(mut self) -> Self {
        self.bidirectional = false;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_travel_time(mut self, time: u32) -> Self {
        self.travel_time = time;
        self
    }

    pub fn locked(mut self, description: impl Into<String>) -> Self {
        self.is_locked = true;
        self.lock_description = Some(description.into());
        self
    }
}
