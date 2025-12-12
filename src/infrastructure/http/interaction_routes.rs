//! Interaction API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::{
    InteractionTarget, InteractionTemplate, InteractionType,
};
use crate::domain::value_objects::{CharacterId, InteractionId, ItemId, SceneId};
use crate::infrastructure::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateInteractionRequest {
    pub name: String,
    pub interaction_type: String,
    #[serde(default)]
    pub target_type: String,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub target_description: Option<String>,
    #[serde(default)]
    pub prompt_hints: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub order: u32,
}

#[derive(Debug, Serialize)]
pub struct InteractionResponse {
    pub id: String,
    pub scene_id: String,
    pub name: String,
    pub interaction_type: String,
    pub target: String,
    pub prompt_hints: String,
    pub allowed_tools: Vec<String>,
    pub is_available: bool,
    pub order: u32,
}

impl From<InteractionTemplate> for InteractionResponse {
    fn from(i: InteractionTemplate) -> Self {
        Self {
            id: i.id.to_string(),
            scene_id: i.scene_id.to_string(),
            name: i.name,
            interaction_type: format!("{:?}", i.interaction_type),
            target: format!("{:?}", i.target),
            prompt_hints: i.prompt_hints,
            allowed_tools: i.allowed_tools,
            is_available: i.is_available,
            order: i.order,
        }
    }
}

/// List interactions in a scene
pub async fn list_interactions(
    State(state): State<Arc<AppState>>,
    Path(scene_id): Path<String>,
) -> Result<Json<Vec<InteractionResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&scene_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    let interactions = state
        .repository
        .interactions()
        .list_by_scene(SceneId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        interactions
            .into_iter()
            .map(InteractionResponse::from)
            .collect(),
    ))
}

/// Create an interaction in a scene
pub async fn create_interaction(
    State(state): State<Arc<AppState>>,
    Path(scene_id): Path<String>,
    Json(req): Json<CreateInteractionRequest>,
) -> Result<(StatusCode, Json<InteractionResponse>), (StatusCode, String)> {
    let scene_uuid = Uuid::parse_str(&scene_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    let interaction_type = parse_interaction_type(&req.interaction_type);
    let target = parse_target(
        &req.target_type,
        req.target_id.as_deref(),
        req.target_description.as_deref(),
    )?;

    let mut interaction = InteractionTemplate::new(
        SceneId::from_uuid(scene_uuid),
        &req.name,
        interaction_type,
        target,
    );

    if !req.prompt_hints.is_empty() {
        interaction = interaction.with_prompt_hints(&req.prompt_hints);
    }

    for tool in req.allowed_tools {
        interaction = interaction.with_allowed_tool(tool);
    }

    interaction = interaction.with_order(req.order);

    state
        .repository
        .interactions()
        .create(&interaction)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(InteractionResponse::from(interaction)),
    ))
}

/// Get an interaction by ID
pub async fn get_interaction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<InteractionResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid interaction ID".to_string(),
        )
    })?;

    let interaction = state
        .repository
        .interactions()
        .get(InteractionId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Interaction not found".to_string()))?;

    Ok(Json(InteractionResponse::from(interaction)))
}

/// Update an interaction
pub async fn update_interaction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateInteractionRequest>,
) -> Result<Json<InteractionResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid interaction ID".to_string(),
        )
    })?;

    let mut interaction = state
        .repository
        .interactions()
        .get(InteractionId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Interaction not found".to_string()))?;

    interaction.name = req.name;
    interaction.interaction_type = parse_interaction_type(&req.interaction_type);
    interaction.target = parse_target(
        &req.target_type,
        req.target_id.as_deref(),
        req.target_description.as_deref(),
    )?;
    interaction.prompt_hints = req.prompt_hints;
    interaction.allowed_tools = req.allowed_tools;
    interaction.order = req.order;

    state
        .repository
        .interactions()
        .update(&interaction)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(InteractionResponse::from(interaction)))
}

/// Delete an interaction
pub async fn delete_interaction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid interaction ID".to_string(),
        )
    })?;

    state
        .repository
        .interactions()
        .delete(InteractionId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SetAvailabilityRequest {
    pub available: bool,
}

/// Toggle interaction availability
pub async fn set_interaction_availability(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SetAvailabilityRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid interaction ID".to_string(),
        )
    })?;

    state
        .repository
        .interactions()
        .set_availability(InteractionId::from_uuid(uuid), req.available)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

fn parse_interaction_type(s: &str) -> InteractionType {
    match s {
        "Dialogue" => InteractionType::Dialogue,
        "Examine" => InteractionType::Examine,
        "UseItem" => InteractionType::UseItem,
        "PickUp" => InteractionType::PickUp,
        "GiveItem" => InteractionType::GiveItem,
        "Attack" => InteractionType::Attack,
        "Travel" => InteractionType::Travel,
        other => InteractionType::Custom(other.to_string()),
    }
}

fn parse_target(
    target_type: &str,
    target_id: Option<&str>,
    description: Option<&str>,
) -> Result<InteractionTarget, (StatusCode, String)> {
    match target_type {
        "Character" => {
            let id = target_id.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "Character target requires target_id".to_string(),
                )
            })?;
            let uuid = Uuid::parse_str(id)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
            Ok(InteractionTarget::Character(CharacterId::from_uuid(uuid)))
        }
        "Item" => {
            let id = target_id.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "Item target requires target_id".to_string(),
                )
            })?;
            let uuid = Uuid::parse_str(id)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid item ID".to_string()))?;
            Ok(InteractionTarget::Item(ItemId::from_uuid(uuid)))
        }
        "Environment" => {
            let desc = description.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "Environment target requires target_description".to_string(),
                )
            })?;
            Ok(InteractionTarget::Environment(desc.to_string()))
        }
        "None" | "" => Ok(InteractionTarget::None),
        _ => Ok(InteractionTarget::None),
    }
}
