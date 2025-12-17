//! Skill API routes
//!
//! Endpoints for managing skills within a world.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{
    CreateSkillRequest as ServiceCreateRequest, SkillService,
    UpdateSkillRequest as ServiceUpdateRequest,
};
use crate::domain::value_objects::{SkillId, WorldId};
use crate::application::dto::{CreateSkillRequestDto, SkillResponseDto, UpdateSkillRequestDto};
use crate::infrastructure::state::AppState;

/// List all skills for a world
///
/// If the world has no custom skills yet, returns the default skills for the world's rule system.
pub async fn list_skills(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<SkillResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let skills = state
        .core.skill_service
        .list_skills(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(skills.into_iter().map(SkillResponseDto::from).collect()))
}

/// Create a custom skill for a world
pub async fn create_skill(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateSkillRequestDto>,
) -> Result<(StatusCode, Json<SkillResponseDto>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    // Convert HTTP request to service request
    let service_req = ServiceCreateRequest {
        name: req.name,
        description: req.description,
        category: req.category.into(),
        base_attribute: req.base_attribute,
    };

    let skill = state
        .core.skill_service
        .create_skill(world_id, service_req)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(SkillResponseDto::from(skill))))
}

/// Update a skill
pub async fn update_skill(
    State(state): State<Arc<AppState>>,
    Path((world_id, skill_id)): Path<(String, String)>,
    Json(req): Json<UpdateSkillRequestDto>,
) -> Result<Json<SkillResponseDto>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let skill_uuid = Uuid::parse_str(&skill_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid skill ID".to_string()))?;

    let world_id = WorldId::from_uuid(world_uuid);
    let skill_id = SkillId::from_uuid(skill_uuid);

    // Get existing skill to verify ownership
    let existing_skill = state
        .core.skill_service
        .get_skill(skill_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Skill not found".to_string()))?;

    // Verify skill belongs to the world
    if existing_skill.world_id != world_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Skill does not belong to this world".to_string(),
        ));
    }

    // Convert HTTP request to service request
    let service_req = ServiceUpdateRequest {
        name: req.name,
        description: req.description,
        category: req.category.map(Into::into),
        base_attribute: req.base_attribute,
        is_hidden: req.is_hidden,
        order: req.order,
    };

    let skill = state
        .core.skill_service
        .update_skill(skill_id, service_req)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SkillResponseDto::from(skill)))
}

/// Delete a custom skill
pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path((world_id, skill_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let skill_uuid = Uuid::parse_str(&skill_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid skill ID".to_string()))?;

    let world_id = WorldId::from_uuid(world_uuid);
    let skill_id = SkillId::from_uuid(skill_uuid);

    // Get the skill to verify it exists and belongs to this world
    let skill = state
        .core.skill_service
        .get_skill(skill_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Skill not found".to_string()))?;

    // Verify skill belongs to the world
    if skill.world_id != world_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Skill does not belong to this world".to_string(),
        ));
    }

    // Delete the skill (service will validate it's custom)
    state
        .core.skill_service
        .delete_skill(skill_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Initialize default skills for a world
///
/// This populates the world's skills from its rule system preset.
/// Called when a world is first created with a rule system.
pub async fn initialize_skills(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<SkillResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let skills = state
        .core.skill_service
        .initialize_defaults(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(skills.into_iter().map(SkillResponseDto::from).collect()))
}
