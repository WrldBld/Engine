//! Story Event API routes
//!
//! Endpoints for managing story events (gameplay timeline) within a world.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::WorldService;
use crate::application::dto::{
    CreateDmMarkerRequestDto, ListStoryEventsQueryDto, PaginatedStoryEventsResponseDto,
    StoryEventResponseDto, UpdateStoryEventRequestDto,
};
use crate::domain::value_objects::{
    CharacterId, LocationId, SceneId, SessionId, StoryEventId, WorldId,
};
use crate::infrastructure::state::AppState;
// NOTE: story event request/response DTOs live in `application/dto/story_event.rs`.

// ============================================================================
// Handlers
// ============================================================================

/// List story events for a world with optional filters
pub async fn list_story_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Query(query): Query<ListStoryEventsQueryDto>,
) -> Result<Json<PaginatedStoryEventsResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);

    // Handle different query types
    let events = if let Some(session_id_str) = query.session_id {
        let session_uuid = Uuid::parse_str(&session_id_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
        let session_id = SessionId::from_uuid(session_uuid);
        state
                .game.story_event_service
            .list_by_session(session_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if let Some(character_id_str) = query.character_id {
        let char_uuid = Uuid::parse_str(&character_id_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
        let character_id = CharacterId::from_uuid(char_uuid);
        state
                .game.story_event_service
            .list_by_character(character_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if let Some(location_id_str) = query.location_id {
        let loc_uuid = Uuid::parse_str(&location_id_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
        let location_id = LocationId::from_uuid(loc_uuid);
        state
                .game.story_event_service
            .list_by_location(location_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if let Some(tags_str) = query.tags {
        let tags: Vec<String> = tags_str.split(',').map(|s| s.trim().to_string()).collect();
        state
                .game.story_event_service
            .search_by_tags(world_id, tags)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if let Some(search_text) = query.search {
        state
                .game.story_event_service
            .search_by_text(world_id, &search_text)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if query.visible_only.unwrap_or(false) {
        state
                .game.story_event_service
            .list_visible(world_id, limit)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        state
                .game.story_event_service
            .list_by_world_paginated(world_id, limit, offset)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    // Get total count
    let total = state
                .game.story_event_service
        .count_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PaginatedStoryEventsResponseDto {
        events: events.into_iter().map(StoryEventResponseDto::from).collect(),
        total,
        limit,
        offset,
    }))
}

/// Get a single story event by ID
pub async fn get_story_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<StoryEventResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = StoryEventId::from_uuid(uuid);

    let event = state
                .game.story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    Ok(Json(StoryEventResponseDto::from(event)))
}

/// Create a DM marker story event
pub async fn create_dm_marker(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateDmMarkerRequestDto>,
) -> Result<(StatusCode, Json<StoryEventResponseDto>), (StatusCode, String)> {
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

    // Parse session ID
    let session_uuid = Uuid::parse_str(&req.session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    // Parse optional scene ID
    let scene_id = if let Some(ref sid) = req.scene_id {
        Some(
            Uuid::parse_str(sid)
                .map(SceneId::from_uuid)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid scene ID".to_string()))?,
        )
    } else {
        None
    };

    // Parse optional location ID
    let location_id = if let Some(ref lid) = req.location_id {
        Some(
            Uuid::parse_str(lid)
                .map(LocationId::from_uuid)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?,
        )
    } else {
        None
    };

    // Create via service
    let event_id = state
                .game.story_event_service
        .record_dm_marker(
            world_id,
            session_id,
            scene_id,
            location_id,
            req.title,
            req.note,
            req.importance.into(),
            req.marker_type.into(),
            req.is_hidden,
            req.tags,
            req.game_time,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch the created event to return
    let event = state
                .game.story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found after creation".to_string()))?;

    Ok((StatusCode::CREATED, Json(StoryEventResponseDto::from(event))))
}

/// Update a story event (summary, visibility, tags)
pub async fn update_story_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    Json(req): Json<UpdateStoryEventRequestDto>,
) -> Result<Json<StoryEventResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = StoryEventId::from_uuid(uuid);

    // Get existing event (verify it exists before updating)
    let _event = state
                .game.story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    // Apply updates
    if let Some(summary) = req.summary {
        state
                .game.story_event_service
            .update_summary(event_id, &summary)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(is_hidden) = req.is_hidden {
        state
                .game.story_event_service
            .set_hidden(event_id, is_hidden)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(tags) = req.tags {
        state
                .game.story_event_service
            .update_tags(event_id, tags)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Fetch updated event
    let updated_event = state
                .game.story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    Ok(Json(StoryEventResponseDto::from(updated_event)))
}

/// Toggle visibility of a story event
pub async fn toggle_visibility(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<bool>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = StoryEventId::from_uuid(uuid);

    // Get current visibility
    let event = state
                .game.story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    let new_hidden = !event.is_hidden;
    state
                .game.story_event_service
        .set_hidden(event_id, new_hidden)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(new_hidden))
}

/// Delete a story event (rarely used - events are usually immutable)
pub async fn delete_story_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = StoryEventId::from_uuid(uuid);

    // Verify event exists
    let _ = state
                .game.story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    // Delete it
    state
                .game.story_event_service
        .delete(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get story events count for a world
pub async fn count_story_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<u64>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let count = state
                .game.story_event_service
        .count_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(count))
}
