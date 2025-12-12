//! Challenge API routes
//!
//! Endpoints for managing challenges within a world.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::{
    Challenge, ChallengeOutcomes, ChallengeType, Difficulty, DifficultyDescriptor, Outcome,
    OutcomeTrigger, TriggerCondition, TriggerType,
};
use crate::domain::value_objects::{ChallengeId, SceneId, SkillId, WorldId};
use crate::infrastructure::state::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a challenge
#[derive(Debug, Deserialize)]
pub struct CreateChallengeRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub skill_id: String,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub challenge_type: ChallengeType,
    pub difficulty: DifficultyRequest,
    #[serde(default)]
    pub outcomes: OutcomesRequest,
    #[serde(default)]
    pub trigger_conditions: Vec<TriggerConditionRequest>,
    #[serde(default)]
    pub prerequisite_challenges: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Request to update a challenge
#[derive(Debug, Deserialize)]
pub struct UpdateChallengeRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub skill_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub challenge_type: Option<ChallengeType>,
    #[serde(default)]
    pub difficulty: Option<DifficultyRequest>,
    #[serde(default)]
    pub outcomes: Option<OutcomesRequest>,
    #[serde(default)]
    pub trigger_conditions: Option<Vec<TriggerConditionRequest>>,
    #[serde(default)]
    pub prerequisite_challenges: Option<Vec<String>>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub order: Option<u32>,
    #[serde(default)]
    pub is_favorite: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Difficulty request variants
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DifficultyRequest {
    Dc { value: u32 },
    Percentage { value: u32 },
    Descriptor { value: String },
    Opposed,
    Custom { value: String },
}

impl From<DifficultyRequest> for Difficulty {
    fn from(req: DifficultyRequest) -> Self {
        match req {
            DifficultyRequest::Dc { value } => Difficulty::DC(value),
            DifficultyRequest::Percentage { value } => Difficulty::Percentage(value),
            DifficultyRequest::Descriptor { value } => {
                let descriptor = match value.to_lowercase().as_str() {
                    "trivial" => DifficultyDescriptor::Trivial,
                    "easy" => DifficultyDescriptor::Easy,
                    "routine" => DifficultyDescriptor::Routine,
                    "moderate" => DifficultyDescriptor::Moderate,
                    "challenging" => DifficultyDescriptor::Challenging,
                    "hard" => DifficultyDescriptor::Hard,
                    "very_hard" | "veryhard" => DifficultyDescriptor::VeryHard,
                    "extreme" => DifficultyDescriptor::Extreme,
                    "impossible" => DifficultyDescriptor::Impossible,
                    "risky" => DifficultyDescriptor::Risky,
                    "desperate" => DifficultyDescriptor::Desperate,
                    _ => DifficultyDescriptor::Moderate,
                };
                Difficulty::Descriptor(descriptor)
            }
            DifficultyRequest::Opposed => Difficulty::Opposed,
            DifficultyRequest::Custom { value } => Difficulty::Custom(value),
        }
    }
}

impl From<Difficulty> for DifficultyRequest {
    fn from(d: Difficulty) -> Self {
        match d {
            Difficulty::DC(v) => DifficultyRequest::Dc { value: v },
            Difficulty::Percentage(v) => DifficultyRequest::Percentage { value: v },
            Difficulty::Descriptor(d) => DifficultyRequest::Descriptor {
                value: format!("{:?}", d).to_lowercase(),
            },
            Difficulty::Opposed => DifficultyRequest::Opposed,
            Difficulty::Custom(s) => DifficultyRequest::Custom { value: s },
        }
    }
}

/// Outcomes request
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OutcomesRequest {
    pub success: OutcomeRequest,
    pub failure: OutcomeRequest,
    #[serde(default)]
    pub partial: Option<OutcomeRequest>,
    #[serde(default)]
    pub critical_success: Option<OutcomeRequest>,
    #[serde(default)]
    pub critical_failure: Option<OutcomeRequest>,
}

