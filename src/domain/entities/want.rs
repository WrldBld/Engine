//! Want entity - Character desires (Actantial Model)
//!
//! # Graph-First Design (Phase 0.C)
//!
//! A Want is a node that represents a character's desire. The target of the want
//! is stored as a TARGETS edge, NOT embedded in the Want node:
//!
//! ```cypher
//! (character:Character)-[:HAS_WANT {priority: 1}]->(want:Want)
//! (want:Want)-[:TARGETS]->(target)  // Character, Item, or Goal
//! ```
//!
//! Actantial roles (Helper, Opponent, Sender, Receiver) are edges from the
//! character to other characters, referencing the want_id:
//!
//! ```cypher
//! (subject:Character)-[:VIEWS_AS_HELPER {want_id: "...", reason: "..."}]->(helper:Character)
//! ```

use chrono::{DateTime, Utc};

use crate::domain::value_objects::WantId;

/// A character's desire or goal (Actantial model)
///
/// The want's target is stored via a `TARGETS` edge to:
/// - Character (wants something from/about a person)
/// - Item (wants a specific item)
/// - Goal (wants an abstract outcome)
#[derive(Debug, Clone)]
pub struct Want {
    pub id: WantId,
    /// Description of what the character wants
    pub description: String,
    /// Intensity of the want (0.0 = mild interest, 1.0 = obsession)
    pub intensity: f32,
    /// Whether players know about this want
    pub known_to_player: bool,
    /// When this want was created
    pub created_at: DateTime<Utc>,
}

impl Want {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: WantId::new(),
            description: description.into(),
            intensity: 0.5,
            known_to_player: false,
            created_at: Utc::now(),
        }
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

/// Data for the HAS_WANT edge between Character and Want
#[derive(Debug, Clone)]
pub struct CharacterWant {
    /// The want node
    pub want: Want,
    /// Priority (1 = primary want, 2 = secondary, etc.)
    pub priority: u32,
    /// When this want was acquired
    pub acquired_at: DateTime<Utc>,
}

impl CharacterWant {
    pub fn new(want: Want, priority: u32) -> Self {
        Self {
            want,
            priority,
            acquired_at: Utc::now(),
        }
    }
}

/// The type of target a want can have (for querying purposes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WantTargetType {
    Character,
    Item,
    Goal,
}

/// Actantial role type for character views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActantialRole {
    /// Character sees target as helping their want
    Helper,
    /// Character sees target as opposing their want
    Opponent,
    /// Character sees target as having initiated/motivated their want
    Sender,
    /// Character sees target as benefiting from their want's fulfillment
    Receiver,
}

/// Data for actantial view edges (VIEWS_AS_HELPER, etc.)
#[derive(Debug, Clone)]
pub struct ActantialView {
    /// Which want this relates to
    pub want_id: WantId,
    /// Why the character views the target this way
    pub reason: String,
    /// When this view was assigned
    pub assigned_at: DateTime<Utc>,
}

impl ActantialView {
    pub fn new(want_id: WantId, reason: impl Into<String>) -> Self {
        Self {
            want_id,
            reason: reason.into(),
            assigned_at: Utc::now(),
        }
    }
}
