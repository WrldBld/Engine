//! Story Event API routes
//!
//! Endpoints for managing story events (gameplay timeline) within a world.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::WorldService;
use crate::domain::entities::{
    DmMarkerType, ItemSource, MarkerImportance, StoryEvent, StoryEventType,
};
use crate::domain::value_objects::{
    CharacterId, LocationId, SceneId, SessionId, StoryEventId, WorldId,
};
use crate::infrastructure::state::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Query parameters for listing story events
#[derive(Debug, Deserialize)]
pub struct ListStoryEventsQuery {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub character_id: Option<String>,
    #[serde(default)]
    pub location_id: Option<String>,
    #[serde(default)]
    pub visible_only: Option<bool>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
}

/// Request to create a DM marker story event
#[derive(Debug, Deserialize)]
pub struct CreateDmMarkerRequest {
    pub session_id: String,
    pub title: String,
    pub note: String,
    #[serde(default)]
    pub importance: MarkerImportanceRequest,
    #[serde(default)]
    pub marker_type: DmMarkerTypeRequest,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub location_id: Option<String>,
    #[serde(default)]
    pub game_time: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_hidden: bool,
}

/// Request to update a story event
#[derive(Debug, Deserialize)]
pub struct UpdateStoryEventRequest {
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub is_hidden: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Marker importance for request
#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarkerImportanceRequest {
    #[default]
    Minor,
    Notable,
    Major,
    Critical,
}

impl From<MarkerImportanceRequest> for MarkerImportance {
    fn from(req: MarkerImportanceRequest) -> Self {
        match req {
            MarkerImportanceRequest::Minor => MarkerImportance::Minor,
            MarkerImportanceRequest::Notable => MarkerImportance::Notable,
            MarkerImportanceRequest::Major => MarkerImportance::Major,
            MarkerImportanceRequest::Critical => MarkerImportance::Critical,
        }
    }
}

impl From<MarkerImportance> for MarkerImportanceRequest {
    fn from(m: MarkerImportance) -> Self {
        match m {
            MarkerImportance::Minor => MarkerImportanceRequest::Minor,
            MarkerImportance::Notable => MarkerImportanceRequest::Notable,
            MarkerImportance::Major => MarkerImportanceRequest::Major,
            MarkerImportance::Critical => MarkerImportanceRequest::Critical,
        }
    }
}

/// DM marker type for request
#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DmMarkerTypeRequest {
    #[default]
    Note,
    PlotPoint,
    CharacterMoment,
    WorldEvent,
    PlayerDecision,
    Foreshadowing,
    Callback,
    Custom,
}

impl From<DmMarkerTypeRequest> for DmMarkerType {
    fn from(req: DmMarkerTypeRequest) -> Self {
        match req {
            DmMarkerTypeRequest::Note => DmMarkerType::Note,
            DmMarkerTypeRequest::PlotPoint => DmMarkerType::PlotPoint,
            DmMarkerTypeRequest::CharacterMoment => DmMarkerType::CharacterMoment,
            DmMarkerTypeRequest::WorldEvent => DmMarkerType::WorldEvent,
            DmMarkerTypeRequest::PlayerDecision => DmMarkerType::PlayerDecision,
            DmMarkerTypeRequest::Foreshadowing => DmMarkerType::Foreshadowing,
            DmMarkerTypeRequest::Callback => DmMarkerType::Callback,
            DmMarkerTypeRequest::Custom => DmMarkerType::Custom,
        }
    }
}

impl From<DmMarkerType> for DmMarkerTypeRequest {
    fn from(m: DmMarkerType) -> Self {
        match m {
            DmMarkerType::Note => DmMarkerTypeRequest::Note,
            DmMarkerType::PlotPoint => DmMarkerTypeRequest::PlotPoint,
            DmMarkerType::CharacterMoment => DmMarkerTypeRequest::CharacterMoment,
            DmMarkerType::WorldEvent => DmMarkerTypeRequest::WorldEvent,
            DmMarkerType::PlayerDecision => DmMarkerTypeRequest::PlayerDecision,
            DmMarkerType::Foreshadowing => DmMarkerTypeRequest::Foreshadowing,
            DmMarkerType::Callback => DmMarkerTypeRequest::Callback,
            DmMarkerType::Custom => DmMarkerTypeRequest::Custom,
        }
    }
}

