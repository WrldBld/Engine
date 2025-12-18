//! StoryEvent entity - Immutable records of gameplay events
//!
//! StoryEvents are automatically created when actions occur during gameplay,
//! forming a complete timeline of the game session.
//!
//! # Graph Relationships (stored as Neo4j edges, not embedded fields)
//!
//! - `OCCURRED_IN_SESSION` → Session: The session where this event occurred
//! - `OCCURRED_AT` → Location: Optional location where the event occurred
//! - `OCCURRED_IN_SCENE` → Scene: Optional scene where the event occurred
//! - `INVOLVES` → Character/PlayerCharacter: Characters involved in the event (with role)
//! - `TRIGGERED_BY_NARRATIVE` → NarrativeEvent: Optional causative narrative event
//! - `RECORDS_CHALLENGE` → Challenge: Optional challenge this event records
//!
//! Note: `event_type` (StoryEventType) remains as JSON because it contains complex
//! discriminated union data that doesn't represent entity relationships.

use chrono::{DateTime, Utc};

use crate::domain::value_objects::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId, SessionId, StoryEventId,
    WorldId,
};

/// A story event - an immutable record of something that happened
///
/// # Graph Relationships
///
/// The following associations are stored as graph edges (not embedded fields):
/// - Session: Use `OCCURRED_IN_SESSION` edge via repository methods
/// - Location: Use `OCCURRED_AT` edge via repository methods
/// - Scene: Use `OCCURRED_IN_SCENE` edge via repository methods
/// - Involved characters: Use `INVOLVES` edges via repository methods
/// - Triggering narrative event: Use `TRIGGERED_BY_NARRATIVE` edge via repository methods
/// - Challenge recorded: Use `RECORDS_CHALLENGE` edge via repository methods
#[derive(Debug, Clone)]
pub struct StoryEvent {
    pub id: StoryEventId,
    pub world_id: WorldId,
    // NOTE: session_id moved to OCCURRED_IN_SESSION edge
    // NOTE: scene_id moved to OCCURRED_IN_SCENE edge
    // NOTE: location_id moved to OCCURRED_AT edge
    /// The type and details of the event
    /// (Kept as JSON - contains complex discriminated union data)
    pub event_type: StoryEventType,
    /// When this event occurred (real-world timestamp)
    pub timestamp: DateTime<Utc>,
    /// In-game time context (optional, e.g., "Day 3, Evening")
    pub game_time: Option<String>,
    /// Narrative summary (auto-generated or DM-edited)
    pub summary: String,
    // NOTE: involved_characters moved to INVOLVES edges
    /// Whether this event is hidden from timeline UI (but still tracked)
    pub is_hidden: bool,
    /// Tags for filtering/searching
    pub tags: Vec<String>,
    // NOTE: triggered_by moved to TRIGGERED_BY_NARRATIVE edge
}

/// Categories of story events that occurred during gameplay
#[derive(Debug, Clone, PartialEq)]
pub enum StoryEventType {
    /// Player character moved to a new location
    LocationChange {
        from_location: Option<LocationId>,
        to_location: LocationId,
        character_id: CharacterId,
        travel_method: Option<String>,
    },

    /// Dialogue exchange with an NPC
    DialogueExchange {
        npc_id: CharacterId,
        npc_name: String,
        player_dialogue: String,
        npc_response: String,
        topics_discussed: Vec<String>,
        tone: Option<String>,
    },

    /// Combat encounter started or completed
    CombatEvent {
        combat_type: CombatEventType,
        participants: Vec<CharacterId>,
        enemies: Vec<String>,
        outcome: Option<CombatOutcome>,
        location_id: LocationId,
        rounds: Option<u32>,
    },

    /// Challenge attempted (skill check, saving throw, etc.)
    ChallengeAttempted {
        challenge_id: Option<ChallengeId>,
        challenge_name: String,
        character_id: CharacterId,
        skill_used: Option<String>,
        difficulty: Option<String>,
        roll_result: Option<i32>,
        modifier: Option<i32>,
        outcome: ChallengeEventOutcome,
    },

