//! StoryEvent repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::StoryEventRepositoryPort;
use crate::domain::entities::{
    ChallengeEventOutcome, CombatEventType, CombatOutcome, DmMarkerType, InfoImportance, InfoType,
    InvolvedCharacter, ItemSource, MarkerImportance, StoryEvent, StoryEventType,
};
use crate::domain::value_objects::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId, SessionId, StoryEventId,
    WorldId,
};

// ============================================================================
// Storage DTOs for StoryEventType
// These types have serde derives for JSON persistence in Neo4j
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredStoryEventType {
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
        combat_type: StoredCombatEventType,
        participants: Vec<String>,
        enemies: Vec<String>,
        outcome: Option<StoredCombatOutcome>,
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
        outcome: StoredChallengeEventOutcome,
    },
    ItemAcquired {
        item_name: String,
        item_description: Option<String>,
        character_id: String,
        source: StoredItemSource,
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
        info_type: StoredInfoType,
        title: String,
        content: String,
        source: Option<String>,
        importance: StoredInfoImportance,
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
        importance: StoredMarkerImportance,
        marker_type: StoredDmMarkerType,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredCombatEventType {
    Started,
    RoundCompleted,
    CharacterDefeated,
    CharacterFled,
    Ended,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredCombatOutcome {
    Victory,
    Defeat,
    Fled,
    Negotiated,
    Draw,
    Interrupted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredChallengeEventOutcome {
    CriticalSuccess,
    Success,
    PartialSuccess,
    Failure,
    CriticalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source_type")]
enum StoredItemSource {
    Found { location: String },
    Purchased { from: String, cost: Option<String> },
    Gifted { from: String },
    Looted { from: String },
    Crafted,
    Reward { for_what: String },
    Stolen { from: String },
    Custom { description: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredInfoType {
    Lore,
    Quest,
    Character,
    Location,
    Item,
    Secret,
    Rumor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredInfoImportance {
    Minor,
    Notable,
    Major,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredMarkerImportance {
    Minor,
    Notable,
    Major,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredDmMarkerType {
    Note,
    PlotPoint,
    CharacterMoment,
    WorldEvent,
    PlayerDecision,
    Foreshadowing,
    Callback,
    Custom,
}

// Conversion from domain to stored types
impl From<&StoryEventType> for StoredStoryEventType {
    fn from(e: &StoryEventType) -> Self {
        match e {
            StoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => StoredStoryEventType::LocationChange {
                from_location: from_location.map(|id| id.to_string()),
                to_location: to_location.to_string(),
                character_id: character_id.to_string(),
                travel_method: travel_method.clone(),
            },
            StoryEventType::DialogueExchange {
                npc_id,
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            } => StoredStoryEventType::DialogueExchange {
                npc_id: npc_id.to_string(),
                npc_name: npc_name.clone(),
                player_dialogue: player_dialogue.clone(),
                npc_response: npc_response.clone(),
                topics_discussed: topics_discussed.clone(),
                tone: tone.clone(),
            },
            StoryEventType::CombatEvent {
                combat_type,
                participants,
                enemies,
                outcome,
                location_id,
                rounds,
            } => StoredStoryEventType::CombatEvent {
                combat_type: (*combat_type).into(),
                participants: participants.iter().map(|id| id.to_string()).collect(),
                enemies: enemies.clone(),
                outcome: outcome.map(|o| o.into()),
                location_id: location_id.to_string(),
                rounds: *rounds,
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
            } => StoredStoryEventType::ChallengeAttempted {
                challenge_id: challenge_id.map(|id| id.to_string()),
                challenge_name: challenge_name.clone(),
                character_id: character_id.to_string(),
                skill_used: skill_used.clone(),
                difficulty: difficulty.clone(),
                roll_result: *roll_result,
                modifier: *modifier,
                outcome: (*outcome).into(),
            },
            StoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id,
                source,
                quantity,
            } => StoredStoryEventType::ItemAcquired {
                item_name: item_name.clone(),
                item_description: item_description.clone(),
                character_id: character_id.to_string(),
                source: source.into(),
                quantity: *quantity,
            },
            StoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => StoredStoryEventType::ItemTransferred {
                item_name: item_name.clone(),
                from_character: from_character.map(|id| id.to_string()),
                to_character: to_character.to_string(),
                quantity: *quantity,
                reason: reason.clone(),
            },
            StoryEventType::ItemUsed {
                item_name,
                character_id,
                target,
                effect,
                consumed,
            } => StoredStoryEventType::ItemUsed {
                item_name: item_name.clone(),
                character_id: character_id.to_string(),
                target: target.clone(),
                effect: effect.clone(),
                consumed: *consumed,
            },
            StoryEventType::RelationshipChanged {
                from_character,
                to_character,
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            } => StoredStoryEventType::RelationshipChanged {
                from_character: from_character.to_string(),
                to_character: to_character.to_string(),
                previous_sentiment: *previous_sentiment,
                new_sentiment: *new_sentiment,
                sentiment_change: *sentiment_change,
                reason: reason.clone(),
            },
            StoryEventType::SceneTransition {
                from_scene,
                to_scene,
                from_scene_name,
                to_scene_name,
                trigger_reason,
            } => StoredStoryEventType::SceneTransition {
                from_scene: from_scene.map(|id| id.to_string()),
                to_scene: to_scene.to_string(),
                from_scene_name: from_scene_name.clone(),
                to_scene_name: to_scene_name.clone(),
                trigger_reason: trigger_reason.clone(),
            },
            StoryEventType::InformationRevealed {
                info_type,
                title,
                content,
                source,
                importance,
                persist_to_journal,
            } => StoredStoryEventType::InformationRevealed {
                info_type: (*info_type).into(),
                title: title.clone(),
                content: content.clone(),
                source: source.map(|id| id.to_string()),
                importance: (*importance).into(),
                persist_to_journal: *persist_to_journal,
            },
            StoryEventType::NpcAction {
                npc_id,
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            } => StoredStoryEventType::NpcAction {
                npc_id: npc_id.to_string(),
                npc_name: npc_name.clone(),
                action_type: action_type.clone(),
                description: description.clone(),
                dm_approved: *dm_approved,
                dm_modified: *dm_modified,
            },
            StoryEventType::DmMarker {
                title,
                note,
                importance,
                marker_type,
            } => StoredStoryEventType::DmMarker {
                title: title.clone(),
                note: note.clone(),
                importance: (*importance).into(),
                marker_type: (*marker_type).into(),
            },
            StoryEventType::NarrativeEventTriggered {
                narrative_event_id,
                narrative_event_name,
                outcome_branch,
                effects_applied,
            } => StoredStoryEventType::NarrativeEventTriggered {
                narrative_event_id: narrative_event_id.to_string(),
                narrative_event_name: narrative_event_name.clone(),
                outcome_branch: outcome_branch.clone(),
                effects_applied: effects_applied.clone(),
            },
            StoryEventType::StatModified {
                character_id,
                stat_name,
                previous_value,
                new_value,
                reason,
            } => StoredStoryEventType::StatModified {
                character_id: character_id.to_string(),
                stat_name: stat_name.clone(),
                previous_value: *previous_value,
                new_value: *new_value,
                reason: reason.clone(),
            },
            StoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            } => StoredStoryEventType::FlagChanged {
                flag_name: flag_name.clone(),
                new_value: *new_value,
                reason: reason.clone(),
            },
            StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => StoredStoryEventType::SessionStarted {
                session_number: *session_number,
                session_name: session_name.clone(),
                players_present: players_present.clone(),
            },
            StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => StoredStoryEventType::SessionEnded {
                duration_minutes: *duration_minutes,
                summary: summary.clone(),
            },
            StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            } => StoredStoryEventType::Custom {
                event_subtype: event_subtype.clone(),
                title: title.clone(),
                description: description.clone(),
                data: data.clone(),
            },
        }
    }
}

impl From<CombatEventType> for StoredCombatEventType {
    fn from(c: CombatEventType) -> Self {
        match c {
            CombatEventType::Started => StoredCombatEventType::Started,
            CombatEventType::RoundCompleted => StoredCombatEventType::RoundCompleted,
            CombatEventType::CharacterDefeated => StoredCombatEventType::CharacterDefeated,
            CombatEventType::CharacterFled => StoredCombatEventType::CharacterFled,
            CombatEventType::Ended => StoredCombatEventType::Ended,
        }
    }
}

impl From<CombatOutcome> for StoredCombatOutcome {
    fn from(c: CombatOutcome) -> Self {
        match c {
            CombatOutcome::Victory => StoredCombatOutcome::Victory,
            CombatOutcome::Defeat => StoredCombatOutcome::Defeat,
            CombatOutcome::Fled => StoredCombatOutcome::Fled,
            CombatOutcome::Negotiated => StoredCombatOutcome::Negotiated,
            CombatOutcome::Draw => StoredCombatOutcome::Draw,
            CombatOutcome::Interrupted => StoredCombatOutcome::Interrupted,
        }
    }
}

impl From<ChallengeEventOutcome> for StoredChallengeEventOutcome {
    fn from(c: ChallengeEventOutcome) -> Self {
        match c {
            ChallengeEventOutcome::CriticalSuccess => StoredChallengeEventOutcome::CriticalSuccess,
            ChallengeEventOutcome::Success => StoredChallengeEventOutcome::Success,
            ChallengeEventOutcome::PartialSuccess => StoredChallengeEventOutcome::PartialSuccess,
            ChallengeEventOutcome::Failure => StoredChallengeEventOutcome::Failure,
            ChallengeEventOutcome::CriticalFailure => StoredChallengeEventOutcome::CriticalFailure,
        }
    }
}

impl From<&ItemSource> for StoredItemSource {
    fn from(s: &ItemSource) -> Self {
        match s {
            ItemSource::Found { location } => StoredItemSource::Found {
                location: location.clone(),
            },
            ItemSource::Purchased { from, cost } => StoredItemSource::Purchased {
                from: from.clone(),
                cost: cost.clone(),
            },
            ItemSource::Gifted { from } => StoredItemSource::Gifted {
                from: from.to_string(),
            },
            ItemSource::Looted { from } => StoredItemSource::Looted { from: from.clone() },
            ItemSource::Crafted => StoredItemSource::Crafted,
            ItemSource::Reward { for_what } => StoredItemSource::Reward {
                for_what: for_what.clone(),
            },
            ItemSource::Stolen { from } => StoredItemSource::Stolen { from: from.clone() },
            ItemSource::Custom { description } => StoredItemSource::Custom {
                description: description.clone(),
            },
        }
    }
}

impl From<InfoType> for StoredInfoType {
    fn from(i: InfoType) -> Self {
        match i {
            InfoType::Lore => StoredInfoType::Lore,
            InfoType::Quest => StoredInfoType::Quest,
            InfoType::Character => StoredInfoType::Character,
            InfoType::Location => StoredInfoType::Location,
            InfoType::Item => StoredInfoType::Item,
            InfoType::Secret => StoredInfoType::Secret,
            InfoType::Rumor => StoredInfoType::Rumor,
        }
    }
}

impl From<InfoImportance> for StoredInfoImportance {
    fn from(i: InfoImportance) -> Self {
        match i {
            InfoImportance::Minor => StoredInfoImportance::Minor,
            InfoImportance::Notable => StoredInfoImportance::Notable,
            InfoImportance::Major => StoredInfoImportance::Major,
            InfoImportance::Critical => StoredInfoImportance::Critical,
        }
    }
}

impl From<MarkerImportance> for StoredMarkerImportance {
    fn from(m: MarkerImportance) -> Self {
        match m {
            MarkerImportance::Minor => StoredMarkerImportance::Minor,
            MarkerImportance::Notable => StoredMarkerImportance::Notable,
            MarkerImportance::Major => StoredMarkerImportance::Major,
            MarkerImportance::Critical => StoredMarkerImportance::Critical,
        }
    }
}

impl From<DmMarkerType> for StoredDmMarkerType {
    fn from(d: DmMarkerType) -> Self {
        match d {
            DmMarkerType::Note => StoredDmMarkerType::Note,
            DmMarkerType::PlotPoint => StoredDmMarkerType::PlotPoint,
            DmMarkerType::CharacterMoment => StoredDmMarkerType::CharacterMoment,
            DmMarkerType::WorldEvent => StoredDmMarkerType::WorldEvent,
            DmMarkerType::PlayerDecision => StoredDmMarkerType::PlayerDecision,
            DmMarkerType::Foreshadowing => StoredDmMarkerType::Foreshadowing,
            DmMarkerType::Callback => StoredDmMarkerType::Callback,
            DmMarkerType::Custom => StoredDmMarkerType::Custom,
        }
    }
}

// Conversion from stored to domain types (deserialization)
impl From<StoredStoryEventType> for StoryEventType {
    fn from(s: StoredStoryEventType) -> Self {
        match s {
            StoredStoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => StoryEventType::LocationChange {
                from_location: from_location
                    .and_then(|id| Uuid::parse_str(&id).ok().map(LocationId::from)),
                to_location: LocationId::from(Uuid::parse_str(&to_location).unwrap_or_default()),
                character_id: CharacterId::from(Uuid::parse_str(&character_id).unwrap_or_default()),
                travel_method,
            },
            StoredStoryEventType::DialogueExchange {
                npc_id,
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            } => StoryEventType::DialogueExchange {
                npc_id: CharacterId::from(Uuid::parse_str(&npc_id).unwrap_or_default()),
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            },
            StoredStoryEventType::CombatEvent {
                combat_type,
                participants,
                enemies,
                outcome,
                location_id,
                rounds,
            } => StoryEventType::CombatEvent {
                combat_type: combat_type.into(),
                participants: participants
                    .into_iter()
                    .filter_map(|id| Uuid::parse_str(&id).ok().map(CharacterId::from))
                    .collect(),
                enemies,
                outcome: outcome.map(|o| o.into()),
                location_id: LocationId::from(Uuid::parse_str(&location_id).unwrap_or_default()),
                rounds,
            },
            StoredStoryEventType::ChallengeAttempted {
                challenge_id,
                challenge_name,
                character_id,
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome,
            } => StoryEventType::ChallengeAttempted {
                challenge_id: challenge_id
                    .and_then(|id| Uuid::parse_str(&id).ok().map(ChallengeId::from)),
                challenge_name,
                character_id: CharacterId::from(Uuid::parse_str(&character_id).unwrap_or_default()),
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome: outcome.into(),
            },
            StoredStoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id,
                source,
                quantity,
            } => StoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id: CharacterId::from(Uuid::parse_str(&character_id).unwrap_or_default()),
                source: source.into(),
                quantity,
            },
            StoredStoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => StoryEventType::ItemTransferred {
                item_name,
                from_character: from_character
                    .and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                to_character: CharacterId::from(Uuid::parse_str(&to_character).unwrap_or_default()),
                quantity,
                reason,
            },
            StoredStoryEventType::ItemUsed {
                item_name,
                character_id,
                target,
                effect,
                consumed,
            } => StoryEventType::ItemUsed {
                item_name,
                character_id: CharacterId::from(Uuid::parse_str(&character_id).unwrap_or_default()),
                target,
                effect,
                consumed,
            },
            StoredStoryEventType::RelationshipChanged {
                from_character,
                to_character,
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            } => StoryEventType::RelationshipChanged {
                from_character: CharacterId::from(
                    Uuid::parse_str(&from_character).unwrap_or_default(),
                ),
                to_character: CharacterId::from(
                    Uuid::parse_str(&to_character).unwrap_or_default(),
                ),
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            },
            StoredStoryEventType::SceneTransition {
                from_scene,
                to_scene,
                from_scene_name,
                to_scene_name,
                trigger_reason,
            } => StoryEventType::SceneTransition {
                from_scene: from_scene.and_then(|id| Uuid::parse_str(&id).ok().map(SceneId::from)),
                to_scene: SceneId::from(Uuid::parse_str(&to_scene).unwrap_or_default()),
                from_scene_name,
                to_scene_name,
                trigger_reason,
            },
            StoredStoryEventType::InformationRevealed {
                info_type,
                title,
                content,
                source,
                importance,
                persist_to_journal,
            } => StoryEventType::InformationRevealed {
                info_type: info_type.into(),
                title,
                content,
                source: source.and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                importance: importance.into(),
                persist_to_journal,
            },
            StoredStoryEventType::NpcAction {
                npc_id,
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            } => StoryEventType::NpcAction {
                npc_id: CharacterId::from(Uuid::parse_str(&npc_id).unwrap_or_default()),
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            },
            StoredStoryEventType::DmMarker {
                title,
                note,
                importance,
                marker_type,
            } => StoryEventType::DmMarker {
                title,
                note,
                importance: importance.into(),
                marker_type: marker_type.into(),
            },
            StoredStoryEventType::NarrativeEventTriggered {
                narrative_event_id,
                narrative_event_name,
                outcome_branch,
                effects_applied,
            } => StoryEventType::NarrativeEventTriggered {
                narrative_event_id: NarrativeEventId::from(
                    Uuid::parse_str(&narrative_event_id).unwrap_or_default(),
                ),
                narrative_event_name,
                outcome_branch,
                effects_applied,
            },
            StoredStoryEventType::StatModified {
                character_id,
                stat_name,
                previous_value,
                new_value,
                reason,
            } => StoryEventType::StatModified {
                character_id: CharacterId::from(Uuid::parse_str(&character_id).unwrap_or_default()),
                stat_name,
                previous_value,
                new_value,
                reason,
            },
            StoredStoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            } => StoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            },
            StoredStoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            },
            StoredStoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            },
            StoredStoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            } => StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            },
        }
    }
}

