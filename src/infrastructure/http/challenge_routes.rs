//! Challenge API routes
//!
//! Endpoints for managing challenges within a world.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{ChallengeService, WorldService};
use crate::application::dto::{
    ChallengeResponseDto, CreateChallengeRequestDto, UpdateChallengeRequestDto,
};
use crate::domain::entities::Challenge;
use crate::domain::value_objects::{ChallengeId, SceneId, SkillId, WorldId};
use crate::infrastructure::state::AppState;
// NOTE: challenge request/response DTOs + conversions live in `application/dto/challenge.rs`.

// ============================================================================
// Handlers
// ============================================================================

/// List all challenges for a world
pub async fn list_challenges(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<ChallengeResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let challenges = state
        .challenge_service
        .list_challenges(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        challenges
            .into_iter()
            .map(ChallengeResponseDto::from)
            .collect(),
    ))
}

/// List challenges for a specific scene
pub async fn list_scene_challenges(
    State(state): State<Arc<AppState>>,
    Path(scene_id): Path<String>,
) -> Result<Json<Vec<ChallengeResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&scene_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;
    let scene_id = SceneId::from_uuid(uuid);

    let challenges = state
        .challenge_service
        .list_by_scene(scene_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        challenges
            .into_iter()
            .map(ChallengeResponseDto::from)
            .collect(),
    ))
}

/// List active challenges for a world (for LLM context)
pub async fn list_active_challenges(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<ChallengeResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let challenges = state
        .challenge_service
        .list_active(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        challenges
            .into_iter()
            .map(ChallengeResponseDto::from)
            .collect(),
    ))
}

/// List favorite challenges
pub async fn list_favorite_challenges(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<ChallengeResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let challenges = state
        .challenge_service
        .list_favorites(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        challenges
            .into_iter()
            .map(ChallengeResponseDto::from)
            .collect(),
    ))
}

/// Get a single challenge
pub async fn get_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<String>,
) -> Result<Json<ChallengeResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&challenge_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid challenge ID".to_string()))?;
    let challenge_id = ChallengeId::from_uuid(uuid);

    let challenge = state
        .challenge_service
        .get_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Challenge not found".to_string()))?;

    Ok(Json(ChallengeResponseDto::from(challenge)))
}

/// Create a new challenge
pub async fn create_challenge(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateChallengeRequestDto>,
) -> Result<(StatusCode, Json<ChallengeResponseDto>), (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);

    // Verify world exists
    let _ = state
        .world_service
        .get_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    // Parse skill ID
    let skill_uuid = Uuid::parse_str(&req.skill_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid skill ID".to_string()))?;
    let skill_id = SkillId::from_uuid(skill_uuid);

    // Parse scene ID if provided
    let scene_id = if let Some(ref sid) = req.scene_id {
        Some(
            Uuid::parse_str(sid)
                .map(SceneId::from_uuid)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?,
        )
    } else {
        None
    };

    // Parse prerequisite challenge IDs
    let prerequisites: Vec<ChallengeId> = req
        .prerequisite_challenges
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok().map(ChallengeId::from_uuid))
        .collect();

    // Build the challenge
    let mut challenge = Challenge::new(world_id, req.name, skill_id, req.difficulty.into())
        .with_description(req.description)
        .with_challenge_type(req.challenge_type.into())
        .with_outcomes(req.outcomes.into());

    if let Some(sid) = scene_id {
        challenge = challenge.with_scene(sid);
    }

    for tc in req.trigger_conditions {
        challenge = challenge.with_trigger(tc.into());
    }

    for prereq in prerequisites {
        challenge = challenge.with_prerequisite(prereq);
    }

    for tag in req.tags {
        challenge = challenge.with_tag(tag);
    }

    // Save via service
    let challenge = state
        .challenge_service
        .create_challenge(challenge)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(ChallengeResponseDto::from(challenge))))
}

/// Update a challenge
pub async fn update_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<String>,
    Json(req): Json<UpdateChallengeRequestDto>,
) -> Result<Json<ChallengeResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&challenge_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid challenge ID".to_string()))?;
    let challenge_id = ChallengeId::from_uuid(uuid);

    // Get existing challenge
    let mut challenge = state
        .challenge_service
        .get_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Challenge not found".to_string()))?;

    // Apply updates
    if let Some(name) = req.name {
        challenge.name = name;
    }
    if let Some(description) = req.description {
        challenge.description = description;
    }
    if let Some(skill_id) = req.skill_id {
        let skill_uuid = Uuid::parse_str(&skill_id)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid skill ID".to_string()))?;
        challenge.skill_id = SkillId::from_uuid(skill_uuid);
    }
    if let Some(scene_id) = req.scene_id {
        if scene_id.is_empty() {
            challenge.scene_id = None;
        } else {
            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;
            challenge.scene_id = Some(SceneId::from_uuid(scene_uuid));
        }
    }
    if let Some(challenge_type) = req.challenge_type {
        challenge.challenge_type = challenge_type.into();
    }
    if let Some(difficulty) = req.difficulty {
        challenge.difficulty = difficulty.into();
    }
    if let Some(outcomes) = req.outcomes {
        challenge.outcomes = outcomes.into();
    }
    if let Some(trigger_conditions) = req.trigger_conditions {
        challenge.trigger_conditions = trigger_conditions.into_iter().map(Into::into).collect();
    }
    if let Some(prerequisites) = req.prerequisite_challenges {
        challenge.prerequisite_challenges = prerequisites
            .iter()
            .filter_map(|s| Uuid::parse_str(s).ok().map(ChallengeId::from_uuid))
            .collect();
    }
    if let Some(active) = req.active {
        challenge.active = active;
    }
    if let Some(order) = req.order {
        challenge.order = order;
    }
    if let Some(is_favorite) = req.is_favorite {
        challenge.is_favorite = is_favorite;
    }
    if let Some(tags) = req.tags {
        challenge.tags = tags;
    }

    // Save updates
    let challenge = state
        .challenge_service
        .update_challenge(challenge)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ChallengeResponseDto::from(challenge)))
}

/// Delete a challenge
pub async fn delete_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&challenge_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid challenge ID".to_string()))?;
    let challenge_id = ChallengeId::from_uuid(uuid);

    // Verify challenge exists
    let _ = state
        .challenge_service
        .get_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Challenge not found".to_string()))?;

    // Delete it
    state
        .challenge_service
        .delete_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Toggle favorite status for a challenge
pub async fn toggle_favorite(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<String>,
) -> Result<Json<bool>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&challenge_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid challenge ID".to_string()))?;
    let challenge_id = ChallengeId::from_uuid(uuid);

    let is_favorite = state
        .challenge_service
        .toggle_favorite(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(is_favorite))
}

/// Set active status for a challenge
pub async fn set_active(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<String>,
    Json(active): Json<bool>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&challenge_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid challenge ID".to_string()))?;
    let challenge_id = ChallengeId::from_uuid(uuid);

    state
        .challenge_service
        .set_active(challenge_id, active)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