/// Story event response
#[derive(Debug, Serialize)]
pub struct StoryEventResponse {
    pub id: String,
    pub world_id: String,
    pub session_id: String,
    pub scene_id: Option<String>,
    pub location_id: Option<String>,
    pub event_type: StoryEventTypeResponse,
    pub timestamp: String,
    pub game_time: Option<String>,
    pub summary: String,
    pub involved_characters: Vec<String>,
    pub is_hidden: bool,
    pub tags: Vec<String>,
    pub triggered_by: Option<String>,
    pub type_name: String,
}

impl From<StoryEvent> for StoryEventResponse {
    fn from(e: StoryEvent) -> Self {
        let type_name = e.type_name().to_string();
        Self {
            id: e.id.to_string(),
            world_id: e.world_id.to_string(),
            session_id: e.session_id.to_string(),
            scene_id: e.scene_id.map(|s| s.to_string()),
            location_id: e.location_id.map(|l| l.to_string()),
            event_type: StoryEventTypeResponse::from(e.event_type),
            timestamp: e.timestamp.to_rfc3339(),
            game_time: e.game_time,
            summary: e.summary,
            involved_characters: e.involved_characters.iter().map(|c| c.to_string()).collect(),
            is_hidden: e.is_hidden,
            tags: e.tags,
            triggered_by: e.triggered_by.map(|t| t.to_string()),
            type_name,
        }
    }
}

/// Story event type response (simplified for API)
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoryEventTypeResponse {
    LocationChange {
        from_location: Option<String>,
        to_location: String,
        character_id: String,
        travel_method: Option<String>,
    },
    DialogueExchange {
        npc_id: String,
        npc_name: String,
        player_dialogue: String,
        npc_response: String,
        topics_discussed: Vec<String>,
        tone: Option<String>,
    },
    CombatEvent {
        combat_type: String,
        participants: Vec<String>,
        enemies: Vec<String>,
        outcome: Option<String>,
        location_id: String,
        rounds: Option<u32>,
    },
    ChallengeAttempted {
        challenge_id: Option<String>,
        challenge_name: String,
        character_id: String,
        skill_used: Option<String>,
        difficulty: Option<String>,
        roll_result: Option<i32>,
        modifier: Option<i32>,
        outcome: String,
    },
    ItemAcquired {
        item_name: String,
        item_description: Option<String>,
        character_id: String,
        source: ItemSourceResponse,
        quantity: u32,
    },
    ItemTransferred {
        item_name: String,
        from_character: Option<String>,
        to_character: String,
        quantity: u32,
        reason: Option<String>,
    },
    ItemUsed {
        item_name: String,
        character_id: String,
        target: Option<String>,
        effect: String,
        consumed: bool,
    },
    RelationshipChanged {
        from_character: String,
        to_character: String,
        previous_sentiment: Option<f32>,
        new_sentiment: f32,
        sentiment_change: f32,
        reason: String,
    },
    SceneTransition {
        from_scene: Option<String>,
        to_scene: String,
        from_scene_name: Option<String>,
        to_scene_name: String,
        trigger_reason: String,
    },
    InformationRevealed {
        info_type: String,
        title: String,
        content: String,
        source: Option<String>,
        importance: String,
        persist_to_journal: bool,
    },
    NpcAction {
        npc_id: String,
        npc_name: String,
        action_type: String,
        description: String,
        dm_approved: bool,
        dm_modified: bool,
    },
    DmMarker {
        title: String,
        note: String,
        importance: String,
        marker_type: String,
    },
    NarrativeEventTriggered {
        narrative_event_id: String,
        narrative_event_name: String,
        outcome_branch: Option<String>,
        effects_applied: Vec<String>,
    },
    StatModified {
        character_id: String,
        stat_name: String,
        previous_value: i32,
        new_value: i32,
        reason: String,
    },
    FlagChanged {
        flag_name: String,
        new_value: bool,
        reason: String,
    },
    SessionStarted {
        session_number: u32,
        session_name: Option<String>,
        players_present: Vec<String>,
    },
    SessionEnded {
        duration_minutes: u32,
        summary: String,
    },
    Custom {
        event_subtype: String,
        title: String,
        description: String,
        data: serde_json::Value,
    },
}

