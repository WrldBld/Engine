//! Narrative event approval service - encapsulates DM approval of narrative
//! event suggestions, marking events as triggered, recording story events, and
//! constructing `ServerMessage::NarrativeEventTriggered`.
//!
//! # Architecture Note: Hexagonal Violation
//!
//! This service currently imports `SessionManager` directly from the infrastructure layer:
//! ```ignore
//! use crate::infrastructure::session::{ClientId, SessionManager};
//! ```
//!
//! This violates hexagonal architecture rules where application services should depend only
//! on domain types and port traits, not infrastructure implementations.
//!
//! ## Planned Refactoring
//!
//! To fix this violation, the service should be refactored to:
//! 1. Accept `AsyncSessionPort` trait bound instead of concrete `SessionManager`
//! 2. Replace all `SessionManager` method calls with equivalent `AsyncSessionPort` methods
//! 3. Move `ClientId` to domain or port definitions if it's not already there
//!
//! The port trait already exists at: `application/ports/outbound/async_session_port.rs`
//! The adapter already exists at: `infrastructure/session_adapter.rs`
//!
//! This service primarily uses SessionManager for DM authorization checks and sending
//! messages to DMs. A refactoring should preserve these authorization and messaging semantics
//! via the port trait.

use std::sync::Arc;

use crate::application::ports::outbound::AsyncSessionPort;
use crate::application::services::{NarrativeEventService, StoryEventService};
use crate::domain::value_objects::{NarrativeEventId, SessionId};
use crate::infrastructure::websocket::messages::ServerMessage;

/// Service responsible for narrative suggestion approval flows.
///
/// # TODO: Architecture Violation
///
/// This service previously depended on `SessionManager` (a concrete infrastructure type)
/// rather than the async session port trait. It now uses `AsyncSessionPort`, restoring a
/// proper hexagonal boundary between application and infrastructure.
pub struct NarrativeEventApprovalService<N: NarrativeEventService> {
    sessions: Arc<dyn AsyncSessionPort>,
    narrative_event_service: Arc<N>,
    story_event_service: Arc<StoryEventService>,
}

impl<N> NarrativeEventApprovalService<N>
where
    N: NarrativeEventService,
{
    pub fn new(
        sessions: Arc<dyn AsyncSessionPort>,
        narrative_event_service: Arc<N>,
        story_event_service: Arc<StoryEventService>,
    ) -> Self {
        Self {
            sessions,
            narrative_event_service,
            story_event_service,
        }
    }

    /// Handle `ClientMessage::NarrativeEventSuggestionDecision`.
    pub async fn handle_decision(
        &self,
        client_id: String,
        request_id: String,
        event_id: String,
        approved: bool,
        selected_outcome: Option<String>,
    ) -> Option<ServerMessage> {
        tracing::debug!(
            "Received narrative event suggestion decision for {}: event={}, approved={}, outcome={:?}",
            request_id,
            event_id,
            approved,
            selected_outcome
        );
        // Only the DM for the client's session may approve/reject narrative events
        if !self.sessions.is_client_dm(&client_id).await {
            return None;
        }

        if let Some(session_id) = self.sessions.get_client_session(&client_id).await {
            if approved {
                return self
                    .approve_and_trigger(
                        session_id,
                        request_id,
                        event_id,
                        selected_outcome,
                    )
                    .await;
            } else {
                tracing::info!(
                    "DM rejected narrative event {} trigger for request {}",
                    event_id,
                    request_id
                );
            }
        }

        None
    }

    async fn approve_and_trigger(
        &self,
        session_id: SessionId,
        request_id: String,
        event_id: String,
        selected_outcome: Option<String>,
    ) -> Option<ServerMessage> {
        let event_uuid = match uuid::Uuid::parse_str(&event_id) {
            Ok(uuid) => NarrativeEventId::from_uuid(uuid),
            Err(_) => {
                tracing::error!("Invalid event_id: {}", event_id);
                return Some(ServerMessage::Error {
                    code: "INVALID_EVENT_ID".to_string(),
                    message: "Invalid narrative event ID format".to_string(),
                });
            }
        };

        let narrative_event = match self.narrative_event_service.get(event_uuid).await {
            Ok(Some(event)) => event,
            Ok(None) => {
                tracing::error!("Narrative event {} not found", event_id);
                return Some(ServerMessage::Error {
                    code: "EVENT_NOT_FOUND".to_string(),
                    message: format!("Narrative event {} not found", event_id),
                });
            }
            Err(e) => {
                tracing::error!("Failed to load narrative event: {}", e);
                return Some(ServerMessage::Error {
                    code: "EVENT_LOAD_ERROR".to_string(),
                    message: format!("Failed to load narrative event: {}", e),
                });
            }
        };

        // 2. Find the selected outcome (or default to first)
        let outcome = if let Some(outcome_name) = &selected_outcome {
            narrative_event
                .outcomes
                .iter()
                .find(|o| o.name == *outcome_name)
                .cloned()
                .or_else(|| narrative_event.outcomes.first().cloned())
        } else {
            narrative_event.outcomes.first().cloned()
        };

        let outcome = match outcome {
            Some(o) => o,
            None => {
                tracing::error!("Narrative event {} has no outcomes", event_id);
                return Some(ServerMessage::Error {
                    code: "NO_OUTCOMES".to_string(),
                    message: format!("Narrative event {} has no outcomes", event_id),
                });
            }
        };

        // 3. Mark event as triggered
        if let Err(e) = self
            .narrative_event_service
            .mark_triggered(event_uuid, Some(outcome.name.clone()))
            .await
        {
            tracing::error!("Failed to mark narrative event as triggered: {}", e);
        }

        // 4. Record a StoryEvent for the timeline
        let session_id_for_story = session_id.to_string();
        let session_uuid = match uuid::Uuid::parse_str(&session_id_for_story) {
            Ok(uuid) => SessionId::from_uuid(uuid),
            Err(_) => {
                tracing::warn!(
                    "Invalid session_id for story event: {}",
                    session_id_for_story
                );
                SessionId::from_uuid(uuid::Uuid::nil())
            }
        };

        if let Err(e) = self
            .story_event_service
            .record_narrative_event_triggered(
                narrative_event.world_id,
                session_uuid,
                None, // scene_id
                None, // location_id
                event_uuid,
                narrative_event.name.clone(),
                Some(outcome.name.clone()),
                outcome
                    .effects
                    .iter()
                    .map(|e| format!("{:?}", e))
                    .collect(),
                vec![], // involved_characters
                None,   // game_time
            )
            .await
        {
            tracing::error!("Failed to record story event: {}", e);
        }

        // 5. Broadcast scene direction to DM via the async session port
        let scene_direction = ServerMessage::NarrativeEventTriggered {
            event_id: event_id.clone(),
            event_name: narrative_event.name.clone(),
            outcome_description: outcome.description.clone(),
            scene_direction: narrative_event.scene_direction.clone(),
        };
        if let Ok(msg_json) = serde_json::to_value(&scene_direction) {
            if let Err(e) = self.sessions.send_to_dm(session_id, msg_json).await {
                tracing::error!("Failed to send NarrativeEventTriggered to DM: {}", e);
            }
        } else {
            tracing::error!("Failed to serialize NarrativeEventTriggered message for event {}", event_id);
        }

        tracing::info!(
            "Triggered narrative event '{}' with outcome '{}' for request {}",
            narrative_event.name,
            outcome.description,
            request_id
        );

        None
    }
}


