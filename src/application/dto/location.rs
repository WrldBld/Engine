use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    BackdropRegion, Location, LocationConnection, LocationType, RegionBounds, SpatialRelationship,
};

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
    pub backdrop_regions: Vec<BackdropRegionRequestDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackdropRegionRequestDto {
    pub name: String,
    pub bounds: RegionBoundsRequestDto,
    pub backdrop_asset: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RegionBoundsRequestDto {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize)]
pub struct LocationResponseDto {
    pub id: String,
    pub world_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: String,
    pub location_type: String,
    pub backdrop_asset: Option<String>,
    pub grid_map_id: Option<String>,
    pub backdrop_regions: Vec<BackdropRegionResponseDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackdropRegionResponseDto {
    pub id: String,
    pub name: String,
    pub bounds: RegionBoundsRequestDto,
    pub backdrop_asset: String,
    pub description: Option<String>,
}

impl From<BackdropRegion> for BackdropRegionResponseDto {
    fn from(r: BackdropRegion) -> Self {
        Self {
            id: r.id,
            name: r.name,
            bounds: RegionBoundsRequestDto {
                x: r.bounds.x,
                y: r.bounds.y,
                width: r.bounds.width,
                height: r.bounds.height,
            },
            backdrop_asset: r.backdrop_asset,
            description: r.description,
        }
    }
}

impl From<Location> for LocationResponseDto {
    fn from(l: Location) -> Self {
        Self {
            id: l.id.to_string(),
            world_id: l.world_id.to_string(),
            parent_id: l.parent_id.map(|id| id.to_string()),
            name: l.name,
            description: l.description,
            location_type: format!("{:?}", l.location_type),
            backdrop_asset: l.backdrop_asset,
            grid_map_id: l.grid_map_id.map(|id| id.to_string()),
            backdrop_regions: l
                .backdrop_regions
                .into_iter()
                .map(BackdropRegionResponseDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequestDto {
    pub from_location_id: String,
    pub to_location_id: String,
    #[serde(default)]
    pub connection_type: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_bidirectional")]
    pub bidirectional: bool,
    #[serde(default)]
    pub travel_time: Option<u32>,
}

fn default_bidirectional() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct ConnectionResponseDto {
    pub from_location_id: String,
    pub to_location_id: String,
    pub connection_type: String,
    pub description: String,
    pub bidirectional: bool,
    pub travel_time: Option<u32>,
}

impl From<LocationConnection> for ConnectionResponseDto {
    fn from(c: LocationConnection) -> Self {
        Self {
            from_location_id: c.from_location.to_string(),
            to_location_id: c.to_location.to_string(),
            connection_type: format!("{:?}", c.connection_type),
            description: c.description,
            bidirectional: c.bidirectional,
            travel_time: c.travel_time,
        }
    }
}

pub fn parse_spatial_relationship(s: &str) -> SpatialRelationship {
    match s {
        "Enters" => SpatialRelationship::Enters,
        "Exits" => SpatialRelationship::Exits,
        "LeadsTo" => SpatialRelationship::LeadsTo,
        "AdjacentTo" => SpatialRelationship::AdjacentTo,
        "ConnectsTo" | _ => SpatialRelationship::ConnectsTo,
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

pub fn backdrop_regions_from_requests(reqs: Vec<BackdropRegionRequestDto>) -> Vec<BackdropRegion> {
    reqs.into_iter()
        .map(|r| {
            let region = BackdropRegion::new(
                r.name,
                RegionBounds {
                    x: r.bounds.x,
                    y: r.bounds.y,
                    width: r.bounds.width,
                    height: r.bounds.height,
                },
                r.backdrop_asset,
            );
            if let Some(desc) = r.description {
                region.with_description(desc)
            } else {
                region
            }
        })
        .collect()
}

