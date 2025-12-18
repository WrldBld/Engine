use serde::{Deserialize, Serialize};

use crate::domain::entities::{Location, LocationConnection, LocationType, MapBounds, Region};

#[derive(Debug, Deserialize)]
pub struct CreateLocationRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub location_type: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub backdrop_asset: Option<String>,
    #[serde(default)]
    pub map_asset: Option<String>,
    #[serde(default)]
    pub parent_map_bounds: Option<MapBoundsDto>,
    #[serde(default)]
    pub default_region_id: Option<String>,
    #[serde(default)]
    pub atmosphere: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBoundsDto {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl From<MapBounds> for MapBoundsDto {
    fn from(b: MapBounds) -> Self {
        Self {
            x: b.x,
            y: b.y,
            width: b.width,
            height: b.height,
        }
    }
}

impl From<MapBoundsDto> for MapBounds {
    fn from(b: MapBoundsDto) -> Self {
        Self {
            x: b.x,
            y: b.y,
            width: b.width,
            height: b.height,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRegionRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub backdrop_asset: Option<String>,
    #[serde(default)]
    pub atmosphere: Option<String>,
    #[serde(default)]
    pub map_bounds: Option<MapBoundsDto>,
    #[serde(default)]
    pub is_spawn_point: bool,
    #[serde(default)]
    pub order: u32,
}

#[derive(Debug, Serialize)]
pub struct LocationResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub location_type: String,
    pub backdrop_asset: Option<String>,
    pub map_asset: Option<String>,
    pub parent_map_bounds: Option<MapBoundsDto>,
    pub default_region_id: Option<String>,
    pub atmosphere: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegionResponseDto {
    pub id: String,
    pub location_id: String,
    pub name: String,
    pub description: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub map_bounds: Option<MapBoundsDto>,
    pub is_spawn_point: bool,
    pub order: u32,
}

impl From<Region> for RegionResponseDto {
    fn from(r: Region) -> Self {
        Self {
            id: r.id.to_string(),
            location_id: r.location_id.to_string(),
            name: r.name,
            description: r.description,
            backdrop_asset: r.backdrop_asset,
            atmosphere: r.atmosphere,
            map_bounds: r.map_bounds.map(MapBoundsDto::from),
            is_spawn_point: r.is_spawn_point,
            order: r.order,
        }
    }
}

impl From<Location> for LocationResponseDto {
    fn from(l: Location) -> Self {
        Self {
            id: l.id.to_string(),
            world_id: l.world_id.to_string(),
            name: l.name,
            description: l.description,
            location_type: format!("{:?}", l.location_type),
            backdrop_asset: l.backdrop_asset,
            map_asset: l.map_asset,
            parent_map_bounds: l.parent_map_bounds.map(MapBoundsDto::from),
            default_region_id: l.default_region_id.map(|id| id.to_string()),
            atmosphere: l.atmosphere,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequestDto {
    pub from_location_id: String,
    pub to_location_id: String,
    #[serde(default = "default_connection_type")]
    pub connection_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_bidirectional")]
    pub bidirectional: bool,
    #[serde(default)]
    pub travel_time: u32,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub lock_description: Option<String>,
}

fn default_connection_type() -> String {
    "Door".to_string()
}

fn default_bidirectional() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct ConnectionResponseDto {
    pub from_location_id: String,
    pub to_location_id: String,
    pub connection_type: String,
    pub description: Option<String>,
    pub bidirectional: bool,
    pub travel_time: u32,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

impl From<LocationConnection> for ConnectionResponseDto {
    fn from(c: LocationConnection) -> Self {
        Self {
            from_location_id: c.from_location.to_string(),
            to_location_id: c.to_location.to_string(),
            connection_type: c.connection_type,
            description: c.description,
            bidirectional: c.bidirectional,
            travel_time: c.travel_time,
            is_locked: c.is_locked,
            lock_description: c.lock_description,
        }
    }
}

pub fn parse_location_type(s: &str) -> LocationType {
    match s {
        "Interior" => LocationType::Interior,
        "Exterior" => LocationType::Exterior,
        "Abstract" => LocationType::Abstract,
        _ => LocationType::Interior,
    }
}
