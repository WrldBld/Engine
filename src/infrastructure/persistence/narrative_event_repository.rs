//! NarrativeEvent repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::NarrativeEventRepositoryPort;
use crate::domain::entities::{
    ChainedEvent, EventChainMembership, EventEffect, EventOutcome, FeaturedNpc, NarrativeEvent,
    NarrativeTrigger, NarrativeTriggerType, OutcomeCondition, TriggerLogic,
};
use crate::domain::value_objects::{
    ActId, ChallengeId, CharacterId, EventChainId, LocationId, NarrativeEventId, SceneId, WorldId,
};

// ============================================================================
// Storage DTOs for NarrativeTrigger and EventOutcome
// These types have serde derives for JSON persistence in Neo4j
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredNarrativeTrigger {
    trigger_type: StoredNarrativeTriggerType,
    description: String,
    is_required: bool,
    trigger_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredNarrativeTriggerType {
    NpcAction {
        npc_id: String,
        npc_name: String,
        action_keywords: Vec<String>,
        action_description: String,
    },
    PlayerEntersLocation {
        location_id: String,
        location_name: String,
    },
    TimeAtLocation {
        location_id: String,
        location_name: String,
        time_context: String,
    },
    DialogueTopic {
        keywords: Vec<String>,
        with_npc: Option<String>,
        npc_name: Option<String>,
    },
    ChallengeCompleted {
        challenge_id: String,
        challenge_name: String,
        requires_success: Option<bool>,
    },
    RelationshipThreshold {
        character_id: String,
        character_name: String,
        with_character: String,
        with_character_name: String,
        min_sentiment: Option<f32>,
        max_sentiment: Option<f32>,
    },
    HasItem {
        item_name: String,
        quantity: Option<u32>,
    },
    MissingItem {
        item_name: String,
    },
    EventCompleted {
        event_id: String,
        event_name: String,
        outcome_name: Option<String>,
    },
    TurnCount {
        turns: u32,
        since_event: Option<String>,
    },
    FlagSet {
        flag_name: String,
    },
    FlagNotSet {
        flag_name: String,
    },
    StatThreshold {
        character_id: String,
        stat_name: String,
        min_value: Option<i32>,
        max_value: Option<i32>,
    },
    CombatResult {
        victory: Option<bool>,
        involved_npc: Option<String>,
    },
    Custom {
        description: String,
        llm_evaluation: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredEventOutcome {
    name: String,
    label: String,
    description: String,
    condition: Option<StoredOutcomeCondition>,
    effects: Vec<StoredEventEffect>,
    chain_events: Vec<StoredChainedEvent>,
    timeline_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredOutcomeCondition {
    DmChoice,
    ChallengeResult {
        challenge_id: Option<String>,
        success_required: bool,
    },
    CombatResult {
        victory_required: bool,
    },
    DialogueChoice {
        keywords: Vec<String>,
    },
    PlayerAction {
        action_keywords: Vec<String>,
    },
    HasItem {
        item_name: String,
    },
    Custom {
        description: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredEventEffect {
    ModifyRelationship {
        from_character: String,
        from_name: String,
        to_character: String,
        to_name: String,
        sentiment_change: f32,
        reason: String,
    },
    GiveItem {
        item_name: String,
        item_description: Option<String>,
        quantity: u32,
    },
    TakeItem {
        item_name: String,
        quantity: u32,
    },
    RevealInformation {
        info_type: String,
        title: String,
        content: String,
        persist_to_journal: bool,
    },
    SetFlag {
        flag_name: String,
        value: bool,
    },
    EnableChallenge {
        challenge_id: String,
        challenge_name: String,
    },
    DisableChallenge {
        challenge_id: String,
        challenge_name: String,
    },
    EnableEvent {
        event_id: String,
        event_name: String,
    },
    DisableEvent {
        event_id: String,
        event_name: String,
    },
    TriggerScene {
        scene_id: String,
        scene_name: String,
    },
    StartCombat {
        participants: Vec<String>,
        participant_names: Vec<String>,
        combat_description: String,
    },
    ModifyStat {
        character_id: String,
        character_name: String,
        stat_name: String,
        modifier: i32,
    },
    AddReward {
        reward_type: String,
        amount: i32,
        description: String,
    },
    Custom {
        description: String,
        requires_dm_action: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredChainedEvent {
    event_id: String,
    event_name: String,
    delay_turns: u32,
    additional_trigger: Option<Box<StoredNarrativeTriggerType>>,
    chain_reason: Option<String>,
}

// Conversion from domain to stored types
impl From<&NarrativeTrigger> for StoredNarrativeTrigger {
    fn from(t: &NarrativeTrigger) -> Self {
        Self {
            trigger_type: StoredNarrativeTriggerType::from(&t.trigger_type),
            description: t.description.clone(),
            is_required: t.is_required,
            trigger_id: t.trigger_id.clone(),
        }
    }
}

impl From<&NarrativeTriggerType> for StoredNarrativeTriggerType {
    fn from(t: &NarrativeTriggerType) -> Self {
        match t {
            NarrativeTriggerType::NpcAction { npc_id, npc_name, action_keywords, action_description } => {
                StoredNarrativeTriggerType::NpcAction {
                    npc_id: npc_id.to_string(),
                    npc_name: npc_name.clone(),
                    action_keywords: action_keywords.clone(),
                    action_description: action_description.clone(),
                }
            }
            NarrativeTriggerType::PlayerEntersLocation { location_id, location_name } => {
                StoredNarrativeTriggerType::PlayerEntersLocation {
                    location_id: location_id.to_string(),
                    location_name: location_name.clone(),
                }
            }
            NarrativeTriggerType::TimeAtLocation { location_id, location_name, time_context } => {
                StoredNarrativeTriggerType::TimeAtLocation {
                    location_id: location_id.to_string(),
                    location_name: location_name.clone(),
                    time_context: time_context.clone(),
                }
            }
            NarrativeTriggerType::DialogueTopic { keywords, with_npc, npc_name } => {
                StoredNarrativeTriggerType::DialogueTopic {
                    keywords: keywords.clone(),
                    with_npc: with_npc.as_ref().map(|id| id.to_string()),
                    npc_name: npc_name.clone(),
                }
            }
            NarrativeTriggerType::ChallengeCompleted { challenge_id, challenge_name, requires_success } => {
                StoredNarrativeTriggerType::ChallengeCompleted {
                    challenge_id: challenge_id.to_string(),
                    challenge_name: challenge_name.clone(),
                    requires_success: *requires_success,
                }
            }
            NarrativeTriggerType::RelationshipThreshold { character_id, character_name, with_character, with_character_name, min_sentiment, max_sentiment } => {
                StoredNarrativeTriggerType::RelationshipThreshold {
                    character_id: character_id.to_string(),
                    character_name: character_name.clone(),
                    with_character: with_character.to_string(),
                    with_character_name: with_character_name.clone(),
                    min_sentiment: *min_sentiment,
                    max_sentiment: *max_sentiment,
                }
            }
            NarrativeTriggerType::HasItem { item_name, quantity } => {
                StoredNarrativeTriggerType::HasItem {
                    item_name: item_name.clone(),
                    quantity: *quantity,
                }
            }
            NarrativeTriggerType::MissingItem { item_name } => {
                StoredNarrativeTriggerType::MissingItem {
                    item_name: item_name.clone(),
                }
            }
            NarrativeTriggerType::EventCompleted { event_id, event_name, outcome_name } => {
                StoredNarrativeTriggerType::EventCompleted {
                    event_id: event_id.to_string(),
                    event_name: event_name.clone(),
                    outcome_name: outcome_name.clone(),
                }
            }
            NarrativeTriggerType::TurnCount { turns, since_event } => {
                StoredNarrativeTriggerType::TurnCount {
                    turns: *turns,
                    since_event: since_event.as_ref().map(|id| id.to_string()),
                }
            }
            NarrativeTriggerType::FlagSet { flag_name } => {
                StoredNarrativeTriggerType::FlagSet {
                    flag_name: flag_name.clone(),
                }
            }
            NarrativeTriggerType::FlagNotSet { flag_name } => {
                StoredNarrativeTriggerType::FlagNotSet {
                    flag_name: flag_name.clone(),
                }
            }
            NarrativeTriggerType::StatThreshold { character_id, stat_name, min_value, max_value } => {
                StoredNarrativeTriggerType::StatThreshold {
                    character_id: character_id.to_string(),
                    stat_name: stat_name.clone(),
                    min_value: *min_value,
                    max_value: *max_value,
                }
            }
            NarrativeTriggerType::CombatResult { victory, involved_npc } => {
                StoredNarrativeTriggerType::CombatResult {
                    victory: *victory,
                    involved_npc: involved_npc.as_ref().map(|id| id.to_string()),
                }
            }
            NarrativeTriggerType::Custom { description, llm_evaluation } => {
                StoredNarrativeTriggerType::Custom {
                    description: description.clone(),
                    llm_evaluation: *llm_evaluation,
                }
            }
        }
    }
}

impl From<&EventOutcome> for StoredEventOutcome {
    fn from(o: &EventOutcome) -> Self {
        Self {
            name: o.name.clone(),
            label: o.label.clone(),
            description: o.description.clone(),
            condition: o.condition.as_ref().map(|c| StoredOutcomeCondition::from(c)),
            effects: o.effects.iter().map(|e| StoredEventEffect::from(e)).collect(),
            chain_events: o.chain_events.iter().map(|c| StoredChainedEvent::from(c)).collect(),
            timeline_summary: o.timeline_summary.clone(),
        }
    }
}

impl From<&OutcomeCondition> for StoredOutcomeCondition {
    fn from(c: &OutcomeCondition) -> Self {
        match c {
            OutcomeCondition::DmChoice => StoredOutcomeCondition::DmChoice,
            OutcomeCondition::ChallengeResult { challenge_id, success_required } => {
                StoredOutcomeCondition::ChallengeResult {
                    challenge_id: challenge_id.as_ref().map(|id| id.to_string()),
                    success_required: *success_required,
                }
            }
            OutcomeCondition::CombatResult { victory_required } => {
                StoredOutcomeCondition::CombatResult {
                    victory_required: *victory_required,
                }
            }
            OutcomeCondition::DialogueChoice { keywords } => {
                StoredOutcomeCondition::DialogueChoice {
                    keywords: keywords.clone(),
                }
            }
            OutcomeCondition::PlayerAction { action_keywords } => {
                StoredOutcomeCondition::PlayerAction {
                    action_keywords: action_keywords.clone(),
                }
            }
            OutcomeCondition::HasItem { item_name } => {
                StoredOutcomeCondition::HasItem {
                    item_name: item_name.clone(),
                }
            }
            OutcomeCondition::Custom { description } => {
                StoredOutcomeCondition::Custom {
                    description: description.clone(),
                }
            }
        }
    }
}

impl From<&EventEffect> for StoredEventEffect {
    fn from(e: &EventEffect) -> Self {
        match e {
            EventEffect::ModifyRelationship { from_character, from_name, to_character, to_name, sentiment_change, reason } => {
                StoredEventEffect::ModifyRelationship {
                    from_character: from_character.to_string(),
                    from_name: from_name.clone(),
                    to_character: to_character.to_string(),
                    to_name: to_name.clone(),
                    sentiment_change: *sentiment_change,
                    reason: reason.clone(),
                }
            }
            EventEffect::GiveItem { item_name, item_description, quantity } => {
                StoredEventEffect::GiveItem {
                    item_name: item_name.clone(),
                    item_description: item_description.clone(),
                    quantity: *quantity,
                }
            }
            EventEffect::TakeItem { item_name, quantity } => {
                StoredEventEffect::TakeItem {
                    item_name: item_name.clone(),
                    quantity: *quantity,
                }
            }
            EventEffect::RevealInformation { info_type, title, content, persist_to_journal } => {
                StoredEventEffect::RevealInformation {
                    info_type: info_type.clone(),
                    title: title.clone(),
                    content: content.clone(),
                    persist_to_journal: *persist_to_journal,
                }
            }
            EventEffect::SetFlag { flag_name, value } => {
                StoredEventEffect::SetFlag {
                    flag_name: flag_name.clone(),
                    value: *value,
                }
            }
            EventEffect::EnableChallenge { challenge_id, challenge_name } => {
                StoredEventEffect::EnableChallenge {
                    challenge_id: challenge_id.to_string(),
                    challenge_name: challenge_name.clone(),
                }
            }
            EventEffect::DisableChallenge { challenge_id, challenge_name } => {
                StoredEventEffect::DisableChallenge {
                    challenge_id: challenge_id.to_string(),
                    challenge_name: challenge_name.clone(),
                }
            }
            EventEffect::EnableEvent { event_id, event_name } => {
                StoredEventEffect::EnableEvent {
                    event_id: event_id.to_string(),
                    event_name: event_name.clone(),
                }
            }
            EventEffect::DisableEvent { event_id, event_name } => {
                StoredEventEffect::DisableEvent {
                    event_id: event_id.to_string(),
                    event_name: event_name.clone(),
                }
            }
            EventEffect::TriggerScene { scene_id, scene_name } => {
                StoredEventEffect::TriggerScene {
                    scene_id: scene_id.to_string(),
                    scene_name: scene_name.clone(),
                }
            }
            EventEffect::StartCombat { participants, participant_names, combat_description } => {
                StoredEventEffect::StartCombat {
                    participants: participants.iter().map(|id| id.to_string()).collect(),
                    participant_names: participant_names.clone(),
                    combat_description: combat_description.clone(),
                }
            }
            EventEffect::ModifyStat { character_id, character_name, stat_name, modifier } => {
                StoredEventEffect::ModifyStat {
                    character_id: character_id.to_string(),
                    character_name: character_name.clone(),
                    stat_name: stat_name.clone(),
                    modifier: *modifier,
                }
            }
            EventEffect::AddReward { reward_type, amount, description } => {
                StoredEventEffect::AddReward {
                    reward_type: reward_type.clone(),
                    amount: *amount,
                    description: description.clone(),
                }
            }
            EventEffect::Custom { description, requires_dm_action } => {
                StoredEventEffect::Custom {
                    description: description.clone(),
                    requires_dm_action: *requires_dm_action,
                }
            }
        }
    }
}

impl From<&ChainedEvent> for StoredChainedEvent {
    fn from(c: &ChainedEvent) -> Self {
        Self {
            event_id: c.event_id.to_string(),
            event_name: c.event_name.clone(),
            delay_turns: c.delay_turns,
            additional_trigger: c.additional_trigger.as_ref().map(|t| Box::new(StoredNarrativeTriggerType::from(t))),
            chain_reason: c.chain_reason.clone(),
        }
    }
}

// Conversion from stored to domain types (deserialization)
impl From<StoredNarrativeTrigger> for NarrativeTrigger {
    fn from(s: StoredNarrativeTrigger) -> Self {
        Self {
            trigger_type: NarrativeTriggerType::from(s.trigger_type),
            description: s.description,
            is_required: s.is_required,
            trigger_id: s.trigger_id,
        }
    }
}

impl From<StoredNarrativeTriggerType> for NarrativeTriggerType {
    fn from(s: StoredNarrativeTriggerType) -> Self {
        match s {
            StoredNarrativeTriggerType::NpcAction { npc_id, npc_name, action_keywords, action_description } => {
                NarrativeTriggerType::NpcAction {
                    npc_id: CharacterId::from(Uuid::parse_str(&npc_id).unwrap_or_default()),
                    npc_name,
                    action_keywords,
                    action_description,
                }
            }
            StoredNarrativeTriggerType::PlayerEntersLocation { location_id, location_name } => {
                NarrativeTriggerType::PlayerEntersLocation {
                    location_id: LocationId::from(Uuid::parse_str(&location_id).unwrap_or_default()),
                    location_name,
                }
            }
            StoredNarrativeTriggerType::TimeAtLocation { location_id, location_name, time_context } => {
                NarrativeTriggerType::TimeAtLocation {
                    location_id: LocationId::from(Uuid::parse_str(&location_id).unwrap_or_default()),
                    location_name,
                    time_context,
                }
            }
            StoredNarrativeTriggerType::DialogueTopic { keywords, with_npc, npc_name } => {
                NarrativeTriggerType::DialogueTopic {
                    keywords,
                    with_npc: with_npc.and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                    npc_name,
                }
            }
            StoredNarrativeTriggerType::ChallengeCompleted { challenge_id, challenge_name, requires_success } => {
                NarrativeTriggerType::ChallengeCompleted {
                    challenge_id: ChallengeId::from(Uuid::parse_str(&challenge_id).unwrap_or_default()),
                    challenge_name,
                    requires_success,
                }
            }
            StoredNarrativeTriggerType::RelationshipThreshold { character_id, character_name, with_character, with_character_name, min_sentiment, max_sentiment } => {
                NarrativeTriggerType::RelationshipThreshold {
                    character_id: CharacterId::from(Uuid::parse_str(&character_id).unwrap_or_default()),
                    character_name,
                    with_character: CharacterId::from(Uuid::parse_str(&with_character).unwrap_or_default()),
                    with_character_name,
                    min_sentiment,
                    max_sentiment,
                }
            }
            StoredNarrativeTriggerType::HasItem { item_name, quantity } => {
                NarrativeTriggerType::HasItem { item_name, quantity }
            }
            StoredNarrativeTriggerType::MissingItem { item_name } => {
                NarrativeTriggerType::MissingItem { item_name }
            }
            StoredNarrativeTriggerType::EventCompleted { event_id, event_name, outcome_name } => {
                NarrativeTriggerType::EventCompleted {
                    event_id: NarrativeEventId::from(Uuid::parse_str(&event_id).unwrap_or_default()),
                    event_name,
                    outcome_name,
                }
            }
            StoredNarrativeTriggerType::TurnCount { turns, since_event } => {
                NarrativeTriggerType::TurnCount {
                    turns,
                    since_event: since_event.and_then(|id| Uuid::parse_str(&id).ok().map(NarrativeEventId::from)),
                }
            }
            StoredNarrativeTriggerType::FlagSet { flag_name } => {
                NarrativeTriggerType::FlagSet { flag_name }
            }
            StoredNarrativeTriggerType::FlagNotSet { flag_name } => {
                NarrativeTriggerType::FlagNotSet { flag_name }
            }
            StoredNarrativeTriggerType::StatThreshold { character_id, stat_name, min_value, max_value } => {
                NarrativeTriggerType::StatThreshold {
                    character_id: CharacterId::from(Uuid::parse_str(&character_id).unwrap_or_default()),
                    stat_name,
                    min_value,
                    max_value,
                }
            }
            StoredNarrativeTriggerType::CombatResult { victory, involved_npc } => {
                NarrativeTriggerType::CombatResult {
                    victory,
                    involved_npc: involved_npc.and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                }
            }
            StoredNarrativeTriggerType::Custom { description, llm_evaluation } => {
                NarrativeTriggerType::Custom { description, llm_evaluation }
            }
        }
    }
}

impl From<StoredEventOutcome> for EventOutcome {
    fn from(s: StoredEventOutcome) -> Self {
        Self {
            name: s.name,
            label: s.label,
            description: s.description,
            condition: s.condition.map(|c| OutcomeCondition::from(c)),
            effects: s.effects.into_iter().map(|e| EventEffect::from(e)).collect(),
            chain_events: s.chain_events.into_iter().map(|c| ChainedEvent::from(c)).collect(),
            timeline_summary: s.timeline_summary,
        }
    }
}

impl From<StoredOutcomeCondition> for OutcomeCondition {
    fn from(s: StoredOutcomeCondition) -> Self {
        match s {
            StoredOutcomeCondition::DmChoice => OutcomeCondition::DmChoice,
            StoredOutcomeCondition::ChallengeResult { challenge_id, success_required } => {
                OutcomeCondition::ChallengeResult {
                    challenge_id: challenge_id.and_then(|id| Uuid::parse_str(&id).ok().map(ChallengeId::from)),
                    success_required,
                }
            }
            StoredOutcomeCondition::CombatResult { victory_required } => {
                OutcomeCondition::CombatResult { victory_required }
            }
            StoredOutcomeCondition::DialogueChoice { keywords } => {
                OutcomeCondition::DialogueChoice { keywords }
            }
            StoredOutcomeCondition::PlayerAction { action_keywords } => {
                OutcomeCondition::PlayerAction { action_keywords }
            }
            StoredOutcomeCondition::HasItem { item_name } => {
                OutcomeCondition::HasItem { item_name }
            }
            StoredOutcomeCondition::Custom { description } => {
                OutcomeCondition::Custom { description }
            }
        }
    }
}

impl From<StoredEventEffect> for EventEffect {
    fn from(s: StoredEventEffect) -> Self {
        match s {
            StoredEventEffect::ModifyRelationship { from_character, from_name, to_character, to_name, sentiment_change, reason } => {
                EventEffect::ModifyRelationship {
                    from_character: CharacterId::from(Uuid::parse_str(&from_character).unwrap_or_default()),
                    from_name,
                    to_character: CharacterId::from(Uuid::parse_str(&to_character).unwrap_or_default()),
                    to_name,
                    sentiment_change,
                    reason,
                }
            }
            StoredEventEffect::GiveItem { item_name, item_description, quantity } => {
                EventEffect::GiveItem { item_name, item_description, quantity }
            }
            StoredEventEffect::TakeItem { item_name, quantity } => {
                EventEffect::TakeItem { item_name, quantity }
            }
            StoredEventEffect::RevealInformation { info_type, title, content, persist_to_journal } => {
                EventEffect::RevealInformation { info_type, title, content, persist_to_journal }
            }
            StoredEventEffect::SetFlag { flag_name, value } => {
                EventEffect::SetFlag { flag_name, value }
            }
            StoredEventEffect::EnableChallenge { challenge_id, challenge_name } => {
                EventEffect::EnableChallenge {
                    challenge_id: ChallengeId::from(Uuid::parse_str(&challenge_id).unwrap_or_default()),
                    challenge_name,
                }
            }
            StoredEventEffect::DisableChallenge { challenge_id, challenge_name } => {
                EventEffect::DisableChallenge {
                    challenge_id: ChallengeId::from(Uuid::parse_str(&challenge_id).unwrap_or_default()),
                    challenge_name,
                }
            }
            StoredEventEffect::EnableEvent { event_id, event_name } => {
                EventEffect::EnableEvent {
                    event_id: NarrativeEventId::from(Uuid::parse_str(&event_id).unwrap_or_default()),
                    event_name,
                }
            }
            StoredEventEffect::DisableEvent { event_id, event_name } => {
                EventEffect::DisableEvent {
                    event_id: NarrativeEventId::from(Uuid::parse_str(&event_id).unwrap_or_default()),
                    event_name,
                }
            }
            StoredEventEffect::TriggerScene { scene_id, scene_name } => {
                EventEffect::TriggerScene {
                    scene_id: SceneId::from(Uuid::parse_str(&scene_id).unwrap_or_default()),
                    scene_name,
                }
            }
            StoredEventEffect::StartCombat { participants, participant_names, combat_description } => {
                EventEffect::StartCombat {
                    participants: participants.into_iter().filter_map(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)).collect(),
                    participant_names,
                    combat_description,
                }
            }
            StoredEventEffect::ModifyStat { character_id, character_name, stat_name, modifier } => {
                EventEffect::ModifyStat {
                    character_id: CharacterId::from(Uuid::parse_str(&character_id).unwrap_or_default()),
                    character_name,
                    stat_name,
                    modifier,
                }
            }
            StoredEventEffect::AddReward { reward_type, amount, description } => {
                EventEffect::AddReward { reward_type, amount, description }
            }
            StoredEventEffect::Custom { description, requires_dm_action } => {
                EventEffect::Custom { description, requires_dm_action }
            }
        }
    }
}

impl From<StoredChainedEvent> for ChainedEvent {
    fn from(s: StoredChainedEvent) -> Self {
        Self {
            event_id: NarrativeEventId::from(Uuid::parse_str(&s.event_id).unwrap_or_default()),
            event_name: s.event_name,
            delay_turns: s.delay_turns,
            additional_trigger: s.additional_trigger.map(|t| NarrativeTriggerType::from(*t)),
            chain_reason: s.chain_reason,
        }
    }
}

/// Repository for NarrativeEvent operations
pub struct Neo4jNarrativeEventRepository {
    connection: Neo4jConnection,
}

impl Neo4jNarrativeEventRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new narrative event
    ///
    /// NOTE: This only creates the node. Scene/location/act associations and featured NPCs
    /// are now stored as graph edges and must be created separately using the edge methods:
    /// - `tie_to_scene()` for TIED_TO_SCENE edge
    /// - `tie_to_location()` for TIED_TO_LOCATION edge
    /// - `assign_to_act()` for BELONGS_TO_ACT edge
    /// - `add_featured_npc()` for FEATURES_NPC edges
    /// - Chain membership is managed via EventChainRepositoryPort
    pub async fn create(&self, event: &NarrativeEvent) -> Result<()> {
        let stored_triggers: Vec<StoredNarrativeTrigger> =
            event.trigger_conditions.iter().map(|t| t.into()).collect();
        let triggers_json = serde_json::to_string(&stored_triggers)?;
        let stored_outcomes: Vec<StoredEventOutcome> =
            event.outcomes.iter().map(|o| o.into()).collect();
        let outcomes_json = serde_json::to_string(&stored_outcomes)?;
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (e:NarrativeEvent {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                tags_json: $tags_json,
                triggers_json: $triggers_json,
                trigger_logic: $trigger_logic,
                scene_direction: $scene_direction,
                suggested_opening: $suggested_opening,
                outcomes_json: $outcomes_json,
                default_outcome: $default_outcome,
                is_active: $is_active,
                is_triggered: $is_triggered,
                triggered_at: $triggered_at,
                selected_outcome: $selected_outcome,
                is_repeatable: $is_repeatable,
                trigger_count: $trigger_count,
                delay_turns: $delay_turns,
                expires_after_turns: $expires_after_turns,
                priority: $priority,
                is_favorite: $is_favorite,
                created_at: $created_at,
                updated_at: $updated_at
            })
            CREATE (w)-[:HAS_NARRATIVE_EVENT]->(e)
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("world_id", event.world_id.to_string())
        .param("name", event.name.clone())
        .param("description", event.description.clone())
        .param("tags_json", tags_json)
        .param("triggers_json", triggers_json)
        .param("trigger_logic", format!("{:?}", event.trigger_logic))
        .param("scene_direction", event.scene_direction.clone())
        .param(
            "suggested_opening",
            event.suggested_opening.clone().unwrap_or_default(),
        )
        .param("outcomes_json", outcomes_json)
        .param(
            "default_outcome",
            event.default_outcome.clone().unwrap_or_default(),
        )
        .param("is_active", event.is_active)
        .param("is_triggered", event.is_triggered)
        .param(
            "triggered_at",
            event
                .triggered_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        )
        .param(
            "selected_outcome",
            event.selected_outcome.clone().unwrap_or_default(),
        )
        .param("is_repeatable", event.is_repeatable)
        .param("trigger_count", event.trigger_count as i64)
        .param("delay_turns", event.delay_turns as i64)
        .param(
            "expires_after_turns",
            event.expires_after_turns.map(|t| t as i64).unwrap_or(-1),
        )
        .param("priority", event.priority as i64)
        .param("is_favorite", event.is_favorite)
        .param("created_at", event.created_at.to_rfc3339())
        .param("updated_at", event.updated_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Created narrative event: {}", event.name);

        Ok(())
    }

    /// Get a narrative event by ID
    pub async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            RETURN e",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_narrative_event(row)?))
        } else {
            Ok(None)
        }
    }

    /// Update a narrative event
    ///
    /// NOTE: This only updates node properties. Scene/location/act associations and featured NPCs
    /// are now stored as graph edges and must be managed separately using the edge methods.
    pub async fn update(&self, event: &NarrativeEvent) -> Result<bool> {
        let stored_triggers: Vec<StoredNarrativeTrigger> =
            event.trigger_conditions.iter().map(|t| t.into()).collect();
        let triggers_json = serde_json::to_string(&stored_triggers)?;
        let stored_outcomes: Vec<StoredEventOutcome> =
            event.outcomes.iter().map(|o| o.into()).collect();
        let outcomes_json = serde_json::to_string(&stored_outcomes)?;
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.name = $name,
                e.description = $description,
                e.tags_json = $tags_json,
                e.triggers_json = $triggers_json,
                e.trigger_logic = $trigger_logic,
                e.scene_direction = $scene_direction,
                e.suggested_opening = $suggested_opening,
                e.outcomes_json = $outcomes_json,
                e.default_outcome = $default_outcome,
                e.is_active = $is_active,
                e.is_triggered = $is_triggered,
                e.triggered_at = $triggered_at,
                e.selected_outcome = $selected_outcome,
                e.is_repeatable = $is_repeatable,
                e.trigger_count = $trigger_count,
                e.delay_turns = $delay_turns,
                e.expires_after_turns = $expires_after_turns,
                e.priority = $priority,
                e.is_favorite = $is_favorite,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("name", event.name.clone())
        .param("description", event.description.clone())
        .param("tags_json", tags_json)
        .param("triggers_json", triggers_json)
        .param("trigger_logic", format!("{:?}", event.trigger_logic))
        .param("scene_direction", event.scene_direction.clone())
        .param(
            "suggested_opening",
            event.suggested_opening.clone().unwrap_or_default(),
        )
        .param("outcomes_json", outcomes_json)
        .param(
            "default_outcome",
            event.default_outcome.clone().unwrap_or_default(),
        )
        .param("is_active", event.is_active)
        .param("is_triggered", event.is_triggered)
        .param(
            "triggered_at",
            event
                .triggered_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        )
        .param(
            "selected_outcome",
            event.selected_outcome.clone().unwrap_or_default(),
        )
        .param("is_repeatable", event.is_repeatable)
        .param("trigger_count", event.trigger_count as i64)
        .param("delay_turns", event.delay_turns as i64)
        .param(
            "expires_after_turns",
            event.expires_after_turns.map(|t| t as i64).unwrap_or(-1),
        )
        .param("priority", event.priority as i64)
        .param("is_favorite", event.is_favorite)
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// List all narrative events for a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            RETURN e
            ORDER BY e.is_favorite DESC, e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List active narrative events for a world
    pub async fn list_active(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_active = true
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List favorite narrative events for a world
    pub async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_favorite = true
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List untriggered active events (for LLM context)
    pub async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_active = true AND e.is_triggered = false
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// Toggle favorite status
    pub async fn toggle_favorite(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_favorite = NOT e.is_favorite,
                e.updated_at = $updated_at
            RETURN e.is_favorite as is_favorite",
        )
        .param("id", id.to_string())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let is_favorite: bool = row.get("is_favorite")?;
            Ok(is_favorite)
        } else {
            Ok(false)
        }
    }

    /// Set active status
    pub async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_active = $is_active,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("is_active", is_active)
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Mark event as triggered
    pub async fn mark_triggered(
        &self,
        id: NarrativeEventId,
        outcome_name: Option<String>,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_triggered = true,
                e.triggered_at = $triggered_at,
                e.selected_outcome = $selected_outcome,
                e.trigger_count = e.trigger_count + 1,
                e.is_active = CASE WHEN e.is_repeatable THEN e.is_active ELSE false END,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("triggered_at", Utc::now().to_rfc3339())
        .param("selected_outcome", outcome_name.unwrap_or_default())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Reset triggered status (for repeatable events)
    pub async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_triggered = false,
                e.triggered_at = null,
                e.selected_outcome = null,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Delete a narrative event
    pub async fn delete(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
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

    // =========================================================================
    // TIED_TO_SCENE Edge Methods
    // =========================================================================

    /// Tie event to a scene (creates TIED_TO_SCENE edge)
    pub async fn tie_to_scene(
        &self,
        event_id: NarrativeEventId,
        scene_id: SceneId,
    ) -> Result<bool> {
        // First remove any existing scene tie, then create the new one
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})
            OPTIONAL MATCH (e)-[old:TIED_TO_SCENE]->()
            DELETE old
            WITH e
            MATCH (s:Scene {id: $scene_id})
            CREATE (e)-[:TIED_TO_SCENE]->(s)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the scene this event is tied to (if any)
    pub async fn get_tied_scene(&self, event_id: NarrativeEventId) -> Result<Option<SceneId>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[:TIED_TO_SCENE]->(s:Scene)
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

    /// Remove scene tie (deletes TIED_TO_SCENE edge)
    pub async fn untie_from_scene(&self, event_id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:TIED_TO_SCENE]->()
            DELETE r
            RETURN count(*) as deleted",
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
    // TIED_TO_LOCATION Edge Methods
    // =========================================================================

    /// Tie event to a location (creates TIED_TO_LOCATION edge)
    pub async fn tie_to_location(
        &self,
        event_id: NarrativeEventId,
        location_id: LocationId,
    ) -> Result<bool> {
        // First remove any existing location tie, then create the new one
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})
            OPTIONAL MATCH (e)-[old:TIED_TO_LOCATION]->()
            DELETE old
            WITH e
            MATCH (l:Location {id: $location_id})
            CREATE (e)-[:TIED_TO_LOCATION]->(l)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the location this event is tied to (if any)
    pub async fn get_tied_location(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<Option<LocationId>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[:TIED_TO_LOCATION]->(l:Location)
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

    /// Remove location tie (deletes TIED_TO_LOCATION edge)
    pub async fn untie_from_location(&self, event_id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:TIED_TO_LOCATION]->()
            DELETE r
            RETURN count(*) as deleted",
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
    // BELONGS_TO_ACT Edge Methods
    // =========================================================================

    /// Assign event to an act (creates BELONGS_TO_ACT edge)
    pub async fn assign_to_act(&self, event_id: NarrativeEventId, act_id: ActId) -> Result<bool> {
        // First remove any existing act assignment, then create the new one
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})
            OPTIONAL MATCH (e)-[old:BELONGS_TO_ACT]->()
            DELETE old
            WITH e
            MATCH (a:Act {id: $act_id})
            CREATE (e)-[:BELONGS_TO_ACT]->(a)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("act_id", act_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the act this event belongs to (if any)
    pub async fn get_act(&self, event_id: NarrativeEventId) -> Result<Option<ActId>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[:BELONGS_TO_ACT]->(a:Act)
            RETURN a.id as act_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let act_id_str: String = row.get("act_id")?;
            Ok(Some(ActId::from(Uuid::parse_str(&act_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove act assignment (deletes BELONGS_TO_ACT edge)
    pub async fn unassign_from_act(&self, event_id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:BELONGS_TO_ACT]->()
            DELETE r
            RETURN count(*) as deleted",
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
    // FEATURES_NPC Edge Methods
    // =========================================================================

    /// Add a featured NPC to the event (creates FEATURES_NPC edge)
    pub async fn add_featured_npc(
        &self,
        event_id: NarrativeEventId,
        featured_npc: FeaturedNpc,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})
            MATCH (c:Character {id: $character_id})
            MERGE (e)-[r:FEATURES_NPC]->(c)
            SET r.role = $role
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", featured_npc.character_id.to_string())
        .param("role", featured_npc.role.unwrap_or_default());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get all featured NPCs for an event
    pub async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> Result<Vec<FeaturedNpc>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:FEATURES_NPC]->(c:Character)
            RETURN c.id as character_id, r.role as role
            ORDER BY c.name",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut npcs = Vec::new();

        while let Some(row) = result.next().await? {
            let character_id_str: String = row.get("character_id")?;
            let role: String = row.get("role").unwrap_or_default();

            npcs.push(FeaturedNpc {
                character_id: CharacterId::from(Uuid::parse_str(&character_id_str)?),
                role: if role.is_empty() { None } else { Some(role) },
            });
        }

        Ok(npcs)
    }

    /// Remove a featured NPC from the event (deletes FEATURES_NPC edge)
    pub async fn remove_featured_npc(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:FEATURES_NPC]->(c:Character {id: $character_id})
            DELETE r
            RETURN count(*) as deleted",
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

    /// Update featured NPC role
    pub async fn update_featured_npc_role(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
        role: Option<String>,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:FEATURES_NPC]->(c:Character {id: $character_id})
            SET r.role = $role
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", character_id.to_string())
        .param("role", role.unwrap_or_default());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    // =========================================================================
    // Chain Membership Query Methods
    // =========================================================================

    /// Get chain membership info for an event (queries CONTAINS_EVENT edges)
    pub async fn get_chain_memberships(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<Vec<EventChainMembership>> {
        let q = query(
            "MATCH (chain:EventChain)-[r:CONTAINS_EVENT]->(e:NarrativeEvent {id: $event_id})
            RETURN chain.id as chain_id, r.position as position, r.is_completed as is_completed
            ORDER BY chain.name",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut memberships = Vec::new();

        while let Some(row) = result.next().await? {
            let chain_id_str: String = row.get("chain_id")?;
            let position: i64 = row.get("position").unwrap_or(0);
            let is_completed: bool = row.get("is_completed").unwrap_or(false);

            memberships.push(EventChainMembership {
                chain_id: EventChainId::from(Uuid::parse_str(&chain_id_str)?),
                position: position as u32,
                is_completed,
            });
        }

        Ok(memberships)
    }

    // =========================================================================
    // Query Methods for Events by Edge Relationships
    // =========================================================================

    /// List events tied to a specific scene
    pub async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent)-[:TIED_TO_SCENE]->(s:Scene {id: $scene_id})
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List events tied to a specific location
    pub async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent)-[:TIED_TO_LOCATION]->(l:Location {id: $location_id})
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List events belonging to a specific act
    pub async fn list_by_act(&self, act_id: ActId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent)-[:BELONGS_TO_ACT]->(a:Act {id: $act_id})
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("act_id", act_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List events featuring a specific NPC
    pub async fn list_by_featured_npc(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent)-[:FEATURES_NPC]->(c:Character {id: $character_id})
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }
}

/// Convert a Neo4j row to a NarrativeEvent
///
/// NOTE: Scene/location/act associations and featured NPCs are now stored as graph edges
/// and must be fetched separately using the edge methods on the repository.
fn row_to_narrative_event(row: Row) -> Result<NarrativeEvent> {
    let node: neo4rs::Node = row.get("e")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());
    let triggers_json: String = node.get("triggers_json").unwrap_or_else(|_| "[]".to_string());
    let trigger_logic_str: String = node.get("trigger_logic").unwrap_or_else(|_| "All".to_string());
    let scene_direction: String = node.get("scene_direction").unwrap_or_default();
    let suggested_opening: String = node.get("suggested_opening").unwrap_or_default();
    // NOTE: featured_npcs moved to FEATURES_NPC edges
    let outcomes_json: String = node.get("outcomes_json").unwrap_or_else(|_| "[]".to_string());
    let default_outcome: String = node.get("default_outcome").unwrap_or_default();
    let is_active: bool = node.get("is_active").unwrap_or(true);
    let is_triggered: bool = node.get("is_triggered").unwrap_or(false);
    let triggered_at_str: String = node.get("triggered_at").unwrap_or_default();
    let selected_outcome: String = node.get("selected_outcome").unwrap_or_default();
    let is_repeatable: bool = node.get("is_repeatable").unwrap_or(false);
    let trigger_count: i64 = node.get("trigger_count").unwrap_or(0);
    let delay_turns: i64 = node.get("delay_turns").unwrap_or(0);
    let expires_after_turns: i64 = node.get("expires_after_turns").unwrap_or(-1);
    // NOTE: scene_id, location_id, act_id moved to graph edges
    let priority: i64 = node.get("priority").unwrap_or(0);
    let is_favorite: bool = node.get("is_favorite").unwrap_or(false);
    // NOTE: chain_id, chain_position moved to CONTAINS_EVENT edge
    let created_at_str: String = node.get("created_at")?;
    let updated_at_str: String = node.get("updated_at")?;

    let tags: Vec<String> = serde_json::from_str(&tags_json)?;
    // Deserialize to stored types, then convert to domain types
    let stored_triggers: Vec<StoredNarrativeTrigger> = serde_json::from_str(&triggers_json)?;
    let trigger_conditions: Vec<NarrativeTrigger> =
        stored_triggers.into_iter().map(|t| t.into()).collect();
    let stored_outcomes: Vec<StoredEventOutcome> = serde_json::from_str(&outcomes_json)?;
    let outcomes: Vec<EventOutcome> = stored_outcomes.into_iter().map(|o| o.into()).collect();

    let trigger_logic = match trigger_logic_str.as_str() {
        "Any" => TriggerLogic::Any,
        s if s.starts_with("AtLeast(") => {
            let n: u32 = s
                .trim_start_matches("AtLeast(")
                .trim_end_matches(')')
                .parse()
                .unwrap_or(1);
            TriggerLogic::AtLeast(n)
        }
        _ => TriggerLogic::All,
    };

    Ok(NarrativeEvent {
        id: NarrativeEventId::from(Uuid::parse_str(&id_str)?),
        world_id: WorldId::from(Uuid::parse_str(&world_id_str)?),
        name,
        description,
        tags,
        trigger_conditions,
        trigger_logic,
        scene_direction,
        suggested_opening: if suggested_opening.is_empty() {
            None
        } else {
            Some(suggested_opening)
        },
        // NOTE: featured_npcs now stored as FEATURES_NPC edges
        outcomes,
        default_outcome: if default_outcome.is_empty() {
            None
        } else {
            Some(default_outcome)
        },
        is_active,
        is_triggered,
        triggered_at: if triggered_at_str.is_empty() {
            None
        } else {
            DateTime::parse_from_rfc3339(&triggered_at_str)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        },
        selected_outcome: if selected_outcome.is_empty() {
            None
        } else {
            Some(selected_outcome)
        },
        is_repeatable,
        trigger_count: trigger_count as u32,
        delay_turns: delay_turns as u32,
        expires_after_turns: if expires_after_turns < 0 {
            None
        } else {
            Some(expires_after_turns as u32)
        },
        // NOTE: scene_id, location_id, act_id now stored as graph edges
        priority: priority as i32,
        is_favorite,
        // NOTE: chain_id, chain_position now stored as CONTAINS_EVENT edge
        created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc),
    })
}

// =============================================================================
// Trait Implementation
// =============================================================================

#[async_trait]
impl NarrativeEventRepositoryPort for Neo4jNarrativeEventRepository {
    async fn create(&self, event: &NarrativeEvent) -> Result<()> {
        self.create(event).await
    }

    async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>> {
        self.get(id).await
    }

    async fn update(&self, event: &NarrativeEvent) -> Result<bool> {
        self.update(event).await
    }

    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        self.list_by_world(world_id).await
    }

    async fn list_active(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        self.list_active(world_id).await
    }

    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        self.list_favorites(world_id).await
    }

    async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        self.list_pending(world_id).await
    }

    async fn toggle_favorite(&self, id: NarrativeEventId) -> Result<bool> {
        self.toggle_favorite(id).await
    }

    async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> Result<bool> {
        self.set_active(id, is_active).await
    }

    async fn mark_triggered(&self, id: NarrativeEventId, outcome_name: Option<String>) -> Result<bool> {
        self.mark_triggered(id, outcome_name).await
    }

    async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool> {
        self.reset_triggered(id).await
    }

    async fn delete(&self, id: NarrativeEventId) -> Result<bool> {
        self.delete(id).await
    }

    // =========================================================================
    // TIED_TO_SCENE Edge Methods
    // =========================================================================

    async fn tie_to_scene(&self, event_id: NarrativeEventId, scene_id: SceneId) -> Result<bool> {
        self.tie_to_scene(event_id, scene_id).await
    }

    async fn get_tied_scene(&self, event_id: NarrativeEventId) -> Result<Option<SceneId>> {
        self.get_tied_scene(event_id).await
    }

    async fn untie_from_scene(&self, event_id: NarrativeEventId) -> Result<bool> {
        self.untie_from_scene(event_id).await
    }

    // =========================================================================
    // TIED_TO_LOCATION Edge Methods
    // =========================================================================

    async fn tie_to_location(
        &self,
        event_id: NarrativeEventId,
        location_id: LocationId,
    ) -> Result<bool> {
        self.tie_to_location(event_id, location_id).await
    }

    async fn get_tied_location(&self, event_id: NarrativeEventId) -> Result<Option<LocationId>> {
        self.get_tied_location(event_id).await
    }

    async fn untie_from_location(&self, event_id: NarrativeEventId) -> Result<bool> {
        self.untie_from_location(event_id).await
    }

    // =========================================================================
    // BELONGS_TO_ACT Edge Methods
    // =========================================================================

    async fn assign_to_act(&self, event_id: NarrativeEventId, act_id: ActId) -> Result<bool> {
        self.assign_to_act(event_id, act_id).await
    }

    async fn get_act(&self, event_id: NarrativeEventId) -> Result<Option<ActId>> {
        self.get_act(event_id).await
    }

    async fn unassign_from_act(&self, event_id: NarrativeEventId) -> Result<bool> {
        self.unassign_from_act(event_id).await
    }

    // =========================================================================
    // FEATURES_NPC Edge Methods
    // =========================================================================

    async fn add_featured_npc(
        &self,
        event_id: NarrativeEventId,
        featured_npc: FeaturedNpc,
    ) -> Result<bool> {
        self.add_featured_npc(event_id, featured_npc).await
    }

    async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> Result<Vec<FeaturedNpc>> {
        self.get_featured_npcs(event_id).await
    }

    async fn remove_featured_npc(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
    ) -> Result<bool> {
        self.remove_featured_npc(event_id, character_id).await
    }

    async fn update_featured_npc_role(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
        role: Option<String>,
    ) -> Result<bool> {
        self.update_featured_npc_role(event_id, character_id, role)
            .await
    }

    // =========================================================================
    // Chain Membership Query Methods
    // =========================================================================

    async fn get_chain_memberships(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<Vec<EventChainMembership>> {
        self.get_chain_memberships(event_id).await
    }

    // =========================================================================
    // Query Methods for Events by Edge Relationships
    // =========================================================================

    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<NarrativeEvent>> {
        self.list_by_scene(scene_id).await
    }

    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<NarrativeEvent>> {
        self.list_by_location(location_id).await
    }

    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<NarrativeEvent>> {
        self.list_by_act(act_id).await
    }

    async fn list_by_featured_npc(&self, character_id: CharacterId) -> Result<Vec<NarrativeEvent>> {
        self.list_by_featured_npc(character_id).await
    }
}
