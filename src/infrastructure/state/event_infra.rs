//! Event infrastructure and repositories

use std::sync::Arc;

use crate::application::dto::AppEvent;
use crate::application::ports::outbound::{AppEventRepositoryPort, EventBusPort, GenerationReadStatePort};
use crate::infrastructure::event_bus::InProcessEventNotifier;

/// Event infrastructure for application-level events
///
/// This struct groups the event bus, event notification system, and
/// repositories for tracking application events and generation state.
pub struct EventInfrastructure {
    pub event_bus: Arc<dyn EventBusPort<AppEvent>>,
    pub event_notifier: InProcessEventNotifier,
    pub app_event_repository: Arc<dyn AppEventRepositoryPort>,
    pub generation_read_state_repository: Arc<dyn GenerationReadStatePort>,
}

impl EventInfrastructure {
    /// Creates a new EventInfrastructure instance
    pub fn new(
        event_bus: Arc<dyn EventBusPort<AppEvent>>,
        event_notifier: InProcessEventNotifier,
        app_event_repository: Arc<dyn AppEventRepositoryPort>,
        generation_read_state_repository: Arc<dyn GenerationReadStatePort>,
    ) -> Self {
        Self {
            event_bus,
            event_notifier,
            app_event_repository,
            generation_read_state_repository,
        }
    }
}