impl From<OutcomesRequest> for ChallengeOutcomes {
    fn from(req: OutcomesRequest) -> Self {
        Self {
            success: req.success.into(),
            failure: req.failure.into(),
            partial: req.partial.map(Into::into),
            critical_success: req.critical_success.map(Into::into),
            critical_failure: req.critical_failure.map(Into::into),
        }
    }
}

impl From<ChallengeOutcomes> for OutcomesRequest {
    fn from(o: ChallengeOutcomes) -> Self {
        Self {
            success: o.success.into(),
            failure: o.failure.into(),
            partial: o.partial.map(Into::into),
            critical_success: o.critical_success.map(Into::into),
            critical_failure: o.critical_failure.map(Into::into),
        }
    }
}

/// Single outcome request
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OutcomeRequest {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub triggers: Vec<OutcomeTriggerRequest>,
}

impl From<OutcomeRequest> for Outcome {
    fn from(req: OutcomeRequest) -> Self {
        Self {
            description: req.description,
            triggers: req.triggers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Outcome> for OutcomeRequest {
    fn from(o: Outcome) -> Self {
        Self {
            description: o.description,
            triggers: o.triggers.into_iter().map(Into::into).collect(),
        }
    }
}

/// Outcome trigger request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutcomeTriggerRequest {
    RevealInformation { info: String, persist: bool },
    EnableChallenge { challenge_id: String },
    DisableChallenge { challenge_id: String },
    ModifyCharacterStat { stat: String, modifier: i32 },
    TriggerScene { scene_id: String },
    GiveItem { item_name: String, item_description: Option<String> },
    Custom { description: String },
}

impl From<OutcomeTriggerRequest> for OutcomeTrigger {
    fn from(req: OutcomeTriggerRequest) -> Self {
        match req {
            OutcomeTriggerRequest::RevealInformation { info, persist } => {
                OutcomeTrigger::RevealInformation { info, persist }
            }
            OutcomeTriggerRequest::EnableChallenge { challenge_id } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                OutcomeTrigger::EnableChallenge { challenge_id: id }
            }
            OutcomeTriggerRequest::DisableChallenge { challenge_id } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                OutcomeTrigger::DisableChallenge { challenge_id: id }
            }
            OutcomeTriggerRequest::ModifyCharacterStat { stat, modifier } => {
                OutcomeTrigger::ModifyCharacterStat { stat, modifier }
            }
            OutcomeTriggerRequest::TriggerScene { scene_id } => {
                let id = Uuid::parse_str(&scene_id)
                    .map(SceneId::from_uuid)
                    .unwrap_or_else(|_| SceneId::new());
                OutcomeTrigger::TriggerScene { scene_id: id }
            }
            OutcomeTriggerRequest::GiveItem { item_name, item_description } => {
                OutcomeTrigger::GiveItem { item_name, item_description }
            }
            OutcomeTriggerRequest::Custom { description } => {
                OutcomeTrigger::Custom { description }
            }
        }
    }
}

impl From<OutcomeTrigger> for OutcomeTriggerRequest {
    fn from(t: OutcomeTrigger) -> Self {
        match t {
            OutcomeTrigger::RevealInformation { info, persist } => {
                OutcomeTriggerRequest::RevealInformation { info, persist }
            }
            OutcomeTrigger::EnableChallenge { challenge_id } => {
                OutcomeTriggerRequest::EnableChallenge {
                    challenge_id: challenge_id.to_string(),
                }
            }
            OutcomeTrigger::DisableChallenge { challenge_id } => {
                OutcomeTriggerRequest::DisableChallenge {
                    challenge_id: challenge_id.to_string(),
                }
            }
            OutcomeTrigger::ModifyCharacterStat { stat, modifier } => {
                OutcomeTriggerRequest::ModifyCharacterStat { stat, modifier }
            }
            OutcomeTrigger::TriggerScene { scene_id } => {
                OutcomeTriggerRequest::TriggerScene {
                    scene_id: scene_id.to_string(),
                }
            }
            OutcomeTrigger::GiveItem { item_name, item_description } => {
                OutcomeTriggerRequest::GiveItem { item_name, item_description }
            }
            OutcomeTrigger::Custom { description } => {
                OutcomeTriggerRequest::Custom { description }
            }
        }
    }
}

