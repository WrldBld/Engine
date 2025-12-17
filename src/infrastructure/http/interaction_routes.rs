//! Interaction API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::InteractionService;
use crate::application::dto::{
    parse_interaction_type, parse_target, CreateInteractionRequestDto, InteractionResponseDto,
    SetAvailabilityRequestDto,
};
use crate::domain::entities::InteractionTemplate;
use crate::domain::value_objects::{InteractionId, SceneId};
use crate::infrastructure::state::AppState;

/// List interactions in a scene
pub async fn list_interactions(
    State(state): State<Arc<AppState>>,
    Path(scene_id): Path<String>,
) -> Result<Json<Vec<InteractionResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&scene_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    let interactions = state
        .core.interaction_service
        .list_interactions(SceneId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        interactions
            .into_iter()
            .map(InteractionResponseDto::from)
            .collect(),
    ))
}

/// Create an interaction in a scene
pub async fn create_interaction(
    State(state): State<Arc<AppState>>,
    Path(scene_id): Path<String>,
    Json(req): Json<CreateInteractionRequestDto>,
) -> Result<(StatusCode, Json<InteractionResponseDto>), (StatusCode, String)> {
    let scene_uuid = Uuid::parse_str(&scene_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;

    let interaction_type = parse_interaction_type(&req.interaction_type);
    let target = parse_target(
        &req.target_type,
        req.target_id.as_deref(),
        req.target_description.as_deref(),
    )
    .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

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
        .core.interaction_service
        .create_interaction(&interaction)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(InteractionResponseDto::from(interaction)),
    ))
}

/// Get an interaction by ID
pub async fn get_interaction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<InteractionResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid interaction ID".to_string(),
        )
    })?;

    let interaction = state
        .core.interaction_service
        .get_interaction(InteractionId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Interaction not found".to_string()))?;

    Ok(Json(InteractionResponseDto::from(interaction)))
}

/// Update an interaction
pub async fn update_interaction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateInteractionRequestDto>,
) -> Result<Json<InteractionResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid interaction ID".to_string(),
        )
    })?;

    let mut interaction = state
        .core.interaction_service
        .get_interaction(InteractionId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Interaction not found".to_string()))?;

    interaction.name = req.name;
    interaction.interaction_type = parse_interaction_type(&req.interaction_type);
    interaction.target = parse_target(
        &req.target_type,
        req.target_id.as_deref(),
        req.target_description.as_deref(),
    )
    .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;
    interaction.prompt_hints = req.prompt_hints;
    interaction.allowed_tools = req.allowed_tools;
    interaction.order = req.order;

    state
        .core.interaction_service
        .update_interaction(&interaction)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(InteractionResponseDto::from(interaction)))
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
        .core.interaction_service
        .delete_interaction(InteractionId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Toggle interaction availability
pub async fn set_interaction_availability(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SetAvailabilityRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid interaction ID".to_string(),
        )
    })?;

    state
        .core.interaction_service
        .set_interaction_availability(InteractionId::from_uuid(uuid), req.available)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
