//! Skill API routes
//!
//! Endpoints for managing skills within a world.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::{default_skills_for_variant, Skill, SkillCategory};
use crate::domain::value_objects::{SkillId, WorldId};
use crate::infrastructure::state::AppState;

/// Request to create a custom skill
#[derive(Debug, Deserialize)]
pub struct CreateSkillRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub category: SkillCategory,
    pub base_attribute: Option<String>,
}

/// Request to update a skill
#[derive(Debug, Deserialize)]
pub struct UpdateSkillRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<SkillCategory>,
    #[serde(default)]
    pub base_attribute: Option<String>,
    #[serde(default)]
    pub is_hidden: Option<bool>,
    #[serde(default)]
    pub order: Option<u32>,
}

/// Skill response
#[derive(Debug, Serialize)]
pub struct SkillResponse {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub base_attribute: Option<String>,
    pub is_custom: bool,
    pub is_hidden: bool,
    pub order: u32,
}

impl From<Skill> for SkillResponse {
    fn from(skill: Skill) -> Self {
        Self {
            id: skill.id.to_string(),
            world_id: skill.world_id.to_string(),
            name: skill.name,
            description: skill.description,
            category: skill.category,
            base_attribute: skill.base_attribute,
            is_custom: skill.is_custom,
            is_hidden: skill.is_hidden,
            order: skill.order,
        }
    }
}

/// List all skills for a world
///
/// If the world has no custom skills yet, returns the default skills for the world's rule system.
pub async fn list_skills(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<SkillResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    // Get the world to check its rule system
    let world = state
        .repository
        .worlds()
        .get(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    // Try to get skills from repository
    let skills = state
        .repository
        .skills()
        .list_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // If no skills exist, generate default skills based on rule system variant
    let skills = if skills.is_empty() {
        default_skills_for_variant(world_id, &world.rule_system.variant)
    } else {
        skills
    };

    Ok(Json(skills.into_iter().map(SkillResponse::from).collect()))
}

/// Create a custom skill for a world
pub async fn create_skill(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateSkillRequest>,
) -> Result<(StatusCode, Json<SkillResponse>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    // Verify world exists
    let _ = state
        .repository
        .worlds()
        .get(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    // Create the custom skill
    let mut skill = Skill::custom(world_id, &req.name, req.category)
        .with_description(&req.description);

    if let Some(attr) = req.base_attribute {
        skill = skill.with_base_attribute(attr);
    }

    // Save to repository
    state
        .repository
        .skills()
        .create(&skill)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(SkillResponse::from(skill))))
}

/// Update a skill
pub async fn update_skill(
    State(state): State<Arc<AppState>>,
    Path((world_id, skill_id)): Path<(String, String)>,
    Json(req): Json<UpdateSkillRequest>,
) -> Result<Json<SkillResponse>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let skill_uuid = Uuid::parse_str(&skill_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid skill ID".to_string()))?;

    let world_id = WorldId::from_uuid(world_uuid);
    let skill_id = SkillId::from_uuid(skill_uuid);

    // Get existing skill
    let mut skill = state
        .repository
        .skills()
        .get(skill_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Skill not found".to_string()))?;

    // Verify skill belongs to the world
    if skill.world_id != world_id {
        return Err((StatusCode::FORBIDDEN, "Skill does not belong to this world".to_string()));
    }

    // Apply updates
    if let Some(name) = req.name {
        skill.name = name;
    }
    if let Some(description) = req.description {
        skill.description = description;
    }
    if let Some(category) = req.category {
        skill.category = category;
    }
    if let Some(base_attribute) = req.base_attribute {
        skill.base_attribute = Some(base_attribute);
    }
    if let Some(is_hidden) = req.is_hidden {
        skill.is_hidden = is_hidden;
    }
    if let Some(order) = req.order {
        skill.order = order;
    }

    // Save updates
    state
        .repository
        .skills()
        .update(&skill)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SkillResponse::from(skill)))
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

    // Get the skill to verify it exists and is custom
    let skill = state
        .repository
        .skills()
        .get(skill_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Skill not found".to_string()))?;

    // Verify skill belongs to the world
    if skill.world_id != world_id {
        return Err((StatusCode::FORBIDDEN, "Skill does not belong to this world".to_string()));
    }

    // Only allow deleting custom skills
    if !skill.is_custom {
        return Err((StatusCode::FORBIDDEN, "Cannot delete default skills. Hide them instead.".to_string()));
    }

    // Delete the skill
    state
        .repository
        .skills()
        .delete(skill_id)
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
) -> Result<Json<Vec<SkillResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    // Get the world
    let world = state
        .repository
        .worlds()
        .get(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    // Generate default skills
    let skills = default_skills_for_variant(world_id, &world.rule_system.variant);

    // Save all skills
    for skill in &skills {
        state
            .repository
            .skills()
            .create(skill)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(skills.into_iter().map(SkillResponse::from).collect()))
}