    /// Item acquired by a character
    ItemAcquired {
        item_name: String,
        item_description: Option<String>,
        character_id: CharacterId,
        source: ItemSource,
        quantity: u32,
    },

    /// Item transferred between characters
    ItemTransferred {
        item_name: String,
        from_character: Option<CharacterId>,
        to_character: CharacterId,
        quantity: u32,
        reason: Option<String>,
    },

    /// Item used or consumed
    ItemUsed {
        item_name: String,
        character_id: CharacterId,
        target: Option<String>,
        effect: String,
        consumed: bool,
    },

    /// Relationship changed between characters
    RelationshipChanged {
        from_character: CharacterId,
        to_character: CharacterId,
        previous_sentiment: Option<f32>,
        new_sentiment: f32,
        sentiment_change: f32,
        reason: String,
    },

    /// Scene transition occurred
    SceneTransition {
        from_scene: Option<SceneId>,
        to_scene: SceneId,
        from_scene_name: Option<String>,
        to_scene_name: String,
        trigger_reason: String,
    },

    /// Information revealed to players
    InformationRevealed {
        info_type: InfoType,
        title: String,
        content: String,
        source: Option<CharacterId>,
        importance: InfoImportance,
        persist_to_journal: bool,
    },

    /// NPC performed an action through LLM tool call
    NpcAction {
        npc_id: CharacterId,
        npc_name: String,
        action_type: String,
        description: String,
        dm_approved: bool,
        dm_modified: bool,
    },

    /// DM manually added narrative marker/note
    DmMarker {
        title: String,
        note: String,
        importance: MarkerImportance,
        marker_type: DmMarkerType,
    },

    /// Narrative event was triggered
    NarrativeEventTriggered {
        narrative_event_id: NarrativeEventId,
        narrative_event_name: String,
        outcome_branch: Option<String>,
        effects_applied: Vec<String>,
    },

    /// Character stat was modified
    StatModified {
        character_id: CharacterId,
        stat_name: String,
        previous_value: i32,
        new_value: i32,
        reason: String,
    },

    /// Flag was set or unset
    FlagChanged {
        flag_name: String,
        new_value: bool,
        reason: String,
    },

    /// Session started
    SessionStarted {
        session_number: u32,
        session_name: Option<String>,
        players_present: Vec<String>,
    },

    /// Session ended
    SessionEnded {
        duration_minutes: u32,
        summary: String,
    },

    /// Custom event type for extensibility
    Custom {
        event_subtype: String,
        title: String,
        description: String,
        data: serde_json::Value,
    },
}

/// Combat event subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatEventType {
    Started,
    RoundCompleted,
    CharacterDefeated,
    CharacterFled,
    Ended,
}

/// Combat outcome types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatOutcome {
    Victory,
    Defeat,
    Fled,
    Negotiated,
    Draw,
    Interrupted,
}

/// Challenge event outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeEventOutcome {
    CriticalSuccess,
    Success,
    PartialSuccess,
    Failure,
    CriticalFailure,
}

/// Source of an acquired item
#[derive(Debug, Clone, PartialEq)]
pub enum ItemSource {
    Found { location: String },
    Purchased { from: String, cost: Option<String> },
    Gifted { from: CharacterId },
    Looted { from: String },
    Crafted,
    Reward { for_what: String },
    Stolen { from: String },
    Custom { description: String },
}

/// Type of revealed information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoType {
    Lore,
    Quest,
    Character,
    Location,
    Item,
    Secret,
    Rumor,
}

/// Importance level for revealed information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoImportance {
    Minor,
    Notable,
    Major,
    Critical,
}

/// Importance level for DM markers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerImportance {
    Minor,
    Notable,
    Major,
    Critical,
}

