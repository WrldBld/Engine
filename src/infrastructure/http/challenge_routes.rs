//! Challenge API routes
//!
//! Endpoints for managing challenges within a world.
//!
//! ## Graph-First Design (Phase 0.E)
//!
//! Challenge relationships are stored as Neo4j edges:
//! - `REQUIRES_SKILL` -> Skill required for this challenge
//! - `TIED_TO_SCENE` -> Scene this challenge appears in
//! - `REQUIRES_COMPLETION_OF` -> Prerequisite challenges

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::dto::{
    ChallengeResponseDto, CreateChallengeRequestDto, UpdateChallengeRequestDto,
};
use crate::application::services::{ChallengeService, WorldService};
use crate::domain::entities::{Challenge, ChallengePrerequisite};
use crate::domain::value_objects::{ChallengeId, SceneId, SkillId, WorldId};
use crate::infrastructure::state::AppState;

// ============================================================================
// Helper Functions
// ============================================================================

/// Build a ChallengeResponseDto from a challenge by fetching edge data
async fn build_challenge_response(
    challenge_service: &dyn ChallengeService,
    challenge: Challenge,
) -> Result<ChallengeResponseDto, (StatusCode, String)> {
    let challenge_id = challenge.id;

    // Fetch edge data in parallel conceptually (sequential here for simplicity)
    let skill_id = challenge_service
        .get_required_skill(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(|s| s.to_string());

    let scene_id = challenge_service
        .get_tied_scene(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(|s| s.to_string());

    let prerequisites = challenge_service
        .get_prerequisites(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let prerequisite_ids: Vec<String> = prerequisites
        .into_iter()
        .map(|p| p.challenge_id.to_string())
        .collect();

    Ok(ChallengeResponseDto::from_challenge_with_edges(
        challenge,
        skill_id,
        scene_id,
        prerequisite_ids,
    ))
}

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
        .game
        .challenge_service
        .list_challenges(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // For list views, use minimal response (no edge data) for performance
    Ok(Json(
        challenges
            .into_iter()
            .map(ChallengeResponseDto::from_challenge_minimal)
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
        .game
        .challenge_service
        .list_by_scene(scene_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        challenges
            .into_iter()
            .map(ChallengeResponseDto::from_challenge_minimal)
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
        .game
        .challenge_service
        .list_active(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        challenges
            .into_iter()
            .map(ChallengeResponseDto::from_challenge_minimal)
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
        .game
        .challenge_service
        .list_favorites(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        challenges
            .into_iter()
            .map(ChallengeResponseDto::from_challenge_minimal)
            .collect(),
    ))
}

/// Get a single challenge (with full edge data)
pub async fn get_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<String>,
) -> Result<Json<ChallengeResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&challenge_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid challenge ID".to_string()))?;
    let challenge_id = ChallengeId::from_uuid(uuid);

    let challenge = state
        .game
        .challenge_service
        .get_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Challenge not found".to_string()))?;

    let response =
        build_challenge_response(&state.game.challenge_service, challenge).await?;
    Ok(Json(response))
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
        .core
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

    // Build the challenge (without embedded relationships)
    let mut challenge = Challenge::new(world_id, req.name, req.difficulty.into())
        .with_description(req.description)
        .with_challenge_type(req.challenge_type.into())
        .with_outcomes(req.outcomes.into());

    for tc in req.trigger_conditions {
        challenge = challenge.with_trigger(tc.into());
    }

    for tag in req.tags {
        challenge = challenge.with_tag(tag);
    }

    // Save the challenge first
    let challenge = state
        .game
        .challenge_service
        .create_challenge(challenge)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let challenge_id = challenge.id;

    // Now create the edge relationships
    // Set required skill
    state
        .game
        .challenge_service
        .set_required_skill(challenge_id, skill_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Tie to scene if provided
    if let Some(sid) = scene_id {
        state
            .game
            .challenge_service
            .tie_to_scene(challenge_id, sid)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Add prerequisites
    for prereq_id in prerequisites {
        state
            .game
            .challenge_service
            .add_prerequisite(challenge_id, ChallengePrerequisite::new(prereq_id))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Build the response with edge data
    let response =
        build_challenge_response(&state.game.challenge_service, challenge).await?;

    Ok((StatusCode::CREATED, Json(response)))
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
        .game
        .challenge_service
        .get_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Challenge not found".to_string()))?;

    // Apply updates to node properties
    if let Some(name) = req.name {
        challenge.name = name;
    }
    if let Some(description) = req.description {
        challenge.description = description;
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

    // Save node property updates
    let challenge = state
        .game
        .challenge_service
        .update_challenge(challenge)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Handle edge updates
    // Update skill if provided
    if let Some(skill_id_str) = req.skill_id {
        let skill_uuid = Uuid::parse_str(&skill_id_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid skill ID".to_string()))?;
        state
            .game
            .challenge_service
            .set_required_skill(challenge_id, SkillId::from_uuid(skill_uuid))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Update scene tie if provided
    if let Some(scene_id_str) = req.scene_id {
        if scene_id_str.is_empty() {
            // Remove scene tie
            state
                .game
                .challenge_service
                .untie_from_scene(challenge_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        } else {
            let scene_uuid = Uuid::parse_str(&scene_id_str)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;
            state
                .game
                .challenge_service
                .tie_to_scene(challenge_id, SceneId::from_uuid(scene_uuid))
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    // Update prerequisites if provided (replace all)
    if let Some(prereq_strs) = req.prerequisite_challenges {
        // Get current prerequisites
        let current_prereqs = state
            .game
            .challenge_service
            .get_prerequisites(challenge_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Remove all current prerequisites
        for prereq in current_prereqs {
            state
                .game
                .challenge_service
                .remove_prerequisite(challenge_id, prereq.challenge_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        // Add new prerequisites
        for prereq_str in prereq_strs {
            if let Ok(prereq_uuid) = Uuid::parse_str(&prereq_str) {
                state
                    .game
                    .challenge_service
                    .add_prerequisite(
                        challenge_id,
                        ChallengePrerequisite::new(ChallengeId::from_uuid(prereq_uuid)),
                    )
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }
    }

    // Build response with edge data
    let response =
        build_challenge_response(&state.game.challenge_service, challenge).await?;
    Ok(Json(response))
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
        .game
        .challenge_service
        .get_challenge(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Challenge not found".to_string()))?;

    // Delete it (DETACH DELETE removes all edges)
    state
        .game
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
        .game
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
        .game
        .challenge_service
        .set_active(challenge_id, active)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