/// Trigger condition request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TriggerConditionRequest {
    pub condition_type: TriggerTypeRequest,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

impl From<TriggerConditionRequest> for TriggerCondition {
    fn from(req: TriggerConditionRequest) -> Self {
        let mut tc = TriggerCondition::new(req.condition_type.into(), req.description);
        if req.required {
            tc = tc.required();
        }
        tc
    }
}

impl From<TriggerCondition> for TriggerConditionRequest {
    fn from(tc: TriggerCondition) -> Self {
        Self {
            condition_type: tc.condition_type.into(),
            description: tc.description,
            required: tc.required,
        }
    }
}

/// Trigger type request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerTypeRequest {
    ObjectInteraction { keywords: Vec<String> },
    EnterArea { keywords: Vec<String> },
    DialogueTopic { keywords: Vec<String> },
    ChallengeComplete { challenge_id: String, requires_success: Option<bool> },
    TimeBased { turns: u32 },
    NpcPresent { keywords: Vec<String> },
    Custom { description: String },
}

impl From<TriggerTypeRequest> for TriggerType {
    fn from(req: TriggerTypeRequest) -> Self {
        match req {
            TriggerTypeRequest::ObjectInteraction { keywords } => {
                TriggerType::ObjectInteraction { keywords }
            }
            TriggerTypeRequest::EnterArea { keywords } => {
                TriggerType::EnterArea { area_keywords: keywords }
            }
            TriggerTypeRequest::DialogueTopic { keywords } => {
                TriggerType::DialogueTopic { topic_keywords: keywords }
            }
            TriggerTypeRequest::ChallengeComplete { challenge_id, requires_success } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                TriggerType::ChallengeComplete { challenge_id: id, requires_success }
            }
            TriggerTypeRequest::TimeBased { turns } => TriggerType::TimeBased { turns },
            TriggerTypeRequest::NpcPresent { keywords } => {
                TriggerType::NpcPresent { npc_keywords: keywords }
            }
            TriggerTypeRequest::Custom { description } => TriggerType::Custom { description },
        }
    }
}

impl From<TriggerType> for TriggerTypeRequest {
    fn from(t: TriggerType) -> Self {
        match t {
            TriggerType::ObjectInteraction { keywords } => {
                TriggerTypeRequest::ObjectInteraction { keywords }
            }
            TriggerType::EnterArea { area_keywords } => {
                TriggerTypeRequest::EnterArea { keywords: area_keywords }
            }
            TriggerType::DialogueTopic { topic_keywords } => {
                TriggerTypeRequest::DialogueTopic { keywords: topic_keywords }
            }
            TriggerType::ChallengeComplete { challenge_id, requires_success } => {
                TriggerTypeRequest::ChallengeComplete {
                    challenge_id: challenge_id.to_string(),
                    requires_success,
                }
            }
            TriggerType::TimeBased { turns } => TriggerTypeRequest::TimeBased { turns },
            TriggerType::NpcPresent { npc_keywords } => {
                TriggerTypeRequest::NpcPresent { keywords: npc_keywords }
            }
            TriggerType::Custom { description } => TriggerTypeRequest::Custom { description },
        }
    }
}

/// Challenge response
#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub id: String,
    pub world_id: String,
    pub scene_id: Option<String>,
    pub name: String,
    pub description: String,
    pub challenge_type: ChallengeType,
    pub skill_id: String,
    pub difficulty: DifficultyRequest,
    pub outcomes: OutcomesRequest,
    pub trigger_conditions: Vec<TriggerConditionRequest>,
    pub prerequisite_challenges: Vec<String>,
    pub active: bool,
    pub order: u32,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

