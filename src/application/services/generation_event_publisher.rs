//! Generation Event Publisher - Maps GenerationEvents to AppEvents
//!
//! This service listens to the GenerationEvent channel and publishes
//! corresponding AppEvents through the event bus.

use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::application::dto::AppEvent;
use crate::application::ports::outbound::EventBusPort;
use crate::application::services::generation_service::GenerationEvent;

/// Publisher that converts GenerationEvents to AppEvents
pub struct GenerationEventPublisher {
    event_bus: Arc<dyn EventBusPort<AppEvent>>,
}

impl GenerationEventPublisher {
    /// Create a new publisher
    pub fn new(event_bus: Arc<dyn EventBusPort<AppEvent>>) -> Self {
        Self { event_bus }
    }

    /// Run the publisher, consuming generation events and publishing app events
    ///
    /// This should be spawned as a background task
    pub async fn run(self, mut generation_event_rx: UnboundedReceiver<GenerationEvent>) {
        while let Some(event) = generation_event_rx.recv().await {
            let app_event = self.map_to_app_event(event);
            if let Some(app_event) = app_event {
                if let Err(e) = self.event_bus.publish(app_event).await {
                    tracing::error!("Failed to publish generation app event: {}", e);
                }
            }
        }
        tracing::info!("Generation event publisher shutting down");
    }

    /// Map a GenerationEvent to an AppEvent.
    ///
    /// For now, generation events are not session-scoped and are broadcast
    /// based on world context in the subscriber. When generation is invoked
    /// from a specific live session in the future, this mapping can be
    /// extended to populate `session_id`.
    fn map_to_app_event(&self, event: GenerationEvent) -> Option<AppEvent> {
        match event {
            GenerationEvent::BatchQueued {
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                position,
            } => Some(AppEvent::GenerationBatchQueued {
                batch_id: batch_id.to_string(),
                entity_type: entity_type.to_string(),
                entity_id,
                asset_type: asset_type.to_string(),
                position,
                session_id: None,
            }),
            GenerationEvent::BatchProgress {
                batch_id,
                progress,
            } => Some(AppEvent::GenerationBatchProgress {
                batch_id: batch_id.to_string(),
                progress,
                session_id: None,
            }),
            GenerationEvent::BatchComplete {
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                asset_count,
            } => Some(AppEvent::GenerationBatchCompleted {
                batch_id: batch_id.to_string(),
                entity_type: entity_type.to_string(),
                entity_id,
                asset_type: asset_type.to_string(),
                asset_count,
                session_id: None,
            }),
            GenerationEvent::BatchFailed {
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                error,
            } => Some(AppEvent::GenerationBatchFailed {
                batch_id: batch_id.to_string(),
                entity_type: entity_type.to_string(),
                entity_id,
                asset_type: asset_type.to_string(),
                error,
                session_id: None,
            }),
            GenerationEvent::SuggestionQueued {
                request_id,
                field_type,
                entity_id,
                world_id,
            } => Some(AppEvent::SuggestionQueued {
                request_id,
                field_type,
                entity_id,
                world_id: world_id.map(|w| w.to_string()),
            }),
            GenerationEvent::SuggestionProgress { request_id, status, world_id } => {
                Some(AppEvent::SuggestionProgress {
                    request_id,
                    status,
                    world_id: world_id.map(|w| w.to_string()),
                })
            }
            GenerationEvent::SuggestionComplete {
                request_id,
                field_type,
                suggestions,
                world_id,
            } => Some(AppEvent::SuggestionCompleted {
                request_id,
                field_type,
                suggestions,
                world_id: world_id.map(|w| w.to_string()),
            }),
            GenerationEvent::SuggestionFailed {
                request_id,
                field_type,
                error,
                world_id,
            } => Some(AppEvent::SuggestionFailed {
                request_id,
                field_type,
                error,
                world_id: world_id.map(|w| w.to_string()),
            }),
        }
    }
}