impl From<StoryEventType> for StoryEventTypeResponse {
    fn from(e: StoryEventType) -> Self {
        match e {
            StoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => StoryEventTypeResponse::LocationChange {
                from_location: from_location.map(|l| l.to_string()),
                to_location: to_location.to_string(),
                character_id: character_id.to_string(),
                travel_method,
            },
            StoryEventType::DialogueExchange {
                npc_id,
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            } => StoryEventTypeResponse::DialogueExchange {
                npc_id: npc_id.to_string(),
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            },
            StoryEventType::CombatEvent {
                combat_type,
                participants,
                enemies,
                outcome,
                location_id,
                rounds,
            } => StoryEventTypeResponse::CombatEvent {
                combat_type: format!("{:?}", combat_type),
                participants: participants.iter().map(|p| p.to_string()).collect(),
                enemies,
                outcome: outcome.map(|o| format!("{:?}", o)),
                location_id: location_id.to_string(),
                rounds,
            },
            StoryEventType::ChallengeAttempted {
                challenge_id,
                challenge_name,
                character_id,
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome,
            } => StoryEventTypeResponse::ChallengeAttempted {
                challenge_id: challenge_id.map(|c| c.to_string()),
                challenge_name,
                character_id: character_id.to_string(),
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome: format!("{:?}", outcome),
            },
            StoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id,
                source,
                quantity,
            } => StoryEventTypeResponse::ItemAcquired {
                item_name,
                item_description,
                character_id: character_id.to_string(),
                source: ItemSourceResponse::from(source),
                quantity,
            },
            StoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => StoryEventTypeResponse::ItemTransferred {
                item_name,
                from_character: from_character.map(|c| c.to_string()),
                to_character: to_character.to_string(),
                quantity,
                reason,
            },
            StoryEventType::ItemUsed {
                item_name,
                character_id,
                target,
                effect,
                consumed,
            } => StoryEventTypeResponse::ItemUsed {
                item_name,
                character_id: character_id.to_string(),
                target,
                effect,
                consumed,
            },
            StoryEventType::RelationshipChanged {
                from_character,
                to_character,
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            } => StoryEventTypeResponse::RelationshipChanged {
                from_character: from_character.to_string(),
                to_character: to_character.to_string(),
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            },
            StoryEventType::SceneTransition {
                from_scene,
                to_scene,
                from_scene_name,
                to_scene_name,
                trigger_reason,
            } => StoryEventTypeResponse::SceneTransition {
                from_scene: from_scene.map(|s| s.to_string()),
                to_scene: to_scene.to_string(),
                from_scene_name,
                to_scene_name,
                trigger_reason,
            },
            StoryEventType::InformationRevealed {
                info_type,
                title,
                content,
                source,
                importance,
                persist_to_journal,
            } => StoryEventTypeResponse::InformationRevealed {
                info_type: format!("{:?}", info_type),
                title,
                content,
                source: source.map(|s| s.to_string()),
                importance: format!("{:?}", importance),
                persist_to_journal,
            },
            StoryEventType::NpcAction {
                npc_id,
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            } => StoryEventTypeResponse::NpcAction {
                npc_id: npc_id.to_string(),
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            },
            StoryEventType::DmMarker {
                title,
                note,
                importance,
                marker_type,
            } => StoryEventTypeResponse::DmMarker {
                title,
                note,
                importance: format!("{:?}", importance),
                marker_type: format!("{:?}", marker_type),
            },
            StoryEventType::NarrativeEventTriggered {
                narrative_event_id,
                narrative_event_name,
                outcome_branch,
                effects_applied,
            } => StoryEventTypeResponse::NarrativeEventTriggered {
                narrative_event_id: narrative_event_id.to_string(),
                narrative_event_name,
                outcome_branch,
                effects_applied,
            },
            StoryEventType::StatModified {
                character_id,
                stat_name,
                previous_value,
                new_value,
                reason,
            } => StoryEventTypeResponse::StatModified {
                character_id: character_id.to_string(),
                stat_name,
                previous_value,
                new_value,
                reason,
            },
            StoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            } => StoryEventTypeResponse::FlagChanged {
                flag_name,
                new_value,
                reason,
            },
            StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => StoryEventTypeResponse::SessionStarted {
                session_number,
                session_name,
                players_present,
            },
            StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => StoryEventTypeResponse::SessionEnded {
                duration_minutes,
                summary,
            },
            StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            } => StoryEventTypeResponse::Custom {
                event_subtype,
                title,
                description,
                data,
            },
        }
    }
}

