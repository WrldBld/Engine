//! Application Events - Cross-cutting notifications about system outcomes
//!
//! These events are published through the EventBus after significant
//! application/domain state changes. They are coarse-grained, serializable,
//! and suitable for persistence, fan-out to multiple subscribers, and
//! eventual cross-service/cross-process communication.

use serde::{Deserialize, Serialize};

/// Application-level events published through the EventBus
///
/// Many of these events are tied to a live play session. For those, we
/// include an optional `session_id` so consumers (e.g. WebSocket
/// subscribers) can route them to the correct session without guessing
/// from world context alone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    // ========================================================================
    // Story & Narrative Events
    // ========================================================================
    /// A story event was created (gameplay history)
    StoryEventCreated {
        story_event_id: String,
        world_id: String,
        event_type: String,
    },

    /// A narrative event was triggered
    NarrativeEventTriggered {
        event_id: String,
        world_id: String,
        event_name: String,
        outcome_name: String,
        /// Optional live session that triggered this event
        #[serde(default)]
        session_id: Option<String>,
    },

    // ========================================================================
    // Challenge Events
    // ========================================================================
    /// A challenge was resolved (success or failure)
    ChallengeResolved {
        challenge_id: Option<String>,
        challenge_name: String,
        world_id: String,
        character_id: String,
        success: bool,
        roll: Option<i32>,
        total: Option<i32>,
        /// Optional live session where this challenge occurred
        #[serde(default)]
        session_id: Option<String>,
    },

    // ========================================================================
    // Generation Events (Asset/Image)
    // ========================================================================
    /// A generation batch was queued
    GenerationBatchQueued {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        position: u32,
        /// Optional live session associated with this generation request
        #[serde(default)]
        session_id: Option<String>,
    },

    /// A generation batch is progressing
    GenerationBatchProgress {
        batch_id: String,
        progress: u8,
        #[serde(default)]
        session_id: Option<String>,
    },

    /// A generation batch completed successfully
    GenerationBatchCompleted {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        asset_count: u32,
        #[serde(default)]
        session_id: Option<String>,
    },

    /// A generation batch failed
    GenerationBatchFailed {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        error: String,
        #[serde(default)]
        session_id: Option<String>,
    },

    // ========================================================================
    // Suggestion Events (LLM Text) - World-scoped
    // ========================================================================
    /// An LLM suggestion request was queued
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
        /// World ID for routing to correct clients
        #[serde(default)]
        world_id: Option<String>,
    },

    /// An LLM suggestion request is being processed
    SuggestionProgress {
        request_id: String,
        status: String,
        /// World ID for routing to correct clients
        #[serde(default)]
        world_id: Option<String>,
    },

    /// An LLM suggestion request completed
    SuggestionCompleted {
        request_id: String,
        field_type: String,
        suggestions: Vec<String>,
        /// World ID for routing to correct clients
        #[serde(default)]
        world_id: Option<String>,
    },

    /// An LLM suggestion request failed
    SuggestionFailed {
        request_id: String,
        field_type: String,
        error: String,
        /// World ID for routing to correct clients
        #[serde(default)]
        world_id: Option<String>,
    },
}

impl AppEvent {
    /// Get the event type as a string (for logging, storage, filtering)
    pub fn event_type(&self) -> &'static str {
        match self {
            AppEvent::StoryEventCreated { .. } => "StoryEventCreated",
            AppEvent::NarrativeEventTriggered { .. } => "NarrativeEventTriggered",
            AppEvent::ChallengeResolved { .. } => "ChallengeResolved",
            AppEvent::GenerationBatchQueued { .. } => "GenerationBatchQueued",
            AppEvent::GenerationBatchProgress { .. } => "GenerationBatchProgress",
            AppEvent::GenerationBatchCompleted { .. } => "GenerationBatchCompleted",
            AppEvent::GenerationBatchFailed { .. } => "GenerationBatchFailed",
            AppEvent::SuggestionQueued { .. } => "SuggestionQueued",
            AppEvent::SuggestionProgress { .. } => "SuggestionProgress",
            AppEvent::SuggestionCompleted { .. } => "SuggestionCompleted",
            AppEvent::SuggestionFailed { .. } => "SuggestionFailed",
        }
    }

    /// Get the world_id if this event is associated with a specific world
    pub fn world_id(&self) -> Option<&str> {
        match self {
            AppEvent::StoryEventCreated { world_id, .. }
            | AppEvent::NarrativeEventTriggered { world_id, .. }
            | AppEvent::ChallengeResolved { world_id, .. } => Some(world_id.as_str()),
            // Suggestion events are world-scoped
            AppEvent::SuggestionQueued { world_id, .. }
            | AppEvent::SuggestionProgress { world_id, .. }
            | AppEvent::SuggestionCompleted { world_id, .. }
            | AppEvent::SuggestionFailed { world_id, .. } => world_id.as_deref(),
            _ => None,
        }
    }

    /// Get the session_id if this event is associated with a live session
    pub fn session_id(&self) -> Option<&str> {
        match self {
            AppEvent::NarrativeEventTriggered { session_id, .. }
            | AppEvent::ChallengeResolved { session_id, .. }
            | AppEvent::GenerationBatchQueued { session_id, .. }
            | AppEvent::GenerationBatchProgress { session_id, .. }
            | AppEvent::GenerationBatchCompleted { session_id, .. }
            | AppEvent::GenerationBatchFailed { session_id, .. } => {
                session_id.as_deref()
            }
            // Suggestion events use world_id for routing, not session_id
            _ => None,
        }
    }
}