impl From<StoredCombatEventType> for CombatEventType {
    fn from(s: StoredCombatEventType) -> Self {
        match s {
            StoredCombatEventType::Started => CombatEventType::Started,
            StoredCombatEventType::RoundCompleted => CombatEventType::RoundCompleted,
            StoredCombatEventType::CharacterDefeated => CombatEventType::CharacterDefeated,
            StoredCombatEventType::CharacterFled => CombatEventType::CharacterFled,
            StoredCombatEventType::Ended => CombatEventType::Ended,
        }
    }
}

impl From<StoredCombatOutcome> for CombatOutcome {
    fn from(s: StoredCombatOutcome) -> Self {
        match s {
            StoredCombatOutcome::Victory => CombatOutcome::Victory,
            StoredCombatOutcome::Defeat => CombatOutcome::Defeat,
            StoredCombatOutcome::Fled => CombatOutcome::Fled,
            StoredCombatOutcome::Negotiated => CombatOutcome::Negotiated,
            StoredCombatOutcome::Draw => CombatOutcome::Draw,
            StoredCombatOutcome::Interrupted => CombatOutcome::Interrupted,
        }
    }
}

impl From<StoredChallengeEventOutcome> for ChallengeEventOutcome {
    fn from(s: StoredChallengeEventOutcome) -> Self {
        match s {
            StoredChallengeEventOutcome::CriticalSuccess => ChallengeEventOutcome::CriticalSuccess,
            StoredChallengeEventOutcome::Success => ChallengeEventOutcome::Success,
            StoredChallengeEventOutcome::PartialSuccess => ChallengeEventOutcome::PartialSuccess,
            StoredChallengeEventOutcome::Failure => ChallengeEventOutcome::Failure,
            StoredChallengeEventOutcome::CriticalFailure => ChallengeEventOutcome::CriticalFailure,
        }
    }
}

