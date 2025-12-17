//! World API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{
    CreateActRequest as ServiceCreateActRequest, CreateWorldRequest as ServiceCreateWorldRequest,
    UpdateWorldRequest as ServiceUpdateWorldRequest, WorldService,
};
use crate::application::dto::{
    ActResponseDto, CreateActRequestDto, CreateWorldRequestDto, UpdateWorldRequestDto,
    WorldResponseDto, parse_monomyth_stage,
};
use crate::domain::value_objects::WorldId;
use crate::infrastructure::state::AppState;

/// List all worlds
pub async fn list_worlds(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WorldResponseDto>>, (StatusCode, String)> {
    let worlds = state
        .core.world_service
        .list_worlds()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(worlds.into_iter().map(WorldResponseDto::from).collect()))
}

/// Create a new world
pub async fn create_world(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorldRequestDto>,
) -> Result<(StatusCode, Json<WorldResponseDto>), (StatusCode, String)> {
    let service_request = ServiceCreateWorldRequest {
        name: req.name,
        description: req.description,
        rule_system: req.rule_system.map(|r| r.into_domain()),
    };

    let world = state
        .core.world_service
        .create_world(service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(WorldResponseDto::from(world))))
}

/// Get a world by ID
pub async fn get_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<WorldResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let world = state
        .core.world_service
        .get_world(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    Ok(Json(WorldResponseDto::from(world)))
}

/// Update a world
pub async fn update_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorldRequestDto>,
) -> Result<Json<WorldResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let service_request = ServiceUpdateWorldRequest {
        name: Some(req.name),
        description: Some(req.description),
        rule_system: Some(req.rule_system.into()),
    };

    let world = state
        .core.world_service
        .update_world(WorldId::from_uuid(uuid), service_request)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "World not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(Json(WorldResponseDto::from(world)))
}

/// Delete a world
pub async fn delete_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    state
        .core.world_service
        .delete_world(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "World not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

// Act endpoints

/// List acts in a world
pub async fn list_acts(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ActResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let acts = state
        .core.world_service
        .get_acts(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(acts.into_iter().map(ActResponseDto::from).collect()))
}

/// Create an act in a world
pub async fn create_act(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateActRequestDto>,
) -> Result<(StatusCode, Json<ActResponseDto>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let stage = parse_monomyth_stage(&req.stage);
    let service_request = ServiceCreateActRequest {
        name: req.name,
        stage,
        description: if req.description.is_empty() {
            None
        } else {
            Some(req.description)
        },
        order: req.order,
    };

    let act = state
        .core.world_service
        .create_act(WorldId::from_uuid(uuid), service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(ActResponseDto::from(act))))
}
