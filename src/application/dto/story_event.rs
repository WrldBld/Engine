use serde::{Deserialize, Serialize};

use crate::domain::entities::{DmMarkerType, ItemSource, MarkerImportance, StoryEvent, StoryEventType};

/// Query parameters for listing story events.
#[derive(Debug, Deserialize)]
pub struct ListStoryEventsQueryDto {
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

/// Request to create a DM marker story event.
#[derive(Debug, Deserialize)]
pub struct CreateDmMarkerRequestDto {
    pub session_id: String,
    pub title: String,
    pub note: String,
    #[serde(default)]
    pub importance: MarkerImportanceRequestDto,
    #[serde(default)]
    pub marker_type: DmMarkerTypeRequestDto,
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

/// Request to update a story event.
#[derive(Debug, Deserialize)]
pub struct UpdateStoryEventRequestDto {
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub is_hidden: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Marker importance for request.
#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarkerImportanceRequestDto {
    #[default]
    Minor,
    Notable,
    Major,
    Critical,
}

impl From<MarkerImportanceRequestDto> for MarkerImportance {
    fn from(req: MarkerImportanceRequestDto) -> Self {
        match req {
            MarkerImportanceRequestDto::Minor => MarkerImportance::Minor,
            MarkerImportanceRequestDto::Notable => MarkerImportance::Notable,
            MarkerImportanceRequestDto::Major => MarkerImportance::Major,
            MarkerImportanceRequestDto::Critical => MarkerImportance::Critical,
        }
    }
}

impl From<MarkerImportance> for MarkerImportanceRequestDto {
    fn from(m: MarkerImportance) -> Self {
        match m {
            MarkerImportance::Minor => MarkerImportanceRequestDto::Minor,
            MarkerImportance::Notable => MarkerImportanceRequestDto::Notable,
            MarkerImportance::Major => MarkerImportanceRequestDto::Major,
            MarkerImportance::Critical => MarkerImportanceRequestDto::Critical,
        }
    }
}

/// DM marker type for request.
#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DmMarkerTypeRequestDto {
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

impl From<DmMarkerTypeRequestDto> for DmMarkerType {
    fn from(req: DmMarkerTypeRequestDto) -> Self {
        match req {
            DmMarkerTypeRequestDto::Note => DmMarkerType::Note,
            DmMarkerTypeRequestDto::PlotPoint => DmMarkerType::PlotPoint,
            DmMarkerTypeRequestDto::CharacterMoment => DmMarkerType::CharacterMoment,
            DmMarkerTypeRequestDto::WorldEvent => DmMarkerType::WorldEvent,
            DmMarkerTypeRequestDto::PlayerDecision => DmMarkerType::PlayerDecision,
            DmMarkerTypeRequestDto::Foreshadowing => DmMarkerType::Foreshadowing,
            DmMarkerTypeRequestDto::Callback => DmMarkerType::Callback,
            DmMarkerTypeRequestDto::Custom => DmMarkerType::Custom,
        }
    }
}

impl From<DmMarkerType> for DmMarkerTypeRequestDto {
    fn from(m: DmMarkerType) -> Self {
        match m {
            DmMarkerType::Note => DmMarkerTypeRequestDto::Note,
            DmMarkerType::PlotPoint => DmMarkerTypeRequestDto::PlotPoint,
            DmMarkerType::CharacterMoment => DmMarkerTypeRequestDto::CharacterMoment,
            DmMarkerType::WorldEvent => DmMarkerTypeRequestDto::WorldEvent,
            DmMarkerType::PlayerDecision => DmMarkerTypeRequestDto::PlayerDecision,
            DmMarkerType::Foreshadowing => DmMarkerTypeRequestDto::Foreshadowing,
            DmMarkerType::Callback => DmMarkerTypeRequestDto::Callback,
            DmMarkerType::Custom => DmMarkerTypeRequestDto::Custom,
        }
    }
}

/// Story event response.
///
/// # Graph-First Architecture
///
/// Session, scene, location, involved_characters, and triggered_by are now stored as
/// graph edges and must be provided separately when constructing this DTO.
#[derive(Debug, Serialize)]
pub struct StoryEventResponseDto {
    pub id: String,
    pub world_id: String,
    /// Session ID from OCCURRED_IN_SESSION edge (optional until we have edge data)
    pub session_id: Option<String>,
    /// Scene ID from OCCURRED_IN_SCENE edge
    pub scene_id: Option<String>,
    /// Location ID from OCCURRED_AT edge
    pub location_id: Option<String>,
    pub event_type: StoryEventTypeResponseDto,
    pub timestamp: String,
    pub game_time: Option<String>,
    pub summary: String,
    /// Character IDs from INVOLVES edges (with roles)
    pub involved_characters: Vec<InvolvedCharacterResponseDto>,
    pub is_hidden: bool,
    pub tags: Vec<String>,
    /// Narrative event ID from TRIGGERED_BY_NARRATIVE edge
    pub triggered_by: Option<String>,
    pub type_name: String,
}

/// Response DTO for involved character (from INVOLVES edge)
#[derive(Debug, Serialize)]
pub struct InvolvedCharacterResponseDto {
    pub character_id: String,
    pub role: String,
}

impl StoryEventResponseDto {
    /// Create a response DTO with edge data
    ///
    /// Use this when you have fetched edge data separately from the repository.
    pub fn with_edges(
        event: StoryEvent,
        session_id: Option<String>,
        scene_id: Option<String>,
        location_id: Option<String>,
        involved_characters: Vec<InvolvedCharacterResponseDto>,
        triggered_by: Option<String>,
    ) -> Self {
        let type_name = event.type_name().to_string();
        Self {
            id: event.id.to_string(),
            world_id: event.world_id.to_string(),
            session_id,
            scene_id,
            location_id,
            event_type: StoryEventTypeResponseDto::from(event.event_type),
            timestamp: event.timestamp.to_rfc3339(),
            game_time: event.game_time,
            summary: event.summary,
            involved_characters,
            is_hidden: event.is_hidden,
            tags: event.tags,
            triggered_by,
            type_name,
        }
    }
}

impl From<StoryEvent> for StoryEventResponseDto {
    /// Create a minimal response DTO without edge data.
    ///
    /// NOTE: This creates a DTO with edge fields set to None/empty.
    /// For full response with edges, use `StoryEventResponseDto::with_edges()`.
    fn from(e: StoryEvent) -> Self {
        let type_name = e.type_name().to_string();
        Self {
            id: e.id.to_string(),
            world_id: e.world_id.to_string(),
            session_id: None,
            scene_id: None,
            location_id: None,
            event_type: StoryEventTypeResponseDto::from(e.event_type),
            timestamp: e.timestamp.to_rfc3339(),
            game_time: e.game_time,
            summary: e.summary,
            involved_characters: Vec::new(),
            is_hidden: e.is_hidden,
            tags: e.tags,
            triggered_by: None,
            type_name,
        }
    }
}

/// Story event type response (simplified for API).
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoryEventTypeResponseDto {
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
        source: ItemSourceResponseDto,
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

impl From<StoryEventType> for StoryEventTypeResponseDto {
    fn from(e: StoryEventType) -> Self {
        match e {
            StoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => StoryEventTypeResponseDto::LocationChange {
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
            } => StoryEventTypeResponseDto::DialogueExchange {
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
            } => StoryEventTypeResponseDto::CombatEvent {
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
            } => StoryEventTypeResponseDto::ChallengeAttempted {
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
            } => StoryEventTypeResponseDto::ItemAcquired {
                item_name,
                item_description,
                character_id: character_id.to_string(),
                source: ItemSourceResponseDto::from(source),
                quantity,
            },
            StoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => StoryEventTypeResponseDto::ItemTransferred {
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
            } => StoryEventTypeResponseDto::ItemUsed {
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
            } => StoryEventTypeResponseDto::RelationshipChanged {
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
            } => StoryEventTypeResponseDto::SceneTransition {
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
            } => StoryEventTypeResponseDto::InformationRevealed {
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
            } => StoryEventTypeResponseDto::NpcAction {
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
            } => StoryEventTypeResponseDto::DmMarker {
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
            } => StoryEventTypeResponseDto::NarrativeEventTriggered {
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
            } => StoryEventTypeResponseDto::StatModified {
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
            } => StoryEventTypeResponseDto::FlagChanged {
                flag_name,
                new_value,
                reason,
            },
            StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => StoryEventTypeResponseDto::SessionStarted {
                session_number,
                session_name,
                players_present,
            },
            StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => StoryEventTypeResponseDto::SessionEnded {
                duration_minutes,
                summary,
            },
            StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            } => StoryEventTypeResponseDto::Custom {
                event_subtype,
                title,
                description,
                data,
            },
        }
    }
}

/// Item source response.
#[derive(Debug, Serialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum ItemSourceResponseDto {
    Found { location: String },
    Purchased { from: String, cost: Option<String> },
    Gifted { from: String },
    Looted { from: String },
    Crafted,
    Reward { for_what: String },
    Stolen { from: String },
    Custom { description: String },
}

impl From<ItemSource> for ItemSourceResponseDto {
    fn from(s: ItemSource) -> Self {
        match s {
            ItemSource::Found { location } => ItemSourceResponseDto::Found { location },
            ItemSource::Purchased { from, cost } => ItemSourceResponseDto::Purchased { from, cost },
            ItemSource::Gifted { from } => ItemSourceResponseDto::Gifted {
                from: from.to_string(),
            },
            ItemSource::Looted { from } => ItemSourceResponseDto::Looted { from },
            ItemSource::Crafted => ItemSourceResponseDto::Crafted,
            ItemSource::Reward { for_what } => ItemSourceResponseDto::Reward { for_what },
            ItemSource::Stolen { from } => ItemSourceResponseDto::Stolen { from },
            ItemSource::Custom { description } => ItemSourceResponseDto::Custom { description },
        }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedStoryEventsResponseDto {
    pub events: Vec<StoryEventResponseDto>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