impl From<StoredItemSource> for ItemSource {
    fn from(s: StoredItemSource) -> Self {
        match s {
            StoredItemSource::Found { location } => ItemSource::Found { location },
            StoredItemSource::Purchased { from, cost } => ItemSource::Purchased { from, cost },
            StoredItemSource::Gifted { from } => ItemSource::Gifted {
                from: CharacterId::from(Uuid::parse_str(&from).unwrap_or_default()),
            },
            StoredItemSource::Looted { from } => ItemSource::Looted { from },
            StoredItemSource::Crafted => ItemSource::Crafted,
            StoredItemSource::Reward { for_what } => ItemSource::Reward { for_what },
            StoredItemSource::Stolen { from } => ItemSource::Stolen { from },
            StoredItemSource::Custom { description } => ItemSource::Custom { description },
        }
    }
}

impl From<StoredInfoType> for InfoType {
    fn from(s: StoredInfoType) -> Self {
        match s {
            StoredInfoType::Lore => InfoType::Lore,
            StoredInfoType::Quest => InfoType::Quest,
            StoredInfoType::Character => InfoType::Character,
            StoredInfoType::Location => InfoType::Location,
            StoredInfoType::Item => InfoType::Item,
            StoredInfoType::Secret => InfoType::Secret,
            StoredInfoType::Rumor => InfoType::Rumor,
        }
    }
}

