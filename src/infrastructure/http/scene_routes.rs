//! Scene API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{
    CreateSceneRequest as ServiceCreateSceneRequest, SceneService,
    UpdateSceneRequest as ServiceUpdateSceneRequest,
};
use crate::application::dto::{CreateSceneRequestDto, SceneResponseDto, UpdateNotesRequestDto};
use crate::domain::entities::TimeContext;
use crate::domain::value_objects::{ActId, CharacterId, LocationId, SceneId};
use crate::infrastructure::state::AppState;

/// List scenes in an act
pub async fn list_scenes_by_act(
    State(state): State<Arc<AppState>>,
    Path(act_id): Path<String>,
) -> Result<Json<Vec<SceneResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&act_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid act ID".to_string()))?;

    let scenes = state
        .core.scene_service
        .list_scenes_by_act(ActId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(scenes.into_iter().map(SceneResponseDto::from).collect()))
}

/// Create a scene
pub async fn create_scene(
    State(state): State<Arc<AppState>>,
    Path(act_id): Path<String>,
    Json(req): Json<CreateSceneRequestDto>,
) -> Result<(StatusCode, Json<SceneResponseDto>), (StatusCode, String)> {
    let act_uuid = Uuid::parse_str(&act_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid act ID".to_string()))?;
    let location_uuid = Uuid::parse_str(&req.location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    // Parse featured character IDs
    let featured_characters: Vec<CharacterId> = req
        .featured_characters
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .map(CharacterId::from_uuid)
        .collect();

    let service_request = ServiceCreateSceneRequest {
        act_id: ActId::from_uuid(act_uuid),
        name: req.name,
        location_id: LocationId::from_uuid(location_uuid),
        time_context: req.time_context.map(TimeContext::Custom),
        backdrop_override: req.backdrop_override,
        featured_characters,
        directorial_notes: if req.directorial_notes.is_empty() {
            None
        } else {
            Some(req.directorial_notes)
        },
        entry_conditions: vec![],
        order: req.order,
    };

    let scene = state
        .core.scene_service
        .create_scene(service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(SceneResponseDto::from(scene))))
}

/// Get a scene by ID
pub async fn get_scene(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SceneResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    let scene = state
        .core.scene_service
        .get_scene(SceneId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Scene not found".to_string()))?;

    Ok(Json(SceneResponseDto::from(scene)))
}

/// Update a scene
pub async fn update_scene(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateSceneRequestDto>,
) -> Result<Json<SceneResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;
    let scene_id = SceneId::from_uuid(uuid);

    // Update basic scene fields via service
    let service_request = ServiceUpdateSceneRequest {
        name: Some(req.name),
        time_context: req.time_context.map(TimeContext::Custom),
        backdrop_override: req.backdrop_override,
        entry_conditions: None,
        order: Some(req.order),
    };

    let _scene = state
        .core.scene_service
        .update_scene(scene_id, service_request)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Scene not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    // Update directorial notes if provided
    if !req.directorial_notes.is_empty() {
        state
            .core.scene_service
            .update_directorial_notes(scene_id, req.directorial_notes)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Update featured characters
    let featured_characters: Vec<CharacterId> = req
        .featured_characters
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .map(CharacterId::from_uuid)
        .collect();

    let scene = state
        .core.scene_service
        .update_featured_characters(scene_id, featured_characters)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SceneResponseDto::from(scene)))
}

/// Delete a scene
pub async fn delete_scene(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    state
        .core.scene_service
        .delete_scene(SceneId::from_uuid(uuid))
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Scene not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Update directorial notes for a scene
pub async fn update_directorial_notes(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNotesRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    state
        .core.scene_service
        .update_directorial_notes(SceneId::from_uuid(uuid), req.notes)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Scene not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::OK)
}
