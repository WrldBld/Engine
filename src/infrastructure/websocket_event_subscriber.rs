//! WebSocket Event Subscriber - Maps AppEvents to ServerMessages
//!
//! This subscriber polls the event repository and broadcasts relevant events
//! to WebSocket clients via the SessionManager.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::application::dto::AppEvent;
use crate::application::ports::outbound::{AppEventRepositoryPort, AsyncSessionPort};
use crate::infrastructure::event_bus::InProcessEventNotifier;
use crate::infrastructure::session::SessionManager;
use crate::infrastructure::websocket::ServerMessage;

/// WebSocket event subscriber
pub struct WebSocketEventSubscriber {
    repository: Arc<dyn AppEventRepositoryPort>,
    notifier: InProcessEventNotifier,
    async_session_port: Arc<dyn AsyncSessionPort>,
    sessions: Arc<RwLock<SessionManager>>,
    poll_interval: Duration,
}

impl WebSocketEventSubscriber {
    /// Create a new WebSocket event subscriber
    pub fn new(
        repository: Arc<dyn AppEventRepositoryPort>,
        notifier: InProcessEventNotifier,
        async_session_port: Arc<dyn AsyncSessionPort>,
        sessions: Arc<RwLock<SessionManager>>,
        poll_interval_seconds: u64,
    ) -> Self {
        Self {
            repository,
            notifier,
            async_session_port,
            sessions,
            poll_interval: Duration::from_secs(poll_interval_seconds),
        }
    }

    /// Run the subscriber (spawned as background task)
    pub async fn run(self) {
        let mut last_event_id: i64 = 0;
        tracing::info!("Starting WebSocket event subscriber");

        loop {
            tokio::select! {
                _ = self.notifier.wait() => {
                    tracing::debug!("Event notifier triggered, fetching new events");
                    if let Err(e) = self.process_new_events(&mut last_event_id).await {
                        tracing::error!("Failed to process new events: {}", e);
                    }
                }
                _ = tokio::time::sleep(self.poll_interval) => {
                    tracing::debug!("Polling for new events (last_id: {})", last_event_id);
                    if let Err(e) = self.process_new_events(&mut last_event_id).await {
                        tracing::error!("Failed to process new events (poll): {}", e);
                    }
                }
            }
        }
    }

    /// Fetch and process new events since last_event_id
    async fn process_new_events(&self, last_event_id: &mut i64) -> anyhow::Result<()> {
        const BATCH_SIZE: u32 = 100;

        // Fetch events since last_event_id
        let events = self
            .repository
            .fetch_since(*last_event_id, BATCH_SIZE)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch events: {}", e))?;

        if events.is_empty() {
            return Ok(());
        }

        tracing::debug!("Processing {} new events", events.len());

        for (event_id, event, _timestamp) in events {
            // Map AppEvent to ServerMessage and determine target session/world (if any)
            if let Some(message) = self.map_to_server_message(&event) {
                let target_session = event.session_id().map(|s| s.to_string());
                let target_world = event.world_id().map(|s| s.to_string());

                let sessions = self.sessions.read().await;

                if let Some(ref session_id_str) = target_session {
                    // Prefer session-scoped routing when session_id is present.
                    if let Ok(session_uuid) = uuid::Uuid::parse_str(session_id_str) {
                        let session_id = crate::domain::value_objects::SessionId::from_uuid(session_uuid);
                        sessions.broadcast_to_session(session_id, &message);
                    } else {
                        tracing::warn!("Invalid session_id on AppEvent: {}", session_id_str);
                    }
                    drop(sessions);
                } else {
                    // Fall back to world-scoped (or global) routing.
                    drop(sessions);
                    let session_ids = self.async_session_port.list_session_ids().await;
                    let sessions = self.sessions.read().await;
                    for session_id in session_ids {
                        if let Some(session) = sessions.get_session(session_id) {
                            // If the event is associated with a specific world, only
                            // deliver to sessions for that world. Otherwise, broadcast
                            // to all sessions.
                            if let Some(ref world_id_str) = target_world {
                                if session.world_id.to_string() != *world_id_str {
                                    continue;
                                }
                            }
                            sessions.broadcast_to_session(session_id, &message);
                        }
                    }
                    drop(sessions);
                }
            }

            // Update last processed event ID
            *last_event_id = event_id;
        }

        tracing::debug!("Processed events up to ID {}", *last_event_id);
        Ok(())
    }

    /// Map an AppEvent to a ServerMessage
    ///
    /// Returns None if the event is not relevant to WebSocket clients
    fn map_to_server_message(&self, event: &AppEvent) -> Option<ServerMessage> {
        match event {
            AppEvent::GenerationBatchQueued {
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                position,
                ..
            } => Some(ServerMessage::GenerationQueued {
                batch_id: batch_id.clone(),
                entity_type: entity_type.clone(),
                entity_id: entity_id.clone(),
                asset_type: asset_type.clone(),
                position: *position,
            }),
            AppEvent::GenerationBatchProgress { batch_id, progress, .. } => {
                Some(ServerMessage::GenerationProgress {
                    batch_id: batch_id.clone(),
                    progress: *progress,
                })
            }
            AppEvent::GenerationBatchCompleted {
                batch_id,
                asset_count,
                ..
            } => Some(ServerMessage::GenerationComplete {
                batch_id: batch_id.clone(),
                asset_count: *asset_count,
            }),
            AppEvent::GenerationBatchFailed { batch_id, error, .. } => {
                Some(ServerMessage::GenerationFailed {
                    batch_id: batch_id.clone(),
                    error: error.clone(),
                })
            }
            AppEvent::SuggestionQueued {
                request_id,
                field_type,
                entity_id,
                ..
            } => Some(ServerMessage::SuggestionQueued {
                request_id: request_id.clone(),
                field_type: field_type.clone(),
                entity_id: entity_id.clone(),
            }),
            AppEvent::SuggestionProgress { request_id, status, .. } => {
                Some(ServerMessage::SuggestionProgress {
                    request_id: request_id.clone(),
                    status: status.clone(),
                })
            }
            AppEvent::SuggestionCompleted {
                request_id,
                suggestions,
                ..
            } => Some(ServerMessage::SuggestionComplete {
                request_id: request_id.clone(),
                suggestions: suggestions.clone(),
            }),
            AppEvent::SuggestionFailed {
                request_id,
                error,
                ..
            } => Some(ServerMessage::SuggestionFailed {
                request_id: request_id.clone(),
                error: error.clone(),
            }),
            // Story events, narrative events, and challenges are not yet broadcasted via WebSocket
            // These could be added in the future if needed
            AppEvent::StoryEventCreated { .. }
            | AppEvent::NarrativeEventTriggered { .. }
            | AppEvent::ChallengeResolved { .. } => None,
        }
    }
}