impl From<Challenge> for ChallengeResponse {
    fn from(c: Challenge) -> Self {
        Self {
            id: c.id.to_string(),
            world_id: c.world_id.to_string(),
            scene_id: c.scene_id.map(|s| s.to_string()),
            name: c.name,
            description: c.description,
            challenge_type: c.challenge_type,
            skill_id: c.skill_id.to_string(),
            difficulty: c.difficulty.into(),
            outcomes: c.outcomes.into(),
            trigger_conditions: c.trigger_conditions.into_iter().map(Into::into).collect(),
            prerequisite_challenges: c.prerequisite_challenges.iter().map(|id| id.to_string()).collect(),
            active: c.active,
            order: c.order,
            is_favorite: c.is_favorite,
            tags: c.tags,
        }
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// List all challenges for a world
pub async fn list_challenges(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<ChallengeResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let challenges = state
        .repository
        .challenges()
        .list_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(challenges.into_iter().map(ChallengeResponse::from).collect()))
}

/// List challenges for a specific scene
pub async fn list_scene_challenges(
    State(state): State<Arc<AppState>>,
    Path(scene_id): Path<String>,
) -> Result<Json<Vec<ChallengeResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&scene_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?;
    let scene_id = SceneId::from_uuid(uuid);

    let challenges = state
        .repository
        .challenges()
        .list_by_scene(scene_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(challenges.into_iter().map(ChallengeResponse::from).collect()))
}

/// List active challenges for a world (for LLM context)
pub async fn list_active_challenges(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<ChallengeResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let challenges = state
        .repository
        .challenges()
        .list_active(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(challenges.into_iter().map(ChallengeResponse::from).collect()))
}

/// List favorite challenges
pub async fn list_favorite_challenges(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<ChallengeResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let challenges = state
        .repository
        .challenges()
        .list_favorites(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(challenges.into_iter().map(ChallengeResponse::from).collect()))
}

/// Get a single challenge
pub async fn get_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<String>,
) -> Result<Json<ChallengeResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&challenge_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid challenge ID".to_string()))?;
    let challenge_id = ChallengeId::from_uuid(uuid);

    let challenge = state
        .repository
        .challenges()
        .get(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Challenge not found".to_string()))?;

    Ok(Json(ChallengeResponse::from(challenge)))
}

/// Create a new challenge
pub async fn create_challenge(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateChallengeRequest>,
) -> Result<(StatusCode, Json<ChallengeResponse>), (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);

    // Verify world exists
    let _ = state
        .repository
        .worlds()
        .get(world_id)
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
        .with_challenge_type(req.challenge_type)
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

    // Save to repository
    state
        .repository
        .challenges()
        .create(&challenge)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(ChallengeResponse::from(challenge))))
}

/// Update a challenge
pub async fn update_challenge(
    State(state): State<Arc<AppState>>,
    Path(challenge_id): Path<String>,
    Json(req): Json<UpdateChallengeRequest>,
) -> Result<Json<ChallengeResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&challenge_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid challenge ID".to_string()))?;
    let challenge_id = ChallengeId::from_uuid(uuid);

    // Get existing challenge
    let mut challenge = state
        .repository
        .challenges()
        .get(challenge_id)
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
        challenge.challenge_type = challenge_type;
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
    state
        .repository
        .challenges()
        .update(&challenge)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ChallengeResponse::from(challenge)))
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
        .repository
        .challenges()
        .get(challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Challenge not found".to_string()))?;

    // Delete it
    state
        .repository
        .challenges()
        .delete(challenge_id)
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
        .repository
        .challenges()
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
        .repository
        .challenges()
        .set_active(challenge_id, active)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
