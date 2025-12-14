//! Actantial model - Character wants and desires

use super::{CharacterId, ItemId, WantId};

/// A character's desire or goal (actantial model)
#[derive(Debug, Clone)]
pub struct Want {
    pub id: WantId,
    pub description: String,
    pub target: Option<ActantTarget>,
    /// Intensity of the want (0.0 = mild interest, 1.0 = obsession)
    pub intensity: f32,
    /// Whether players know about this want
    pub known_to_player: bool,
}

impl Want {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: WantId::new(),
            description: description.into(),
            target: None,
            intensity: 0.5,
            known_to_player: false,
        }
    }

    pub fn with_target(mut self, target: ActantTarget) -> Self {
        self.target = Some(target);
        self
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    pub fn known(mut self) -> Self {
        self.known_to_player = true;
        self
    }
}

/// The target of a character's want
#[derive(Debug, Clone)]
pub enum ActantTarget {
    /// Wants something from/about another character
    Character(CharacterId),
    /// Wants a specific item
    Item(ItemId),
    /// Wants an abstract goal (power, revenge, peace, etc.)
    Goal(String),
}
