//! Location API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::{
    BackdropRegion, Location, LocationConnection, LocationType, RegionBounds, SpatialRelationship,
};
use crate::domain::value_objects::{LocationId, WorldId};
use crate::infrastructure::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateLocationRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub location_type: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub backdrop_asset: Option<String>,
    #[serde(default)]
    pub backdrop_regions: Vec<BackdropRegionRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackdropRegionRequest {
    pub name: String,
    pub bounds: RegionBoundsRequest,
    pub backdrop_asset: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RegionBoundsRequest {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize)]
pub struct LocationResponse {
    pub id: String,
    pub world_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: String,
    pub location_type: String,
    pub backdrop_asset: Option<String>,
    pub grid_map_id: Option<String>,
    pub backdrop_regions: Vec<BackdropRegionResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackdropRegionResponse {
    pub id: String,
    pub name: String,
    pub bounds: RegionBoundsRequest,
    pub backdrop_asset: String,
    pub description: Option<String>,
}

impl From<BackdropRegion> for BackdropRegionResponse {
    fn from(r: BackdropRegion) -> Self {
        Self {
            id: r.id,
            name: r.name,
            bounds: RegionBoundsRequest {
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

impl From<Location> for LocationResponse {
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
                .map(BackdropRegionResponse::from)
                .collect(),
        }
    }
}

/// List locations in a world
pub async fn list_locations(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<LocationResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let locations = state
        .repository
        .locations()
        .list_by_world(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        locations.into_iter().map(LocationResponse::from).collect(),
    ))
}

/// Create a location
pub async fn create_location(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateLocationRequest>,
) -> Result<(StatusCode, Json<LocationResponse>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let location_type = parse_location_type(&req.location_type);
    let mut location = Location::new(WorldId::from_uuid(uuid), &req.name, location_type);

    if !req.description.is_empty() {
        location = location.with_description(&req.description);
    }
    if let Some(backdrop) = req.backdrop_asset {
        location = location.with_backdrop(&backdrop);
    }
    if let Some(parent_id) = req.parent_id {
        let parent_uuid = Uuid::parse_str(&parent_id)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid parent ID".to_string()))?;
        location = location.with_parent(LocationId::from_uuid(parent_uuid));
    }
    for region_req in req.backdrop_regions {
        let region = BackdropRegion::new(
            region_req.name,
            RegionBounds {
                x: region_req.bounds.x,
                y: region_req.bounds.y,
                width: region_req.bounds.width,
                height: region_req.bounds.height,
            },
            region_req.backdrop_asset,
        );
        let region = if let Some(desc) = region_req.description {
            region.with_description(desc)
        } else {
            region
        };
        location = location.with_backdrop_region(region);
    }

    state
        .repository
        .locations()
        .create(&location)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(LocationResponse::from(location))))
}

/// Get a location by ID
pub async fn get_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<LocationResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let location = state
        .repository
        .locations()
        .get(LocationId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Location not found".to_string()))?;

    Ok(Json(LocationResponse::from(location)))
}

/// Update a location
pub async fn update_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<LocationResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let mut location = state
        .repository
        .locations()
        .get(LocationId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Location not found".to_string()))?;

    location.name = req.name;
    location.description = req.description;
    location.location_type = parse_location_type(&req.location_type);
    location.backdrop_asset = req.backdrop_asset;

    // Update parent_id
    if let Some(parent_id) = req.parent_id {
        let parent_uuid = Uuid::parse_str(&parent_id)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid parent ID".to_string()))?;
        location.parent_id = Some(LocationId::from_uuid(parent_uuid));
    } else {
        location.parent_id = None;
    }

    // Update backdrop regions
    location.backdrop_regions = req
        .backdrop_regions
        .into_iter()
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
        .collect();

    state
        .repository
        .locations()
        .update(&location)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LocationResponse::from(location)))
}

/// Delete a location
pub async fn delete_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    state
        .repository
        .locations()
        .delete(LocationId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// Connection routes

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequest {
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
pub struct ConnectionResponse {
    pub from_location_id: String,
    pub to_location_id: String,
    pub connection_type: String,
    pub description: String,
    pub bidirectional: bool,
    pub travel_time: Option<u32>,
}

impl From<LocationConnection> for ConnectionResponse {
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

/// Get connections from a location
pub async fn get_connections(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ConnectionResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let connections = state
        .repository
        .locations()
        .get_connections(LocationId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        connections
            .into_iter()
            .map(ConnectionResponse::from)
            .collect(),
    ))
}

/// Create a connection between locations
pub async fn create_connection(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConnectionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&req.from_location_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid from location ID".to_string(),
        )
    })?;
    let to_uuid = Uuid::parse_str(&req.to_location_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid to location ID".to_string(),
        )
    })?;

    let mut connection = LocationConnection::new(
        LocationId::from_uuid(from_uuid),
        LocationId::from_uuid(to_uuid),
    );

    // Set connection type if provided
    if let Some(ref ct) = req.connection_type {
        let spatial_rel = parse_spatial_relationship(ct);
        connection = connection.with_connection_type(spatial_rel);
    }

    if !req.description.is_empty() {
        connection = connection.with_description(&req.description);
    }
    if !req.bidirectional {
        connection = connection.one_way();
    }
    connection.travel_time = req.travel_time;

    state
        .repository
        .locations()
        .create_connection(&connection)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

fn parse_spatial_relationship(s: &str) -> SpatialRelationship {
    match s {
        "Enters" => SpatialRelationship::Enters,
        "Exits" => SpatialRelationship::Exits,
        "LeadsTo" => SpatialRelationship::LeadsTo,
        "AdjacentTo" => SpatialRelationship::AdjacentTo,
        "ConnectsTo" | _ => SpatialRelationship::ConnectsTo,
    }
}

fn parse_location_type(s: &str) -> LocationType {
    match s {
        "Interior" => LocationType::Interior,
        "Exterior" => LocationType::Exterior,
        "Abstract" => LocationType::Abstract,
        _ => LocationType::Interior,
    }
}
