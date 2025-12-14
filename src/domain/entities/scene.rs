//! Scene entity - Complete storytelling unit (location + time + events)

use crate::domain::value_objects::{ActId, CharacterId, LocationId, SceneId};

/// A scene - a complete unit of storytelling
#[derive(Debug, Clone)]
pub struct Scene {
    pub id: SceneId,
    pub act_id: ActId,
    pub name: String,
    pub location_id: LocationId,
    pub time_context: TimeContext,
    /// Override backdrop (if different from location default)
    pub backdrop_override: Option<String>,
    /// Conditions that must be met to enter this scene
    pub entry_conditions: Vec<SceneCondition>,
    /// Characters featured in this scene
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

    pub fn with_time(mut self, time_context: TimeContext) -> Self {
        self.time_context = time_context;
        self
    }

    pub fn with_directorial_notes(mut self, notes: impl Into<String>) -> Self {
        self.directorial_notes = notes.into();
        self
    }

    pub fn with_character(mut self, character_id: CharacterId) -> Self {
        self.featured_characters.push(character_id);
        self
    }

    pub fn with_entry_condition(mut self, condition: SceneCondition) -> Self {
        self.entry_conditions.push(condition);
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
    KnowsCharacter(CharacterId),
    /// A flag must be set
    FlagSet(String),
    /// Custom condition expression
    Custom(String),
}