/// Item source response
#[derive(Debug, Serialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum ItemSourceResponse {
    Found { location: String },
    Purchased { from: String, cost: Option<String> },
    Gifted { from: String },
    Looted { from: String },
    Crafted,
    Reward { for_what: String },
    Stolen { from: String },
    Custom { description: String },
}

impl From<ItemSource> for ItemSourceResponse {
    fn from(s: ItemSource) -> Self {
        match s {
            ItemSource::Found { location } => ItemSourceResponse::Found { location },
            ItemSource::Purchased { from, cost } => ItemSourceResponse::Purchased { from, cost },
            ItemSource::Gifted { from } => ItemSourceResponse::Gifted {
                from: from.to_string(),
            },
            ItemSource::Looted { from } => ItemSourceResponse::Looted { from },
            ItemSource::Crafted => ItemSourceResponse::Crafted,
            ItemSource::Reward { for_what } => ItemSourceResponse::Reward { for_what },
            ItemSource::Stolen { from } => ItemSourceResponse::Stolen { from },
            ItemSource::Custom { description } => ItemSourceResponse::Custom { description },
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Serialize)]
pub struct PaginatedStoryEventsResponse {
    pub events: Vec<StoryEventResponse>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

// ============================================================================
// Handlers
// ============================================================================

/// List story events for a world with optional filters
pub async fn list_story_events(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Query(query): Query<ListStoryEventsQuery>,
) -> Result<Json<PaginatedStoryEventsResponse>, (StatusCode, String)> {
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
            .story_event_service
            .list_by_session(session_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if let Some(character_id_str) = query.character_id {
        let char_uuid = Uuid::parse_str(&character_id_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
        let character_id = CharacterId::from_uuid(char_uuid);
        state
            .story_event_service
            .list_by_character(character_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if let Some(location_id_str) = query.location_id {
        let loc_uuid = Uuid::parse_str(&location_id_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
        let location_id = LocationId::from_uuid(loc_uuid);
        state
            .story_event_service
            .list_by_location(location_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if let Some(tags_str) = query.tags {
        let tags: Vec<String> = tags_str.split(',').map(|s| s.trim().to_string()).collect();
        state
            .story_event_service
            .search_by_tags(world_id, tags)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if let Some(search_text) = query.search {
        state
            .story_event_service
            .search_by_text(world_id, &search_text)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else if query.visible_only.unwrap_or(false) {
        state
            .story_event_service
            .list_visible(world_id, limit)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        state
            .story_event_service
            .list_by_world_paginated(world_id, limit, offset)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    // Get total count
    let total = state
        .story_event_service
        .count_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PaginatedStoryEventsResponse {
        events: events.into_iter().map(StoryEventResponse::from).collect(),
        total,
        limit,
        offset,
    }))
}

/// Get a single story event by ID
pub async fn get_story_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<StoryEventResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = StoryEventId::from_uuid(uuid);

    let event = state
        .story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    Ok(Json(StoryEventResponse::from(event)))
}

/// Create a DM marker story event
pub async fn create_dm_marker(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateDmMarkerRequest>,
) -> Result<(StatusCode, Json<StoryEventResponse>), (StatusCode, String)> {
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
        .story_event_service
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
        .story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found after creation".to_string()))?;

    Ok((StatusCode::CREATED, Json(StoryEventResponse::from(event))))
}

/// Update a story event (summary, visibility, tags)
pub async fn update_story_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    Json(req): Json<UpdateStoryEventRequest>,
) -> Result<Json<StoryEventResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&event_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid event ID".to_string()))?;
    let event_id = StoryEventId::from_uuid(uuid);

    // Get existing event
    let event = state
        .story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    // Apply updates
    if let Some(summary) = req.summary {
        state
            .story_event_service
            .update_summary(event_id, &summary)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(is_hidden) = req.is_hidden {
        state
            .story_event_service
            .set_hidden(event_id, is_hidden)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(tags) = req.tags {
        state
            .story_event_service
            .update_tags(event_id, tags)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Fetch updated event
    let updated_event = state
        .story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    Ok(Json(StoryEventResponse::from(updated_event)))
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
        .story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    let new_hidden = !event.is_hidden;
    state
        .story_event_service
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
        .story_event_service
        .get_event(event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Story event not found".to_string()))?;

    // Delete it
    state
        .story_event_service
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
        .story_event_service
        .count_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(count))
}