impl From<StoredInfoImportance> for InfoImportance {
    fn from(s: StoredInfoImportance) -> Self {
        match s {
            StoredInfoImportance::Minor => InfoImportance::Minor,
            StoredInfoImportance::Notable => InfoImportance::Notable,
            StoredInfoImportance::Major => InfoImportance::Major,
            StoredInfoImportance::Critical => InfoImportance::Critical,
        }
    }
}

impl From<StoredMarkerImportance> for MarkerImportance {
    fn from(s: StoredMarkerImportance) -> Self {
        match s {
            StoredMarkerImportance::Minor => MarkerImportance::Minor,
            StoredMarkerImportance::Notable => MarkerImportance::Notable,
            StoredMarkerImportance::Major => MarkerImportance::Major,
            StoredMarkerImportance::Critical => MarkerImportance::Critical,
        }
    }
}

impl From<StoredDmMarkerType> for DmMarkerType {
    fn from(s: StoredDmMarkerType) -> Self {
        match s {
            StoredDmMarkerType::Note => DmMarkerType::Note,
            StoredDmMarkerType::PlotPoint => DmMarkerType::PlotPoint,
            StoredDmMarkerType::CharacterMoment => DmMarkerType::CharacterMoment,
            StoredDmMarkerType::WorldEvent => DmMarkerType::WorldEvent,
            StoredDmMarkerType::PlayerDecision => DmMarkerType::PlayerDecision,
            StoredDmMarkerType::Foreshadowing => DmMarkerType::Foreshadowing,
            StoredDmMarkerType::Callback => DmMarkerType::Callback,
            StoredDmMarkerType::Custom => DmMarkerType::Custom,
        }
    }
}

