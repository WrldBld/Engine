//! Event Chain API routes
//!
//! Endpoints for managing event chains (story arcs) within a world.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{EventChainService, WorldService};
use crate::application::dto::{
    AddEventRequestDto, ChainStatusResponseDto, CreateEventChainRequestDto, EventChainResponseDto,
    UpdateEventChainRequestDto,
};
use crate::domain::entities::EventChain;
use crate::domain::value_objects::{ActId, EventChainId, NarrativeEventId, WorldId};
use crate::infrastructure::state::AppState;

// NOTE: event chain request/response DTOs + conversions live in `application/dto/event_chain.rs`.

// ============================================================================
// Handlers
// ============================================================================

/// List all event chains for a world
pub async fn list_event_chains(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<EventChainResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let chains = state
                .game.event_chain_service
        .list_event_chains(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        chains.into_iter().map(EventChainResponseDto::from).collect(),
    ))
}

/// List active event chains
pub async fn list_active_chains(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<EventChainResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let chains = state
                .game.event_chain_service
        .list_active(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        chains.into_iter().map(EventChainResponseDto::from).collect(),
    ))
}

/// List favorite event chains
pub async fn list_favorite_chains(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<EventChainResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let chains = state
                .game.event_chain_service
        .list_favorites(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        chains.into_iter().map(EventChainResponseDto::from).collect(),
    ))
}

/// List chain statuses (summary view)
pub async fn list_chain_statuses(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<ChainStatusResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let statuses = state
                .game.event_chain_service
        .list_statuses(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        statuses
            .into_iter()
            .map(ChainStatusResponseDto::from)
            .collect(),
    ))
}

/// Get a single event chain by ID
pub async fn get_event_chain(
    State(state): State<Arc<AppState>>,
    Path(chain_id): Path<String>,
) -> Result<Json<EventChainResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(uuid);

    let chain = state
                .game.event_chain_service
        .get_event_chain(chain_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Event chain not found".to_string()))?;

    Ok(Json(EventChainResponseDto::from(chain)))
}

/// Create a new event chain
pub async fn create_event_chain(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateEventChainRequestDto>,
) -> Result<(StatusCode, Json<EventChainResponseDto>), (StatusCode, String)> {
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

    // Parse event IDs
    let events: Vec<NarrativeEventId> = req
        .events
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok().map(NarrativeEventId::from))
        .collect();

    // Parse optional act ID
    let act_id = if let Some(ref aid) = req.act_id {
        Some(
            Uuid::parse_str(aid)
                .map(ActId::from)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid act ID".to_string()))?,
        )
    } else {
        None
    };

    // Build the event chain
    let mut chain = EventChain::new(world_id, req.name);
    chain.description = req.description;
    chain.events = events;
    chain.act_id = act_id;
    chain.tags = req.tags;
    chain.color = req.color;
    chain.is_active = req.is_active;

    // Save via service
    let created_chain = state
                .game.event_chain_service
        .create_event_chain(chain)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(EventChainResponseDto::from(created_chain)),
    ))
}

/// Update an event chain
pub async fn update_event_chain(
    State(state): State<Arc<AppState>>,
    Path(chain_id): Path<String>,
    Json(req): Json<UpdateEventChainRequestDto>,
) -> Result<Json<EventChainResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(uuid);

    // Get existing chain
    let mut chain = state
                .game.event_chain_service
        .get_event_chain(chain_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Event chain not found".to_string()))?;

    // Apply updates
    if let Some(name) = req.name {
        chain.name = name;
    }
    if let Some(description) = req.description {
        chain.description = description;
    }
    if let Some(events) = req.events {
        chain.events = events
            .iter()
            .filter_map(|s| Uuid::parse_str(s).ok().map(NarrativeEventId::from))
            .collect();
    }
    if let Some(act_id_str) = req.act_id {
        if act_id_str.is_empty() {
            chain.act_id = None;
        } else {
            chain.act_id = Some(
                Uuid::parse_str(&act_id_str)
                    .map(ActId::from)
                    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid act ID".to_string()))?,
            );
        }
    }
    if let Some(tags) = req.tags {
        chain.tags = tags;
    }
    if let Some(color) = req.color {
        chain.color = if color.is_empty() { None } else { Some(color) };
    }
    if let Some(is_active) = req.is_active {
        chain.is_active = is_active;
    }

    // Save updates
    let updated_chain = state
                .game.event_chain_service
        .update_event_chain(chain)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(EventChainResponseDto::from(updated_chain)))
}

/// Delete an event chain
pub async fn delete_event_chain(
    State(state): State<Arc<AppState>>,
    Path(chain_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(uuid);

    // Delete via service (which will verify existence)
    state
                .game.event_chain_service
        .delete_event_chain(chain_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Toggle favorite status
pub async fn toggle_favorite(
    State(state): State<Arc<AppState>>,
    Path(chain_id): Path<String>,
) -> Result<Json<bool>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(uuid);

    let is_favorite = state
                .game.event_chain_service
        .toggle_favorite(chain_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(is_favorite))
}

/// Set active status
pub async fn set_active(
    State(state): State<Arc<AppState>>,
    Path(chain_id): Path<String>,
    Json(is_active): Json<bool>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(uuid);

    state
                .game.event_chain_service
        .set_active(chain_id, is_active)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Reset chain progress
pub async fn reset_chain(
    State(state): State<Arc<AppState>>,
    Path(chain_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(uuid);

    state
                .game.event_chain_service
        .reset_chain(chain_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Add an event to a chain
pub async fn add_event_to_chain(
    State(state): State<Arc<AppState>>,
    Path(chain_id): Path<String>,
    Json(req): Json<AddEventRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let chain_uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(chain_uuid);

    let event_uuid = Uuid::parse_str(&req.event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(event_uuid);

    // If position is specified, we need to get the chain and insert at position
    if let Some(position) = req.position {
        let mut chain = state
                .game.event_chain_service
            .get_event_chain(chain_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Event chain not found".to_string()))?;

        chain.insert_event(position, event_id);

        state
                .game.event_chain_service
            .update_event_chain(chain)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        // Just append to the end
        state
                .game.event_chain_service
            .add_event_to_chain(chain_id, event_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(StatusCode::OK)
}

/// Remove an event from a chain
pub async fn remove_event_from_chain(
    State(state): State<Arc<AppState>>,
    Path((chain_id, event_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let chain_uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(chain_uuid);

    let event_uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(event_uuid);

    state
                .game.event_chain_service
        .remove_event_from_chain(chain_id, event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Mark an event as completed in a chain
pub async fn complete_event_in_chain(
    State(state): State<Arc<AppState>>,
    Path((chain_id, event_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let chain_uuid = Uuid::parse_str(&chain_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid chain ID".to_string()))?;
    let chain_id = EventChainId::from_uuid(chain_uuid);

    let event_uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = NarrativeEventId::from_uuid(event_uuid);

    state
                .game.event_chain_service
        .complete_event(chain_id, event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
