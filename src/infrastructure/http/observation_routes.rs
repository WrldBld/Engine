//! Observation API routes (Phase 23D)
//!
//! Endpoints for managing PC observations of NPCs.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::{NpcObservation, ObservationSummary, ObservationType};
use crate::domain::value_objects::{CharacterId, LocationId, PlayerCharacterId, RegionId};
use crate::infrastructure::state::AppState;

// =============================================================================
// DTOs
// =============================================================================

/// Request to create an observation (typically by DM or system)
#[derive(Debug, Deserialize)]
pub struct CreateObservationRequest {
    pub npc_id: String,
    pub location_id: String,
    pub region_id: String,
    /// Observation type: "direct", "heard_about", "deduced"
    #[serde(default = "default_observation_type")]
    pub observation_type: String,
    pub notes: Option<String>,
}

fn default_observation_type() -> String {
    "direct".to_string()
}

/// Response for an observation
#[derive(Debug, Serialize)]
pub struct ObservationResponse {
    pub npc_id: String,
    pub location_id: String,
    pub region_id: String,
    pub game_time: String,
    pub observation_type: String,
    pub notes: Option<String>,
    pub created_at: String,
}

impl From<NpcObservation> for ObservationResponse {
    fn from(obs: NpcObservation) -> Self {
        Self {
            npc_id: obs.npc_id.to_string(),
            location_id: obs.location_id.to_string(),
            region_id: obs.region_id.to_string(),
            game_time: obs.game_time.to_rfc3339(),
            observation_type: obs.observation_type.to_string(),
            notes: obs.notes,
            created_at: obs.created_at.to_rfc3339(),
        }
    }
}

/// Response for observation summary (with NPC details)
#[derive(Debug, Serialize)]
pub struct ObservationSummaryResponse {
    pub npc_id: String,
    pub npc_name: String,
    pub npc_portrait: Option<String>,
    pub location_name: String,
    pub region_name: String,
    pub game_time: String,
    pub observation_type: String,
    pub observation_type_icon: String,
    pub notes: Option<String>,
}

impl From<ObservationSummary> for ObservationSummaryResponse {
    fn from(summary: ObservationSummary) -> Self {
        Self {
            npc_id: summary.npc_id,
            npc_name: summary.npc_name,
            npc_portrait: summary.npc_portrait,
            location_name: summary.location_name,
            region_name: summary.region_name,
            game_time: summary.game_time.to_rfc3339(),
            observation_type: summary.observation_type.to_string(),
            observation_type_icon: summary.observation_type.icon().to_string(),
            notes: summary.notes,
        }
    }
}

// =============================================================================
// Routes
// =============================================================================

/// List all observations for a PC (with NPC details)
///
/// GET /api/player-characters/{pc_id}/observations
pub async fn list_observations(
    State(state): State<Arc<AppState>>,
    Path(pc_id): Path<String>,
) -> Result<Json<Vec<ObservationSummaryResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid PC ID".to_string()))?;
    let pc_id = PlayerCharacterId::from_uuid(uuid);

    let summaries = state
        .repository
        .observations()
        .get_summaries_for_pc(pc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        summaries
            .into_iter()
            .map(ObservationSummaryResponse::from)
            .collect(),
    ))
}

/// Get a specific observation for a PC/NPC pair
///
/// GET /api/player-characters/{pc_id}/observations/{npc_id}
pub async fn get_observation(
    State(state): State<Arc<AppState>>,
    Path((pc_id, npc_id)): Path<(String, String)>,
) -> Result<Json<ObservationResponse>, (StatusCode, String)> {
    let pc_uuid = Uuid::parse_str(&pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid PC ID".to_string()))?;
    let npc_uuid = Uuid::parse_str(&npc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid NPC ID".to_string()))?;

    let pc_id = PlayerCharacterId::from_uuid(pc_uuid);
    let npc_id = CharacterId::from_uuid(npc_uuid);

    let observation = state
        .repository
        .observations()
        .get_latest(pc_id, npc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Observation not found".to_string()))?;

    Ok(Json(ObservationResponse::from(observation)))
}

/// Create an observation (typically called by DM or system)
///
/// POST /api/player-characters/{pc_id}/observations
pub async fn create_observation(
    State(state): State<Arc<AppState>>,
    Path(pc_id): Path<String>,
    Json(req): Json<CreateObservationRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pc_uuid = Uuid::parse_str(&pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid PC ID".to_string()))?;
    let npc_uuid = Uuid::parse_str(&req.npc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid NPC ID".to_string()))?;
    let location_uuid = Uuid::parse_str(&req.location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
    let region_uuid = Uuid::parse_str(&req.region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;

    let pc_id = PlayerCharacterId::from_uuid(pc_uuid);
    let npc_id = CharacterId::from_uuid(npc_uuid);
    let location_id = LocationId::from_uuid(location_uuid);
    let region_id = RegionId::from_uuid(region_uuid);

    let observation_type: ObservationType = req
        .observation_type
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid observation type".to_string()))?;

    // Get current game time from the PC's session (if available)
    // For now, use current UTC time as a fallback
    let game_time = chrono::Utc::now();

    let observation = match observation_type {
        ObservationType::Direct => {
            NpcObservation::direct(pc_id, npc_id, location_id, region_id, game_time)
        }
        ObservationType::HeardAbout => {
            NpcObservation::heard_about(pc_id, npc_id, location_id, region_id, game_time, req.notes)
        }
        ObservationType::Deduced => {
            NpcObservation::deduced(pc_id, npc_id, location_id, region_id, game_time, req.notes)
        }
    };

    state
        .repository
        .observations()
        .upsert(&observation)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Delete an observation
///
/// DELETE /api/player-characters/{pc_id}/observations/{npc_id}
pub async fn delete_observation(
    State(state): State<Arc<AppState>>,
    Path((pc_id, npc_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pc_uuid = Uuid::parse_str(&pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid PC ID".to_string()))?;
    let npc_uuid = Uuid::parse_str(&npc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid NPC ID".to_string()))?;

    let pc_id = PlayerCharacterId::from_uuid(pc_uuid);
    let npc_id = CharacterId::from_uuid(npc_uuid);

    state
        .repository
        .observations()
        .delete(pc_id, npc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
