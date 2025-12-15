//! Domain events - Notifications of significant state changes
//!
//! Domain events represent things that have happened in the domain.
//! They can be used for:
//! - Event sourcing
//! - Notifying other parts of the system
//! - Audit logging
//! - Triggering side effects
//!
//! **Status**: Planned for Phase 3.1 DDD implementation
//! Currently unused - will be wired with event publisher/subscriber

use chrono::{DateTime, Utc};

use crate::domain::value_objects::{
    WorldId, SceneId, CharacterId, LocationId, InteractionId,
};

/// Base data for all events
///
/// **Status**: Planned for Phase 3.1
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EventMetadata {
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Optional correlation ID for tracing
    pub correlation_id: Option<String>,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            correlation_id: None,
        }
    }
}

/// All domain events in the system
///
/// **Status**: Planned for Phase 3.1
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum DomainEvent {
    // ========================================================================
    // World Events
    // ========================================================================

    /// A new world was created
    WorldCreated {
        metadata: EventMetadata,
        world_id: WorldId,
        name: String,
    },

    /// A world was deleted
    WorldDeleted {
        metadata: EventMetadata,
        world_id: WorldId,
    },

    // ========================================================================
    // Character Events
    // ========================================================================

    /// A new character was created
    CharacterCreated {
        metadata: EventMetadata,
        character_id: CharacterId,
        world_id: WorldId,
        name: String,
    },

    /// A character was updated
    CharacterUpdated {
        metadata: EventMetadata,
        character_id: CharacterId,
    },

    /// A character was deleted
    CharacterDeleted {
        metadata: EventMetadata,
        character_id: CharacterId,
        world_id: WorldId,
    },

    /// A character's archetype changed
    CharacterArchetypeChanged {
        metadata: EventMetadata,
        character_id: CharacterId,
        old_archetype: Option<String>,
        new_archetype: String,
    },

    // ========================================================================
    // Location Events
    // ========================================================================

    /// A new location was created
    LocationCreated {
        metadata: EventMetadata,
        location_id: LocationId,
        world_id: WorldId,
        name: String,
    },

    /// A connection was created between locations
    LocationsConnected {
        metadata: EventMetadata,
        from_location: LocationId,
        to_location: LocationId,
    },

    // ========================================================================
    // Scene Events
    // ========================================================================

    /// A scene transition occurred
    SceneTransitioned {
        metadata: EventMetadata,
        from_scene: Option<SceneId>,
        to_scene: SceneId,
        world_id: WorldId,
    },

    /// A scene was started
    SceneStarted {
        metadata: EventMetadata,
        scene_id: SceneId,
        location_id: LocationId,
    },

    /// A scene was ended
    SceneEnded {
        metadata: EventMetadata,
        scene_id: SceneId,
    },

    // ========================================================================
    // Dialogue Events
    // ========================================================================

    /// Dialogue was spoken by a character
    DialogueSpoken {
        metadata: EventMetadata,
        scene_id: SceneId,
        speaker_id: CharacterId,
        dialogue: String,
    },

    /// A player made a dialogue choice
    DialogueChoiceMade {
        metadata: EventMetadata,
        scene_id: SceneId,
        player_id: String,
        choice_id: String,
        choice_text: String,
    },

    // ========================================================================
    // Interaction Events
    // ========================================================================

    /// An interaction was triggered
    InteractionTriggered {
        metadata: EventMetadata,
        interaction_id: InteractionId,
        actor_id: String,
        target_id: Option<String>,
    },

    // ========================================================================
    // Game Session Events
    // ========================================================================

    /// A game session was started
    SessionStarted {
        metadata: EventMetadata,
        session_id: String,
        world_id: WorldId,
    },

    /// A player joined a session
    PlayerJoined {
        metadata: EventMetadata,
        session_id: String,
        player_id: String,
        role: String,
    },

    /// A player left a session
    PlayerLeft {
        metadata: EventMetadata,
        session_id: String,
        player_id: String,
    },
}

impl DomainEvent {
    /// Get the metadata for this event
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            DomainEvent::WorldCreated { metadata, .. } => metadata,
            DomainEvent::WorldDeleted { metadata, .. } => metadata,
            DomainEvent::CharacterCreated { metadata, .. } => metadata,
            DomainEvent::CharacterUpdated { metadata, .. } => metadata,
            DomainEvent::CharacterDeleted { metadata, .. } => metadata,
            DomainEvent::CharacterArchetypeChanged { metadata, .. } => metadata,
            DomainEvent::LocationCreated { metadata, .. } => metadata,
            DomainEvent::LocationsConnected { metadata, .. } => metadata,
            DomainEvent::SceneTransitioned { metadata, .. } => metadata,
            DomainEvent::SceneStarted { metadata, .. } => metadata,
            DomainEvent::SceneEnded { metadata, .. } => metadata,
            DomainEvent::DialogueSpoken { metadata, .. } => metadata,
            DomainEvent::DialogueChoiceMade { metadata, .. } => metadata,
            DomainEvent::InteractionTriggered { metadata, .. } => metadata,
            DomainEvent::SessionStarted { metadata, .. } => metadata,
            DomainEvent::PlayerJoined { metadata, .. } => metadata,
            DomainEvent::PlayerLeft { metadata, .. } => metadata,
        }
    }

    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            DomainEvent::WorldCreated { .. } => "WorldCreated",
            DomainEvent::WorldDeleted { .. } => "WorldDeleted",
            DomainEvent::CharacterCreated { .. } => "CharacterCreated",
            DomainEvent::CharacterUpdated { .. } => "CharacterUpdated",
            DomainEvent::CharacterDeleted { .. } => "CharacterDeleted",
            DomainEvent::CharacterArchetypeChanged { .. } => "CharacterArchetypeChanged",
            DomainEvent::LocationCreated { .. } => "LocationCreated",
            DomainEvent::LocationsConnected { .. } => "LocationsConnected",
            DomainEvent::SceneTransitioned { .. } => "SceneTransitioned",
            DomainEvent::SceneStarted { .. } => "SceneStarted",
            DomainEvent::SceneEnded { .. } => "SceneEnded",
            DomainEvent::DialogueSpoken { .. } => "DialogueSpoken",
            DomainEvent::DialogueChoiceMade { .. } => "DialogueChoiceMade",
            DomainEvent::InteractionTriggered { .. } => "InteractionTriggered",
            DomainEvent::SessionStarted { .. } => "SessionStarted",
            DomainEvent::PlayerJoined { .. } => "PlayerJoined",
            DomainEvent::PlayerLeft { .. } => "PlayerLeft",
        }
    }
}
