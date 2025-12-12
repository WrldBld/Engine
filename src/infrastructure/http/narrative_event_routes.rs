//! Narrative Event API routes
//!
//! Endpoints for managing DM-designed narrative events within a world.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::NarrativeEvent;
use crate::domain::value_objects::{NarrativeEventId, WorldId};
use crate::infrastructure::state::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Query parameters for listing narrative events
#[derive(Debug, Deserialize)]
pub struct ListNarrativeEventsQuery {
    #[serde(default)]
    pub act_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
}

/// Request to create a narrative event
#[derive(Debug, Deserialize)]
pub struct CreateNarrativeEventRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scene_direction: String,
    #[serde(default)]
    pub suggested_opening: Option<String>,
    #[serde(default)]
    pub is_repeatable: bool,
    #[serde(default)]
    pub delay_turns: u32,
    #[serde(default)]
    pub expires_after_turns: Option<u32>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Request to update a narrative event
#[derive(Debug, Deserialize)]
pub struct UpdateNarrativeEventRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scene_direction: Option<String>,
    #[serde(default)]
    pub suggested_opening: Option<String>,
    #[serde(default)]
    pub is_repeatable: Option<bool>,
    #[serde(default)]
    pub delay_turns: Option<u32>,
    #[serde(default)]
    pub expires_after_turns: Option<u32>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Narrative event response - simplified view for API
#[derive(Debug, Serialize)]
pub struct NarrativeEventResponse {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub scene_direction: String,
    pub suggested_opening: Option<String>,
    pub trigger_count: u32,
    pub is_active: bool,
    pub is_triggered: bool,
    pub triggered_at: Option<String>,
    pub selected_outcome: Option<String>,
    pub is_repeatable: bool,
    pub delay_turns: u32,
    pub expires_after_turns: Option<u32>,
    pub priority: i32,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub scene_id: Option<String>,
    pub location_id: Option<String>,
    pub act_id: Option<String>,
    pub chain_id: Option<String>,
    pub chain_position: Option<u32>,
    pub outcome_count: usize,
    pub trigger_condition_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl From<NarrativeEvent> for NarrativeEventResponse {
    fn from(e: NarrativeEvent) -> Self {
        Self {
            id: e.id.to_string(),
            world_id: e.world_id.to_string(),
            name: e.name,
            description: e.description,
            scene_direction: e.scene_direction,
            suggested_opening: e.suggested_opening,
            trigger_count: e.trigger_count,
            is_active: e.is_active,
            is_triggered: e.is_triggered,
            triggered_at: e.triggered_at.map(|t| t.to_rfc3339()),
            selected_outcome: e.selected_outcome,
            is_repeatable: e.is_repeatable,
            delay_turns: e.delay_turns,
            expires_after_turns: e.expires_after_turns,
            priority: e.priority,
            is_favorite: e.is_favorite,
            tags: e.tags,
            scene_id: e.scene_id.map(|s| s.to_string()),
            location_id: e.location_id.map(|l| l.to_string()),
            act_id: e.act_id.map(|a| a.to_string()),
            chain_id: e.chain_id.map(|c| c.to_string()),
            chain_position: e.chain_position,
            outcome_count: e.outcomes.len(),
            trigger_condition_count: e.trigger_conditions.len(),
            created_at: e.created_at.to_rfc3339(),
            updated_at: e.updated_at.to_rfc3339(),
        }
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// List narrative events for a world
pub async fn list_narrative_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Query(_query): Query<ListNarrativeEventsQuery>,
) -> Result<Json<Vec<NarrativeEventResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let events = state
        .repository
        .narrative_events()
        .list_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        events.into_iter().map(NarrativeEventResponse::from).collect(),
    ))
}

/// List active narrative events
pub async fn list_active_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<NarrativeEventResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let events = state
        .repository
        .narrative_events()
        .list_active(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        events.into_iter().map(NarrativeEventResponse::from).collect(),
    ))
}

/// List favorite narrative events
pub async fn list_favorite_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<NarrativeEventResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let events = state
        .repository
        .narrative_events()
        .list_favorites(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        events.into_iter().map(NarrativeEventResponse::from).collect(),
    ))
}

/// List pending (not yet triggered) narrative events
pub async fn list_pending_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<NarrativeEventResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let events = state
        .repository
        .narrative_events()
        .list_pending(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        events.into_iter().map(NarrativeEventResponse::from).collect(),
    ))
}

/// Get a single narrative event by ID
pub async fn get_narrative_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<NarrativeEventResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    let event = state
        .repository
        .narrative_events()
        .get(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Narrative event not found".to_string()))?;

    Ok(Json(NarrativeEventResponse::from(event)))
}

/// Create a new narrative event
pub async fn create_narrative_event(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateNarrativeEventRequest>,
) -> Result<(StatusCode, Json<NarrativeEventResponse>), (StatusCode, String)> {
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

    // Build the narrative event
    let mut event = NarrativeEvent::new(world_id, req.name);
    event.description = req.description;
    event.scene_direction = req.scene_direction;
    event.suggested_opening = req.suggested_opening;
    event.is_repeatable = req.is_repeatable;
    event.delay_turns = req.delay_turns;
    event.expires_after_turns = req.expires_after_turns;
    event.priority = req.priority;
    event.is_active = req.is_active;
    event.tags = req.tags;

    // Save to repository
    state
        .repository
        .narrative_events()
        .create(&event)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(NarrativeEventResponse::from(event)),
    ))
}

/// Update a narrative event
pub async fn update_narrative_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    Json(req): Json<UpdateNarrativeEventRequest>,
) -> Result<Json<NarrativeEventResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    // Get existing event
    let mut event = state
        .repository
        .narrative_events()
        .get(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Narrative event not found".to_string()))?;

    // Apply updates
    if let Some(name) = req.name {
        event.name = name;
    }
    if let Some(description) = req.description {
        event.description = description;
    }
    if let Some(scene_direction) = req.scene_direction {
        event.scene_direction = scene_direction;
    }
    if let Some(suggested_opening) = req.suggested_opening {
        event.suggested_opening = Some(suggested_opening);
    }
    if let Some(is_repeatable) = req.is_repeatable {
        event.is_repeatable = is_repeatable;
    }
    if let Some(delay_turns) = req.delay_turns {
        event.delay_turns = delay_turns;
    }
    if req.expires_after_turns.is_some() {
        event.expires_after_turns = req.expires_after_turns;
    }
    if let Some(priority) = req.priority {
        event.priority = priority;
    }
    if let Some(is_active) = req.is_active {
        event.is_active = is_active;
    }
    if let Some(tags) = req.tags {
        event.tags = tags;
    }

    // Save updates
    state
        .repository
        .narrative_events()
        .update(&event)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(NarrativeEventResponse::from(event)))
}

/// Delete a narrative event
pub async fn delete_narrative_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    // Verify event exists
    let _ = state
        .repository
        .narrative_events()
        .get(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Narrative event not found".to_string()))?;

    // Delete it
    state
        .repository
        .narrative_events()
        .delete(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Toggle favorite status
pub async fn toggle_favorite(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<bool>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    let is_favorite = state
        .repository
        .narrative_events()
        .toggle_favorite(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(is_favorite))
}

/// Set active status
pub async fn set_active(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    Json(is_active): Json<bool>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    state
        .repository
        .narrative_events()
        .set_active(event_id, is_active)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Mark event as triggered
pub async fn mark_triggered(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    state
        .repository
        .narrative_events()
        .mark_triggered(event_id, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Reset triggered status
pub async fn reset_triggered(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    state
        .repository
        .narrative_events()
        .reset_triggered(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
