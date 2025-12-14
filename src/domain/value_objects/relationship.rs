//! Character relationships for social network modeling

use super::{CharacterId, RelationshipId};

/// A relationship between two characters
#[derive(Debug, Clone)]
pub struct Relationship {
    pub id: RelationshipId,
    pub from_character: CharacterId,
    pub to_character: CharacterId,
    pub relationship_type: RelationshipType,
    /// Sentiment from -1.0 (hatred) to 1.0 (love)
    pub sentiment: f32,
    pub history: Vec<RelationshipEvent>,
    /// Whether players know about this relationship
    pub known_to_player: bool,
}

impl Relationship {
    pub fn new(from: CharacterId, to: CharacterId, relationship_type: RelationshipType) -> Self {
        Self {
            id: RelationshipId::new(),
            from_character: from,
            to_character: to,
            relationship_type,
            sentiment: 0.0,
            history: Vec::new(),
            known_to_player: true,
        }
    }

    pub fn with_sentiment(mut self, sentiment: f32) -> Self {
        self.sentiment = sentiment.clamp(-1.0, 1.0);
        self
    }

    pub fn secret(mut self) -> Self {
        self.known_to_player = false;
        self
    }

    pub fn add_event(&mut self, event: RelationshipEvent) {
        self.history.push(event);
    }
}

/// Types of relationships between characters
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationshipType {
    Family(FamilyRelation),
    Romantic,
    Professional,
    Rivalry,
    Friendship,
    Mentorship,
    Enmity,
    Custom(String),
}

/// Family relationship subtypes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyRelation {
    Parent,
    Child,
    Sibling,
    Spouse,
    Grandparent,
    Grandchild,
    AuntUncle,
    NieceNephew,
    Cousin,
}

/// An event that affected a relationship
#[derive(Debug, Clone)]
pub struct RelationshipEvent {
    pub description: String,
    pub sentiment_change: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
