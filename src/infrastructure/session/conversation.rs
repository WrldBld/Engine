//! Conversation history management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a single turn in the conversation history
///
/// Each turn tracks a message exchange between a player/NPC and captures
/// the speaker identity, content, and timestamp for LLM context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Name of the speaker (character name or "Player")
    pub speaker: String,
    /// Content of the dialogue or action
    pub content: String,
    /// Timestamp when this turn occurred
    pub timestamp: DateTime<Utc>,
    /// Whether this was a player action (true) or NPC response (false)
    pub is_player: bool,
}

impl ConversationTurn {
    /// Create a new conversation turn
    pub fn new(speaker: String, content: String, is_player: bool) -> Self {
        Self {
            speaker,
            content,
            timestamp: Utc::now(),
            is_player,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_turn_creation() {
        let turn = ConversationTurn::new(
            "Alice".to_string(),
            "Hello, world!".to_string(),
            true,
        );

        assert_eq!(turn.speaker, "Alice");
        assert_eq!(turn.content, "Hello, world!");
        assert!(turn.is_player);
        // Timestamp should be very recent
        let elapsed = Utc::now().signed_duration_since(turn.timestamp);
        assert!(elapsed.num_seconds() < 1);
    }
}
