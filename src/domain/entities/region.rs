//! Region entity - Sub-locations within a Location
//!
//! Regions represent distinct areas within a location, each with their own
//! backdrop image for scene display. Think of them as "screens" in a JRPG.
//!
//! # Neo4j Relationships
//! - `(Location)-[:HAS_REGION]->(Region)` - Containment
//! - `(Region)-[:CONNECTED_TO_REGION]->(Region)` - Internal navigation
//! - `(Region)-[:EXITS_TO_LOCATION]->(Location)` - Exit to another location
//! - `(Character)-[:WORKS_AT_REGION]->(Region)` - NPC works here
//! - `(Character)-[:FREQUENTS_REGION]->(Region)` - NPC hangs out here
//! - `(Character)-[:HOME_REGION]->(Region)` - NPC lives here
//! - `(Character)-[:AVOIDS_REGION]->(Region)` - NPC avoids this place

use crate::domain::value_objects::{LocationId, RegionId};

/// A region within a location - represents a distinct "screen" or area
///
/// Regions are the leaf nodes of the location hierarchy. Players navigate
/// between regions, and scenes are derived from the current region's backdrop
/// plus any NPCs present.
#[derive(Debug, Clone)]
pub struct Region {
    pub id: RegionId,
    pub location_id: LocationId,
    pub name: String,
    pub description: String,

    // Scene display (visual novel view)
    /// Path to backdrop image for this region's scene
    pub backdrop_asset: Option<String>,
    /// Sensory/emotional description of the region's atmosphere
    pub atmosphere: Option<String>,

    // Position on parent location's map (clickable area)
    /// Bounds defining where this region is on the parent location's map
    pub map_bounds: Option<MapBounds>,

    /// Whether players can spawn here when creating a new PC
    pub is_spawn_point: bool,
    /// Display order within the location
    pub order: u32,
}

impl Region {
    /// Create a new region within a location
    pub fn new(location_id: LocationId, name: impl Into<String>) -> Self {
        Self {
            id: RegionId::new(),
            location_id,
            name: name.into(),
            description: String::new(),
            backdrop_asset: None,
            atmosphere: None,
            map_bounds: None,
            is_spawn_point: false,
            order: 0,
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

    pub fn with_atmosphere(mut self, atmosphere: impl Into<String>) -> Self {
        self.atmosphere = Some(atmosphere.into());
        self
    }

    pub fn with_map_bounds(mut self, bounds: MapBounds) -> Self {
        self.map_bounds = Some(bounds);
        self
    }

    pub fn as_spawn_point(mut self) -> Self {
        self.is_spawn_point = true;
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    /// Check if a pixel position is within this region's map bounds
    pub fn contains_point(&self, x: u32, y: u32) -> bool {
        if let Some(bounds) = &self.map_bounds {
            bounds.contains(x, y)
        } else {
            false
        }
    }
}

/// Bounds defining a rectangular area on a map image
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapBounds {
    /// X coordinate of the region's top-left corner
    pub x: u32,
    /// Y coordinate of the region's top-left corner
    pub y: u32,
    /// Width of the region
    pub width: u32,
    /// Height of the region
    pub height: u32,
}

impl MapBounds {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Check if a pixel position is within these bounds
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// A connection between two regions
///
/// Stored as a `CONNECTED_TO_REGION` edge in Neo4j with properties.
#[derive(Debug, Clone)]
pub struct RegionConnection {
    pub from_region: RegionId,
    pub to_region: RegionId,
    /// Description of the path/transition (e.g., "A door leads to...")
    pub description: Option<String>,
    /// Whether this connection works both ways
    pub bidirectional: bool,
    /// Whether this connection is currently locked
    pub is_locked: bool,
    /// Description of what's needed to unlock (if locked)
    pub lock_description: Option<String>,
}

impl RegionConnection {
    pub fn new(from: RegionId, to: RegionId) -> Self {
        Self {
            from_region: from,
            to_region: to,
            description: None,
            bidirectional: true,
            is_locked: false,
            lock_description: None,
        }
    }

    pub fn one_way(mut self) -> Self {
        self.bidirectional = false;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn locked(mut self, description: impl Into<String>) -> Self {
        self.is_locked = true;
        self.lock_description = Some(description.into());
        self
    }
}

/// An exit from a region to another location
///
/// Stored as an `EXITS_TO_LOCATION` edge in Neo4j with properties.
/// Used when leaving a building/area to go to a parent or sibling location.
#[derive(Debug, Clone)]
pub struct RegionExit {
    pub from_region: RegionId,
    pub to_location: LocationId,
    /// Which region in the target location the player arrives at
    pub arrival_region_id: RegionId,
    /// Description of the exit (e.g., "Step outside into the market")
    pub description: Option<String>,
    /// Whether this exit works both ways (can enter from that location)
    pub bidirectional: bool,
}

impl RegionExit {
    pub fn new(from: RegionId, to_location: LocationId, arrival_region: RegionId) -> Self {
        Self {
            from_region: from,
            to_location,
            arrival_region_id: arrival_region,
            description: None,
            bidirectional: true,
        }
    }

    pub fn one_way(mut self) -> Self {
        self.bidirectional = false;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
