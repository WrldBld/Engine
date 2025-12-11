//! Scene API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::{Scene, TimeContext};
use crate::domain::value_objects::{ActId, CharacterId, LocationId, SceneId};
use crate::infrastructure::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateSceneRequest {
    pub name: String,
    pub location_id: String,
    #[serde(default)]
    pub time_context: Option<String>,
    #[serde(default)]
    pub backdrop_override: Option<String>,
    #[serde(default)]
    pub featured_characters: Vec<String>,
    #[serde(default)]
    pub directorial_notes: String,
    #[serde(default)]
    pub order: u32,
}

#[derive(Debug, Serialize)]
pub struct SceneResponse {
    pub id: String,
    pub act_id: String,
    pub name: String,
    pub location_id: String,
    pub time_context: String,
    pub backdrop_override: Option<String>,
    pub featured_characters: Vec<String>,
    pub directorial_notes: String,
    pub order: u32,
}

impl From<Scene> for SceneResponse {
    fn from(s: Scene) -> Self {
        Self {
            id: s.id.to_string(),
            act_id: s.act_id.to_string(),
            name: s.name,
            location_id: s.location_id.to_string(),
            time_context: format!("{:?}", s.time_context),
            backdrop_override: s.backdrop_override,
            featured_characters: s
                .featured_characters
                .iter()
                .map(|c| c.to_string())
                .collect(),
            directorial_notes: s.directorial_notes,
            order: s.order,
        }
    }
}

/// List scenes in an act
pub async fn list_scenes_by_act(
    State(state): State<Arc<AppState>>,
    Path(act_id): Path<String>,
) -> Result<Json<Vec<SceneResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&act_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid act ID".to_string()))?;

    let scenes = state
        .repository
        .scenes()
        .list_by_act(ActId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(scenes.into_iter().map(SceneResponse::from).collect()))
}

/// Create a scene
pub async fn create_scene(
    State(state): State<Arc<AppState>>,
    Path(act_id): Path<String>,
    Json(req): Json<CreateSceneRequest>,
) -> Result<(StatusCode, Json<SceneResponse>), (StatusCode, String)> {
    let act_uuid = Uuid::parse_str(&act_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid act ID".to_string()))?;
    let location_uuid = Uuid::parse_str(&req.location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let mut scene = Scene::new(
        ActId::from_uuid(act_uuid),
        &req.name,
        LocationId::from_uuid(location_uuid),
    );

    if let Some(time) = req.time_context {
        scene = scene.with_time(TimeContext::Custom(time));
    }
    if !req.directorial_notes.is_empty() {
        scene = scene.with_directorial_notes(&req.directorial_notes);
    }

    scene.backdrop_override = req.backdrop_override;
    scene.order = req.order;

    // Parse featured character IDs
    for char_id_str in &req.featured_characters {
        if let Ok(char_uuid) = Uuid::parse_str(char_id_str) {
            scene = scene.with_character(CharacterId::from_uuid(char_uuid));
        }
    }

    state
        .repository
        .scenes()
        .create(&scene)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(SceneResponse::from(scene))))
}

/// Get a scene by ID
pub async fn get_scene(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SceneResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    let scene = state
        .repository
        .scenes()
        .get(SceneId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Scene not found".to_string()))?;

    Ok(Json(SceneResponse::from(scene)))
}

/// Update a scene
pub async fn update_scene(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateSceneRequest>,
) -> Result<Json<SceneResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;
    let location_uuid = Uuid::parse_str(&req.location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let mut scene = state
        .repository
        .scenes()
        .get(SceneId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Scene not found".to_string()))?;

    scene.name = req.name;
    scene.location_id = LocationId::from_uuid(location_uuid);
    scene.backdrop_override = req.backdrop_override;
    scene.directorial_notes = req.directorial_notes;
    scene.order = req.order;

    if let Some(time) = req.time_context {
        scene.time_context = TimeContext::Custom(time);
    }

    // Update featured characters
    scene.featured_characters = req
        .featured_characters
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .map(CharacterId::from_uuid)
        .collect();

    state
        .repository
        .scenes()
        .update(&scene)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SceneResponse::from(scene)))
}

/// Delete a scene
pub async fn delete_scene(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    state
        .repository
        .scenes()
        .delete(SceneId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotesRequest {
    pub notes: String,
}

/// Update directorial notes for a scene
pub async fn update_directorial_notes(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNotesRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    state
        .repository
        .scenes()
        .update_directorial_notes(SceneId::from_uuid(uuid), &req.notes)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
