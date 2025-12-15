//! Narrative event approval service - encapsulates DM approval of narrative
//! event suggestions, marking events as triggered, recording story events, and
//! constructing `ServerMessage::NarrativeEventTriggered`.

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::application::services::{NarrativeEventService, StoryEventService};
use crate::domain::value_objects::{NarrativeEventId, SessionId};
use crate::infrastructure::session::{ClientId, SessionManager};
use crate::infrastructure::websocket::messages::ServerMessage;

/// Service responsible for narrative suggestion approval flows.
pub struct NarrativeEventApprovalService<N: NarrativeEventService> {
    sessions: Arc<RwLock<SessionManager>>,
    narrative_event_service: Arc<N>,
    story_event_service: Arc<StoryEventService>,
}

impl<N> NarrativeEventApprovalService<N>
where
    N: NarrativeEventService,
{
    pub fn new(
        sessions: Arc<RwLock<SessionManager>>,
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
        client_id: ClientId,
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

        let sessions_read = self.sessions.read().await;
        if let Some(session_id) = sessions_read.get_client_session(client_id) {
            if let Some(session) = sessions_read.get_session(session_id) {
                if let Some(dm) = session.get_dm() {
                    if dm.client_id == client_id {
                        if approved {
                            return self
                                .approve_and_trigger(
                                    client_id,
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
                }
            }
        }

        None
    }

    async fn approve_and_trigger(
        &self,
        client_id: ClientId,
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

        // 5. Broadcast scene direction to DM
        let sessions_read = self.sessions.read().await;
        let scene_direction = ServerMessage::NarrativeEventTriggered {
            event_id: event_id.clone(),
            event_name: narrative_event.name.clone(),
            outcome_description: outcome.description.clone(),
            scene_direction: narrative_event.scene_direction.clone(),
        };

        if let Some(sid) = sessions_read.get_client_session(client_id) {
            if let Some(session) = sessions_read.get_session(sid) {
                session.send_to_dm(&scene_direction);
            }
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


