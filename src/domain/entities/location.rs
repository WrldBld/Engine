//! Location entity - Physical or conceptual places in the world

use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{GridMapId, LocationId, WorldId};

/// A location in the world
///
/// Locations form a hierarchy - a Town contains a Bar, the Bar contains rooms.
/// The parent_id field establishes this containment relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: LocationId,
    pub world_id: WorldId,
    /// Parent location (if this location is inside another)
    pub parent_id: Option<LocationId>,
    pub name: String,
    pub description: String,
    pub location_type: LocationType,
    /// Path to the default backdrop image asset
    pub backdrop_asset: Option<String>,
    /// Optional tactical grid map for this location
    pub grid_map_id: Option<GridMapId>,
    /// Backdrop regions for different areas of the map
    pub backdrop_regions: Vec<BackdropRegion>,
}

impl Location {
    pub fn new(world_id: WorldId, name: impl Into<String>, location_type: LocationType) -> Self {
        Self {
            id: LocationId::new(),
            world_id,
            parent_id: None,
            name: name.into(),
            description: String::new(),
            location_type,
            backdrop_asset: None,
            grid_map_id: None,
            backdrop_regions: Vec::new(),
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

    pub fn with_grid_map(mut self, grid_map_id: GridMapId) -> Self {
        self.grid_map_id = Some(grid_map_id);
        self
    }

    pub fn with_parent(mut self, parent_id: LocationId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_backdrop_region(mut self, region: BackdropRegion) -> Self {
        self.backdrop_regions.push(region);
        self
    }
}

/// A region within a location that has its own backdrop
///
/// Example: A town map might have regions for "Church", "Tavern", "Slums"
/// each with a different backdrop image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackdropRegion {
    pub id: String,
    pub name: String,
    /// Rectangular region on the grid (x, y, width, height)
    pub bounds: RegionBounds,
    /// Backdrop image for this region
    pub backdrop_asset: String,
    /// Optional description shown when entering this region
    pub description: Option<String>,
}

impl BackdropRegion {
    pub fn new(
        name: impl Into<String>,
        bounds: RegionBounds,
        backdrop_asset: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            bounds,
            backdrop_asset: backdrop_asset.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Check if a grid position is within this region
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.bounds.x
            && x < self.bounds.x + self.bounds.width
            && y >= self.bounds.y
            && y < self.bounds.y + self.bounds.height
    }
}

/// Rectangular bounds for a region
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RegionBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The type of location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocationType {
    /// Indoor location (tavern, dungeon room, etc.)
    Interior,
    /// Outdoor location (forest, city street, etc.)
    Exterior,
    /// Abstract or metaphysical location (dreamscape, etc.)
    Abstract,
}

/// A connection between two locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationConnection {
    pub from_location: LocationId,
    pub to_location: LocationId,
    /// Type of spatial relationship
    pub connection_type: SpatialRelationship,
    /// Description of the path/transition
    pub description: String,
    /// Whether this connection is bidirectional
    pub bidirectional: bool,
    /// Requirements to use this connection
    pub requirements: Vec<ConnectionRequirement>,
    /// Travel time in arbitrary units
    pub travel_time: Option<u32>,
}

impl LocationConnection {
    pub fn new(from: LocationId, to: LocationId) -> Self {
        Self {
            from_location: from,
            to_location: to,
            connection_type: SpatialRelationship::ConnectsTo,
            description: String::new(),
            bidirectional: true,
            requirements: Vec::new(),
            travel_time: None,
        }
    }

    /// Create an "enters" connection (going inside another location)
    pub fn enters(from: LocationId, to: LocationId) -> Self {
        Self {
            from_location: from,
            to_location: to,
            connection_type: SpatialRelationship::Enters,
            description: String::new(),
            bidirectional: false, // Entry is one-way, exit is the reverse
            requirements: Vec::new(),
            travel_time: None,
        }
    }

    /// Create an "exits" connection (leaving a containing location)
    pub fn exits(from: LocationId, to: LocationId) -> Self {
        Self {
            from_location: from,
            to_location: to,
            connection_type: SpatialRelationship::Exits,
            description: String::new(),
            bidirectional: false,
            requirements: Vec::new(),
            travel_time: None,
        }
    }

    pub fn one_way(mut self) -> Self {
        self.bidirectional = false;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_requirement(mut self, requirement: ConnectionRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    pub fn with_connection_type(mut self, connection_type: SpatialRelationship) -> Self {
        self.connection_type = connection_type;
        self
    }
}

/// Spatial relationship between locations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SpatialRelationship {
    /// Standard connection (door, path, road)
    #[default]
    ConnectsTo,
    /// Entering a location (going inside)
    /// Example: Town -> Bar (entering the bar)
    Enters,
    /// Exiting a location (going outside)
    /// Example: Bar -> Town (leaving the bar)
    Exits,
    /// Leads to (travel/transition)
    LeadsTo,
    /// Adjacent locations that share a border
    AdjacentTo,
}

/// A requirement to use a location connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionRequirement {
    /// Must have a specific item
    HasItem(crate::domain::value_objects::ItemId),
    /// Must have completed a scene
    CompletedScene(crate::domain::value_objects::SceneId),
    /// Must pass a skill check
    SkillCheck { stat: String, difficulty: i32 },
    /// Custom requirement
    Custom(String),
}
