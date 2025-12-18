//! Scene entity - Complete storytelling unit
//!
//! # Graph-First Design (Phase 0.D)
//!
//! The following relationships are stored as Neo4j edges, NOT embedded fields:
//! - Location: `(Scene)-[:AT_LOCATION]->(Location)`
//! - Featured characters: `(Scene)-[:FEATURES_CHARACTER {role, entrance_cue}]->(Character)`
//!
//! Entry conditions remain as JSON (acceptable per ADR - complex nested non-relational)

use crate::domain::value_objects::{ActId, CharacterId, LocationId, SceneId};

/// A scene - a complete unit of storytelling
///
/// NOTE: `location_id` and `featured_characters` are kept for backward compatibility
/// during Phase 0.D migration. New code should use repository edge methods:
/// - Location: AT_LOCATION edge via `scene_repository.set_location()`
/// - Characters: FEATURES_CHARACTER edge via `scene_repository.add_featured_character()`
#[derive(Debug, Clone)]
pub struct Scene {
    pub id: SceneId,
    pub act_id: ActId,
    pub name: String,
    /// DEPRECATED: Use AT_LOCATION edge via repository
    pub location_id: LocationId,
    pub time_context: TimeContext,
    /// Override backdrop (if different from location default)
    pub backdrop_override: Option<String>,
    /// Conditions that must be met to enter this scene (stored as JSON)
    pub entry_conditions: Vec<SceneCondition>,
    /// DEPRECATED: Use FEATURES_CHARACTER edge via repository
    pub featured_characters: Vec<CharacterId>,
    /// DM guidance for LLM responses
    pub directorial_notes: String,
    /// Order within the act (for sequential scenes)
    pub order: u32,
}

impl Scene {
    pub fn new(act_id: ActId, name: impl Into<String>, location_id: LocationId) -> Self {
        Self {
            id: SceneId::new(),
            act_id,
            name: name.into(),
            location_id,
            time_context: TimeContext::Unspecified,
            backdrop_override: None,
            entry_conditions: Vec::new(),
            featured_characters: Vec::new(),
            directorial_notes: String::new(),
            order: 0,
        }
    }

    pub fn with_character(mut self, character_id: CharacterId) -> Self {
        self.featured_characters.push(character_id);
        self
    }

    pub fn with_time(mut self, time_context: TimeContext) -> Self {
        self.time_context = time_context;
        self
    }

    pub fn with_directorial_notes(mut self, notes: impl Into<String>) -> Self {
        self.directorial_notes = notes.into();
        self
    }

    pub fn with_entry_condition(mut self, condition: SceneCondition) -> Self {
        self.entry_conditions.push(condition);
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }
}

/// Time context for a scene
#[derive(Debug, Clone)]
pub enum TimeContext {
    /// No specific time
    Unspecified,
    /// Time of day
    TimeOfDay(TimeOfDay),
    /// Relative to an event
    During(String),
    /// Specific description
    Custom(String),
}

/// Time of day
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeOfDay {
    Dawn,
    Morning,
    Midday,
    Afternoon,
    Evening,
    Dusk,
    Night,
    Midnight,
}

/// Condition for entering a scene
#[derive(Debug, Clone)]
pub enum SceneCondition {
    /// Must have completed another scene
    CompletedScene(SceneId),
    /// Must have a specific item
    HasItem(crate::domain::value_objects::ItemId),
    /// Must have a relationship with a character
    KnowsCharacter(crate::domain::value_objects::CharacterId),
    /// A flag must be set
    FlagSet(String),
    /// Custom condition expression
    Custom(String),
}

/// Data for the FEATURES_CHARACTER edge between Scene and Character
#[derive(Debug, Clone)]
pub struct SceneCharacter {
    /// The character's role in this scene
    pub role: SceneCharacterRole,
    /// When/how the character enters the scene
    pub entrance_cue: Option<String>,
}

impl SceneCharacter {
    pub fn new(role: SceneCharacterRole) -> Self {
        Self {
            role,
            entrance_cue: None,
        }
    }

    pub fn with_entrance_cue(mut self, cue: impl Into<String>) -> Self {
        self.entrance_cue = Some(cue.into());
        self
    }
}

/// Role a character plays in a scene
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneCharacterRole {
    /// Primary character in the scene
    Primary,
    /// Secondary/supporting character
    Secondary,
    /// Background character (ambient presence)
    Background,
}

impl std::fmt::Display for SceneCharacterRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "Primary"),
            Self::Secondary => write!(f, "Secondary"),
            Self::Background => write!(f, "Background"),
        }
    }
}

impl std::str::FromStr for SceneCharacterRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Primary" => Ok(Self::Primary),
            "Secondary" => Ok(Self::Secondary),
            "Background" => Ok(Self::Background),
            _ => Err(()),
        }
    }
}
