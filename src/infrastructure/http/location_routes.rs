//! Location API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{
    CreateConnectionRequest as ServiceCreateConnectionRequest,
    CreateLocationRequest as ServiceCreateLocationRequest, LocationService,
    UpdateLocationRequest as ServiceUpdateLocationRequest,
};
use crate::application::dto::{
    ConnectionResponseDto, CreateConnectionRequestDto, CreateLocationRequestDto,
    LocationResponseDto, parse_location_type,
};
use crate::domain::value_objects::{LocationId, WorldId};
use crate::infrastructure::state::AppState;

/// List locations in a world
pub async fn list_locations(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<LocationResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let locations = state
        .core.location_service
        .list_locations(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        locations.into_iter().map(LocationResponseDto::from).collect(),
    ))
}

/// Create a location
pub async fn create_location(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateLocationRequestDto>,
) -> Result<(StatusCode, Json<LocationResponseDto>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let location_type = parse_location_type(&req.location_type);

    let parent_id = if let Some(ref parent_id_str) = req.parent_id {
        let parent_uuid = Uuid::parse_str(parent_id_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid parent ID".to_string()))?;
        Some(LocationId::from_uuid(parent_uuid))
    } else {
        None
    };

    let service_request = ServiceCreateLocationRequest {
        world_id: WorldId::from_uuid(uuid),
        name: req.name,
        description: if req.description.is_empty() {
            None
        } else {
            Some(req.description)
        },
        location_type,
        parent_id,
        backdrop_asset: req.backdrop_asset,
        atmosphere: req.atmosphere,
    };

    let location = state
        .core.location_service
        .create_location(service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(LocationResponseDto::from(location))))
}

/// Get a location by ID
pub async fn get_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<LocationResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let location = state
        .core.location_service
        .get_location(LocationId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Location not found".to_string()))?;

    Ok(Json(LocationResponseDto::from(location)))
}

/// Update a location
pub async fn update_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateLocationRequestDto>,
) -> Result<Json<LocationResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    // Handle parent_id update separately via set_parent
    if let Some(ref parent_id_str) = req.parent_id {
        let parent_uuid = Uuid::parse_str(parent_id_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid parent ID".to_string()))?;
        state
            .core.location_service
            .set_parent(LocationId::from_uuid(uuid), Some(LocationId::from_uuid(parent_uuid)))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    let service_request = ServiceUpdateLocationRequest {
        name: Some(req.name),
        description: if req.description.is_empty() {
            None
        } else {
            Some(req.description)
        },
        location_type: Some(parse_location_type(&req.location_type)),
        backdrop_asset: req.backdrop_asset.map(Some),
        atmosphere: req.atmosphere.map(Some),
    };

    let location = state
        .core.location_service
        .update_location(LocationId::from_uuid(uuid), service_request)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Location not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(Json(LocationResponseDto::from(location)))
}

/// Delete a location
pub async fn delete_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    state
        .core.location_service
        .delete_location(LocationId::from_uuid(uuid))
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Location not found".to_string())
            } else if e.to_string().contains("child locations") {
                (StatusCode::CONFLICT, e.to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// List locations available for starting (filtered for PC creation)
pub async fn list_available_starting_locations(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<LocationResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    // Get all locations in the world
    let locations = state
        .core.location_service
        .list_locations(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // For now, return all locations
    // In the future, we could filter by entry conditions that PCs can't meet
    // or locations that are marked as "not available for starting"
    Ok(Json(
        locations.into_iter().map(LocationResponseDto::from).collect(),
    ))
}

// Connection routes

/// Get connections from a location
pub async fn get_connections(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ConnectionResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let connections = state
        .core.location_service
        .get_connections(LocationId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        connections
            .into_iter()
            .map(ConnectionResponseDto::from)
            .collect(),
    ))
}

/// Create a connection between locations
pub async fn create_connection(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConnectionRequestDto>,
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

    let service_request = ServiceCreateConnectionRequest {
        from_location: LocationId::from_uuid(from_uuid),
        to_location: LocationId::from_uuid(to_uuid),
        connection_type: req.connection_type,
        description: req.description,
        bidirectional: req.bidirectional,
        travel_time: req.travel_time,
        is_locked: req.is_locked,
        lock_description: req.lock_description,
    };

    state
        .core.location_service
        .create_connection(service_request)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, e.to_string())
            } else if e.to_string().contains("different worlds") || e.to_string().contains("itself")
            {
                (StatusCode::BAD_REQUEST, e.to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::CREATED)
}