/// Types of DM markers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmMarkerType {
    Note,
    PlotPoint,
    CharacterMoment,
    WorldEvent,
    PlayerDecision,
    Foreshadowing,
    Callback,
    Custom,
}

impl StoryEvent {
    /// Create a new story event
    ///
    /// NOTE: Session, location, scene, and character associations are now stored as
    /// graph edges and must be created separately using the edge methods:
    /// - `set_session()` for OCCURRED_IN_SESSION edge
    /// - `set_location()` for OCCURRED_AT edge
    /// - `set_scene()` for OCCURRED_IN_SCENE edge
    /// - `add_involved_character()` for INVOLVES edges
    /// - `set_triggered_by()` for TRIGGERED_BY_NARRATIVE edge
    /// - `set_recorded_challenge()` for RECORDS_CHALLENGE edge
    pub fn new(world_id: WorldId, event_type: StoryEventType) -> Self {
        Self {
            id: StoryEventId::new(),
            world_id,
            // NOTE: session_id now stored as OCCURRED_IN_SESSION edge
            // NOTE: scene_id now stored as OCCURRED_IN_SCENE edge
            // NOTE: location_id now stored as OCCURRED_AT edge
            event_type,
            timestamp: Utc::now(),
            game_time: None,
            summary: String::new(),
            // NOTE: involved_characters now stored as INVOLVES edges
            is_hidden: false,
            tags: Vec::new(),
            // NOTE: triggered_by now stored as TRIGGERED_BY_NARRATIVE edge
        }
    }

    // NOTE: with_scene() removed - use repository edge method set_scene()
    // NOTE: with_location() removed - use repository edge method set_location()