/// Repository for StoryEvent operations
pub struct Neo4jStoryEventRepository {
    connection: Neo4jConnection,
}

impl Neo4jStoryEventRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl StoryEventRepositoryPort for Neo4jStoryEventRepository {
    /// Create a new story event
    ///
    /// NOTE: Session, location, scene, involved characters, triggered_by, and recorded_challenge
    /// associations are now stored as graph edges and must be created separately using the
    /// edge methods after calling create().
    async fn create(&self, event: &StoryEvent) -> Result<()> {
        let stored_event_type: StoredStoryEventType = (&event.event_type).into();
        let event_type_json = serde_json::to_string(&stored_event_type)?;
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (e:StoryEvent {
                id: $id,
                world_id: $world_id,
                event_type_json: $event_type_json,
                timestamp: $timestamp,
                game_time: $game_time,
                summary: $summary,
                is_hidden: $is_hidden,
                tags_json: $tags_json
            })
            CREATE (w)-[:HAS_STORY_EVENT]->(e)
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("world_id", event.world_id.to_string())
        .param("event_type_json", event_type_json)
        .param("timestamp", event.timestamp.to_rfc3339())
        .param(
            "game_time",
            event.game_time.clone().unwrap_or_default(),
        )
        .param("summary", event.summary.clone())
        .param("is_hidden", event.is_hidden)
        .param("tags_json", tags_json);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created story event: {}", event.id);

        // NOTE: Session, location, scene, involved characters, triggered_by, and
        // recorded_challenge edges should be created separately using the edge methods

        Ok(())
    }

