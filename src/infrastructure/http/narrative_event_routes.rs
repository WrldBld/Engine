//! Narrative Event API routes
//!
//! Endpoints for managing DM-designed narrative events within a world.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{NarrativeEventService, WorldService};
use crate::application::dto::{
    CreateNarrativeEventRequestDto, ListNarrativeEventsQueryDto, NarrativeEventResponseDto,
    UpdateNarrativeEventRequestDto,
};
use crate::domain::entities::NarrativeEvent;
use crate::domain::value_objects::{NarrativeEventId, WorldId};
use crate::infrastructure::state::AppState;
// NOTE: narrative event request/response DTOs + conversions live in `application/dto/narrative_event.rs`.

// ============================================================================
// Handlers
// ============================================================================

/// List narrative events for a world
pub async fn list_narrative_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Query(_query): Query<ListNarrativeEventsQueryDto>,
) -> Result<Json<Vec<NarrativeEventResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let events = state
                .game.narrative_event_service
        .list_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        events
            .into_iter()
            .map(NarrativeEventResponseDto::from)
            .collect(),
    ))
}

/// List active narrative events
pub async fn list_active_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<NarrativeEventResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let events = state
                .game.narrative_event_service
        .list_active(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        events
            .into_iter()
            .map(NarrativeEventResponseDto::from)
            .collect(),
    ))
}

/// List favorite narrative events
pub async fn list_favorite_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<NarrativeEventResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let events = state
                .game.narrative_event_service
        .list_favorites(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        events
            .into_iter()
            .map(NarrativeEventResponseDto::from)
            .collect(),
    ))
}

/// List pending (not yet triggered) narrative events
pub async fn list_pending_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<NarrativeEventResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let events = state
                .game.narrative_event_service
        .list_pending(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        events
            .into_iter()
            .map(NarrativeEventResponseDto::from)
            .collect(),
    ))
}

/// Get a single narrative event by ID
pub async fn get_narrative_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<NarrativeEventResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    let event = state
                .game.narrative_event_service
        .get(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Narrative event not found".to_string()))?;

    Ok(Json(NarrativeEventResponseDto::from(event)))
}

/// Create a new narrative event
pub async fn create_narrative_event(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateNarrativeEventRequestDto>,
) -> Result<(StatusCode, Json<NarrativeEventResponseDto>), (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);

    // Verify world exists
    let _ = state
        .core.world_service
        .get_world(world_id)
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

    // Save via service
    let event = state
                .game.narrative_event_service
        .create(event)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(NarrativeEventResponseDto::from(event)),
    ))
}

/// Update a narrative event
pub async fn update_narrative_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    Json(req): Json<UpdateNarrativeEventRequestDto>,
) -> Result<Json<NarrativeEventResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(uuid);

    // Get existing event
    let mut event = state
                .game.narrative_event_service
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
    let event = state
                .game.narrative_event_service
        .update(event)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(NarrativeEventResponseDto::from(event)))
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
                .game.narrative_event_service
        .get(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Narrative event not found".to_string()))?;

    // Delete it
    state
                .game.narrative_event_service
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
                .game.narrative_event_service
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
                .game.narrative_event_service
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
                .game.narrative_event_service
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
                .game.narrative_event_service
        .reset_triggered(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