    pub fn with_game_time(mut self, game_time: impl Into<String>) -> Self {
        self.game_time = Some(game_time.into());
        self
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    // NOTE: with_characters() removed - use repository edge method add_involved_character()

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn hidden(mut self) -> Self {
        self.is_hidden = true;
        self
    }

    // NOTE: triggered_by() removed - use repository edge method set_triggered_by()

    /// Generate an automatic summary based on event type
    pub fn auto_summarize(&mut self) {
        self.summary = match &self.event_type {
            StoryEventType::LocationChange { .. } => "Traveled to a new location".to_string(),
            StoryEventType::DialogueExchange { npc_name, .. } => {
                format!("Spoke with {}", npc_name)
            }
            StoryEventType::CombatEvent {
                combat_type,
                outcome,
                ..
            } => match (combat_type, outcome) {
                (CombatEventType::Started, _) => "Combat began".to_string(),
                (CombatEventType::Ended, Some(CombatOutcome::Victory)) => {
                    "Won the battle".to_string()
                }
                (CombatEventType::Ended, Some(CombatOutcome::Defeat)) => {
                    "Lost the battle".to_string()
                }
                (CombatEventType::Ended, Some(CombatOutcome::Fled)) => "Fled from combat".to_string(),
                (CombatEventType::Ended, Some(CombatOutcome::Negotiated)) => {
                    "Combat ended through negotiation".to_string()
                }
                (CombatEventType::Ended, Some(CombatOutcome::Draw)) => "Combat ended in a draw".to_string(),
                (CombatEventType::Ended, Some(CombatOutcome::Interrupted)) => {
                    "Combat was interrupted".to_string()
                }
                (CombatEventType::Ended, None) => "Combat ended".to_string(),
                (CombatEventType::RoundCompleted, _) => "Combat round completed".to_string(),
                (CombatEventType::CharacterDefeated, _) => "Character defeated in combat".to_string(),
                (CombatEventType::CharacterFled, _) => "Character fled from combat".to_string(),
            },
            StoryEventType::ChallengeAttempted {
                challenge_name,
                outcome,
                ..
            } => format!("{}: {:?}", challenge_name, outcome),
            StoryEventType::ItemAcquired { item_name, .. } => format!("Acquired {}", item_name),
            StoryEventType::ItemTransferred { item_name, .. } => {
                format!("Transferred {}", item_name)
            }
            StoryEventType::ItemUsed { item_name, .. } => format!("Used {}", item_name),
            StoryEventType::RelationshipChanged { reason, .. } => reason.clone(),
            StoryEventType::SceneTransition { to_scene_name, .. } => {
                format!("Entered: {}", to_scene_name)
            }
            StoryEventType::InformationRevealed { title, .. } => {
                format!("Discovered: {}", title)
            }
            StoryEventType::NpcAction {
                npc_name,
                action_type,
                ..
            } => format!("{} performed {}", npc_name, action_type),
            StoryEventType::DmMarker { title, .. } => title.clone(),
            StoryEventType::NarrativeEventTriggered {
                narrative_event_name,
                ..
            } => format!("Event: {}", narrative_event_name),
            StoryEventType::StatModified {
                stat_name, reason, ..
            } => format!("{} changed: {}", stat_name, reason),
            StoryEventType::FlagChanged {
                flag_name,
                new_value,
                ..
            } => format!(
                "Flag {}: {}",
                flag_name,
                if *new_value { "set" } else { "unset" }
            ),
            StoryEventType::SessionStarted { session_number, .. } => {
                format!("Session {} started", session_number)
            }
            StoryEventType::SessionEnded { summary, .. } => summary.clone(),
            StoryEventType::Custom { title, .. } => title.clone(),
        };
    }

    /// Get a display-friendly type name
    pub fn type_name(&self) -> &'static str {
        match &self.event_type {
            StoryEventType::LocationChange { .. } => "Location Change",
            StoryEventType::DialogueExchange { .. } => "Dialogue",
            StoryEventType::CombatEvent { .. } => "Combat",
            StoryEventType::ChallengeAttempted { .. } => "Challenge",
            StoryEventType::ItemAcquired { .. } => "Item Acquired",
            StoryEventType::ItemTransferred { .. } => "Item Transfer",
            StoryEventType::ItemUsed { .. } => "Item Used",
            StoryEventType::RelationshipChanged { .. } => "Relationship",
            StoryEventType::SceneTransition { .. } => "Scene Transition",
            StoryEventType::InformationRevealed { .. } => "Information",
            StoryEventType::NpcAction { .. } => "NPC Action",
            StoryEventType::DmMarker { .. } => "DM Marker",
            StoryEventType::NarrativeEventTriggered { .. } => "Narrative Event",
            StoryEventType::StatModified { .. } => "Stat Modified",
            StoryEventType::FlagChanged { .. } => "Flag Changed",
            StoryEventType::SessionStarted { .. } => "Session Start",
            StoryEventType::SessionEnded { .. } => "Session End",
            StoryEventType::Custom { .. } => "Custom",
        }
    }
}

// =============================================================================
// Edge Support Structs
// =============================================================================

/// Represents a character involved in a story event (via INVOLVES edge)
#[derive(Debug, Clone)]
pub struct InvolvedCharacter {
    /// The character ID
    pub character_id: CharacterId,
    /// Role in the event (e.g., "Speaker", "Target", "Witness", "Actor")
    pub role: String,
}

impl InvolvedCharacter {
    pub fn new(character_id: CharacterId, role: impl Into<String>) -> Self {
        Self {
            character_id,
            role: role.into(),
        }
    }

    /// Create an involved character with the "Actor" role
    pub fn actor(character_id: CharacterId) -> Self {
        Self::new(character_id, "Actor")
    }

    /// Create an involved character with the "Target" role
    pub fn target(character_id: CharacterId) -> Self {
        Self::new(character_id, "Target")
    }

    /// Create an involved character with the "Speaker" role
    pub fn speaker(character_id: CharacterId) -> Self {
        Self::new(character_id, "Speaker")
    }

    /// Create an involved character with the "Witness" role
    pub fn witness(character_id: CharacterId) -> Self {
        Self::new(character_id, "Witness")
    }
}