    /// Get a story event by ID
    async fn get(&self, id: StoryEventId) -> Result<Option<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            RETURN e",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_story_event(row)?))
        } else {
            Ok(None)
        }
    }

    /// List story events for a session (via OCCURRED_IN_SESSION edge)
    async fn list_by_session(&self, session_id: SessionId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:OCCURRED_IN_SESSION]->(s:Session {id: $session_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("session_id", session_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List story events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List story events for a world with pagination
    async fn list_by_world_paginated(
        &self,
        world_id: WorldId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN e
            ORDER BY e.timestamp DESC
            SKIP $offset
            LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("offset", offset as i64)
        .param("limit", limit as i64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List visible (non-hidden) story events for a world
    async fn list_visible(&self, world_id: WorldId, limit: u32) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE e.is_hidden = false
            RETURN e
            ORDER BY e.timestamp DESC
            LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("limit", limit as i64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// Search story events by tags
    async fn search_by_tags(
        &self,
        world_id: WorldId,
        tags: Vec<String>,
    ) -> Result<Vec<StoryEvent>> {
        // Note: We store tags as JSON, so we search in the JSON string
        // A more efficient approach would be to store tags as separate nodes
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE ANY(tag IN $tags WHERE e.tags_json CONTAINS tag)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string())
        .param("tags", tags);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// Search story events by text in summary
    async fn search_by_text(
        &self,
        world_id: WorldId,
        search_text: &str,
    ) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE toLower(e.summary) CONTAINS toLower($search_text)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string())
        .param("search_text", search_text);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events involving a specific character (via INVOLVES edge)
    async fn list_by_character(&self, character_id: CharacterId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:INVOLVES]->(c:Character {id: $char_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("char_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events at a specific location (via OCCURRED_AT edge)
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:OCCURRED_AT]->(l:Location {id: $location_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// Update story event summary (DM editing)
    async fn update_summary(&self, id: StoryEventId, summary: &str) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.summary = $summary
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("summary", summary);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Update event visibility
    async fn set_hidden(&self, id: StoryEventId, is_hidden: bool) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.is_hidden = $is_hidden
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("is_hidden", is_hidden);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Update event tags
    async fn update_tags(&self, id: StoryEventId, tags: Vec<String>) -> Result<bool> {
        let tags_json = serde_json::to_string(&tags)?;
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.tags_json = $tags_json
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("tags_json", tags_json);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Delete a story event (rarely used - events are usually immutable)
    async fn delete(&self, id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            DETACH DELETE e
            RETURN count(*) as deleted",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    /// Count events for a world
    async fn count_by_world(&self, world_id: WorldId) -> Result<u64> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN count(e) as count",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            Ok(count as u64)
        } else {
            Ok(0)
        }
    }

    // =========================================================================
    // OCCURRED_IN_SESSION Edge Methods
    // =========================================================================

    /// Set the session for a story event (creates OCCURRED_IN_SESSION edge)
    async fn set_session(&self, event_id: StoryEventId, session_id: SessionId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (s:Session {id: $session_id})
            MERGE (e)-[:OCCURRED_IN_SESSION]->(s)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("session_id", session_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the session for a story event
    async fn get_session(&self, event_id: StoryEventId) -> Result<Option<SessionId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:OCCURRED_IN_SESSION]->(s:Session)
            RETURN s.id as session_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let session_id_str: String = row.get("session_id")?;
            Ok(Some(SessionId::from(Uuid::parse_str(&session_id_str)?)))
        } else {
            Ok(None)
        }
    }

    // =========================================================================
    // OCCURRED_AT Edge Methods (Location)
    // =========================================================================

    /// Set the location where event occurred (creates OCCURRED_AT edge)
    async fn set_location(&self, event_id: StoryEventId, location_id: LocationId) -> Result<bool> {
        // First remove any existing location edge
        let remove_q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:OCCURRED_AT]->(:Location)
            DELETE r",
        )
        .param("event_id", event_id.to_string());
        let _ = self.connection.graph().run(remove_q).await;

        // Create new edge
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (l:Location {id: $location_id})
            CREATE (e)-[:OCCURRED_AT]->(l)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the location where event occurred
    async fn get_location(&self, event_id: StoryEventId) -> Result<Option<LocationId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:OCCURRED_AT]->(l:Location)
            RETURN l.id as location_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let location_id_str: String = row.get("location_id")?;
            Ok(Some(LocationId::from(Uuid::parse_str(&location_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove location association (deletes OCCURRED_AT edge)
    async fn remove_location(&self, event_id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:OCCURRED_AT]->(:Location)
            DELETE r
            RETURN count(r) as deleted",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // OCCURRED_IN_SCENE Edge Methods
    // =========================================================================

    /// Set the scene where event occurred (creates OCCURRED_IN_SCENE edge)
    async fn set_scene(&self, event_id: StoryEventId, scene_id: SceneId) -> Result<bool> {
        // First remove any existing scene edge
        let remove_q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:OCCURRED_IN_SCENE]->(:Scene)
            DELETE r",
        )
        .param("event_id", event_id.to_string());
        let _ = self.connection.graph().run(remove_q).await;

        // Create new edge
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (s:Scene {id: $scene_id})
            CREATE (e)-[:OCCURRED_IN_SCENE]->(s)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the scene where event occurred
    async fn get_scene(&self, event_id: StoryEventId) -> Result<Option<SceneId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:OCCURRED_IN_SCENE]->(s:Scene)
            RETURN s.id as scene_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let scene_id_str: String = row.get("scene_id")?;
            Ok(Some(SceneId::from(Uuid::parse_str(&scene_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove scene association (deletes OCCURRED_IN_SCENE edge)
    async fn remove_scene(&self, event_id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:OCCURRED_IN_SCENE]->(:Scene)
            DELETE r
            RETURN count(r) as deleted",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // INVOLVES Edge Methods
    // =========================================================================

    /// Add an involved character (creates INVOLVES edge with role)
    async fn add_involved_character(
        &self,
        event_id: StoryEventId,
        involved: InvolvedCharacter,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (c:Character {id: $character_id})
            MERGE (e)-[r:INVOLVES]->(c)
            SET r.role = $role
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", involved.character_id.to_string())
        .param("role", involved.role);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get all involved characters for an event
    async fn get_involved_characters(
        &self,
        event_id: StoryEventId,
    ) -> Result<Vec<InvolvedCharacter>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:INVOLVES]->(c:Character)
            RETURN c.id as character_id, r.role as role",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut involved = Vec::new();

        while let Some(row) = result.next().await? {
            let char_id_str: String = row.get("character_id")?;
            let role: String = row.get("role").unwrap_or_else(|_| "Actor".to_string());
            involved.push(InvolvedCharacter {
                character_id: CharacterId::from(Uuid::parse_str(&char_id_str)?),
                role,
            });
        }

        Ok(involved)
    }

    /// Remove an involved character (deletes INVOLVES edge)
    async fn remove_involved_character(
        &self,
        event_id: StoryEventId,
        character_id: CharacterId,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:INVOLVES]->(c:Character {id: $character_id})
            DELETE r
            RETURN count(r) as deleted",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // TRIGGERED_BY_NARRATIVE Edge Methods
    // =========================================================================

    /// Set the narrative event that triggered this story event
    async fn set_triggered_by(
        &self,
        event_id: StoryEventId,
        narrative_event_id: NarrativeEventId,
    ) -> Result<bool> {
        // First remove any existing triggered_by edge
        let remove_q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:TRIGGERED_BY_NARRATIVE]->(:NarrativeEvent)
            DELETE r",
        )
        .param("event_id", event_id.to_string());
        let _ = self.connection.graph().run(remove_q).await;

        // Create new edge
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (n:NarrativeEvent {id: $narrative_event_id})
            CREATE (e)-[:TRIGGERED_BY_NARRATIVE]->(n)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("narrative_event_id", narrative_event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the narrative event that triggered this story event
    async fn get_triggered_by(&self, event_id: StoryEventId) -> Result<Option<NarrativeEventId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:TRIGGERED_BY_NARRATIVE]->(n:NarrativeEvent)
            RETURN n.id as narrative_event_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let ne_id_str: String = row.get("narrative_event_id")?;
            Ok(Some(NarrativeEventId::from(Uuid::parse_str(&ne_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove the triggered_by association
    async fn remove_triggered_by(&self, event_id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:TRIGGERED_BY_NARRATIVE]->(:NarrativeEvent)
            DELETE r
            RETURN count(r) as deleted",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // RECORDS_CHALLENGE Edge Methods
    // =========================================================================

    /// Set the challenge this event records (creates RECORDS_CHALLENGE edge)
    async fn set_recorded_challenge(
        &self,
        event_id: StoryEventId,
        challenge_id: ChallengeId,
    ) -> Result<bool> {
        // First remove any existing recorded_challenge edge
        let remove_q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:RECORDS_CHALLENGE]->(:Challenge)
            DELETE r",
        )
        .param("event_id", event_id.to_string());
        let _ = self.connection.graph().run(remove_q).await;

        // Create new edge
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (c:Challenge {id: $challenge_id})
            CREATE (e)-[:RECORDS_CHALLENGE]->(c)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the challenge this event records
    async fn get_recorded_challenge(&self, event_id: StoryEventId) -> Result<Option<ChallengeId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:RECORDS_CHALLENGE]->(c:Challenge)
            RETURN c.id as challenge_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let challenge_id_str: String = row.get("challenge_id")?;
            Ok(Some(ChallengeId::from(Uuid::parse_str(&challenge_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove the recorded challenge association
    async fn remove_recorded_challenge(&self, event_id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:RECORDS_CHALLENGE]->(:Challenge)
            DELETE r
            RETURN count(r) as deleted",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // Query Methods by Edge Relationships
    // =========================================================================

    /// List events triggered by a specific narrative event
    async fn list_by_narrative_event(
        &self,
        narrative_event_id: NarrativeEventId,
    ) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:TRIGGERED_BY_NARRATIVE]->(n:NarrativeEvent {id: $narrative_event_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("narrative_event_id", narrative_event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events recording a specific challenge
    async fn list_by_challenge(&self, challenge_id: ChallengeId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:RECORDS_CHALLENGE]->(c:Challenge {id: $challenge_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events that occurred in a specific scene (via OCCURRED_IN_SCENE edge)
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:OCCURRED_IN_SCENE]->(s:Scene {id: $scene_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }
}

/// Convert a Neo4j row to a StoryEvent
///
/// NOTE: session_id, scene_id, location_id, involved_characters, and triggered_by
/// are now stored as graph edges, not node properties. Use the edge query methods
/// to retrieve these associations.
fn row_to_story_event(row: Row) -> Result<StoryEvent> {
    let node: neo4rs::Node = row.get("e")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let event_type_json: String = node.get("event_type_json")?;
    let timestamp_str: String = node.get("timestamp")?;
    let game_time: String = node.get("game_time").unwrap_or_default();
    let summary: String = node.get("summary")?;
    let is_hidden: bool = node.get("is_hidden").unwrap_or(false);
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());

    // Deserialize to stored type, then convert to domain type
    let stored_event_type: StoredStoryEventType = serde_json::from_str(&event_type_json)?;
    let event_type: StoryEventType = stored_event_type.into();
    let tags: Vec<String> = serde_json::from_str(&tags_json)?;

    Ok(StoryEvent {
        id: StoryEventId::from(Uuid::parse_str(&id_str)?),
        world_id: WorldId::from(Uuid::parse_str(&world_id_str)?),
        // NOTE: session_id now stored as OCCURRED_IN_SESSION edge
        // NOTE: scene_id now stored as OCCURRED_IN_SCENE edge
        // NOTE: location_id now stored as OCCURRED_AT edge
        event_type,
        timestamp: DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Utc),
        game_time: if game_time.is_empty() {
            None
        } else {
            Some(game_time)
        },
        summary,
        // NOTE: involved_characters now stored as INVOLVES edges
        is_hidden,
        tags,
        // NOTE: triggered_by now stored as TRIGGERED_BY_NARRATIVE edge
    })
}
