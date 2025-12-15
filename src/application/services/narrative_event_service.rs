//! NarrativeEvent Service - Application service for narrative event management
//!
//! This service provides use case implementations for creating, updating,
//! and managing narrative events within a world.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::NarrativeEventRepositoryPort;
use crate::domain::entities::NarrativeEvent;
use crate::domain::value_objects::{NarrativeEventId, WorldId};

/// NarrativeEvent service trait defining the application use cases
#[async_trait]
pub trait NarrativeEventService: Send + Sync {
    /// Get a narrative event by ID
    async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>>;

    /// List all narrative events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List active narrative events for a world
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List favorite narrative events for a world
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List pending (not yet triggered) narrative events
    async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// Create a new narrative event
    async fn create(&self, event: NarrativeEvent) -> Result<NarrativeEvent>;

    /// Update an existing narrative event
    async fn update(&self, event: NarrativeEvent) -> Result<NarrativeEvent>;

    /// Delete a narrative event
    async fn delete(&self, id: NarrativeEventId) -> Result<bool>;

    /// Toggle favorite status for a narrative event
    async fn toggle_favorite(&self, id: NarrativeEventId) -> Result<bool>;

    /// Set active status for a narrative event
    async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> Result<bool>;

    /// Mark event as triggered
    async fn mark_triggered(&self, id: NarrativeEventId, outcome_name: Option<String>) -> Result<bool>;

    /// Reset triggered status (for repeatable events)
    async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool>;
}

/// Default implementation of NarrativeEventService using port abstractions
#[derive(Clone)]
pub struct NarrativeEventServiceImpl {
    repository: Arc<dyn NarrativeEventRepositoryPort>,
}

impl NarrativeEventServiceImpl {
    /// Create a new NarrativeEventServiceImpl with the given repository
    pub fn new(repository: Arc<dyn NarrativeEventRepositoryPort>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl NarrativeEventService for NarrativeEventServiceImpl {
    #[instrument(skip(self))]
    async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>> {
        debug!(event_id = %id, "Fetching narrative event");
        self.repository
            .get(id)
            .await
            .context("Failed to get narrative event from repository")
    }

    #[instrument(skip(self))]
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        debug!(world_id = %world_id, "Listing all narrative events for world");
        self.repository
            .list_by_world(world_id)
            .await
            .context("Failed to list narrative events from repository")
    }

    #[instrument(skip(self))]
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        debug!(world_id = %world_id, "Listing active narrative events for world");
        self.repository
            .list_active(world_id)
            .await
            .context("Failed to list active narrative events from repository")
    }

    #[instrument(skip(self))]
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        debug!(world_id = %world_id, "Listing favorite narrative events for world");
        self.repository
            .list_favorites(world_id)
            .await
            .context("Failed to list favorite narrative events from repository")
    }

    #[instrument(skip(self))]
    async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        debug!(world_id = %world_id, "Listing pending narrative events for world");
        self.repository
            .list_pending(world_id)
            .await
            .context("Failed to list pending narrative events from repository")
    }

    #[instrument(skip(self))]
    async fn create(&self, event: NarrativeEvent) -> Result<NarrativeEvent> {
        info!(
            event_id = %event.id,
            world_id = %event.world_id,
            name = %event.name,
            "Creating narrative event"
        );

        self.repository
            .create(&event)
            .await
            .context("Failed to create narrative event in repository")?;

        Ok(event)
    }

    #[instrument(skip(self))]
    async fn update(&self, event: NarrativeEvent) -> Result<NarrativeEvent> {
        info!(
            event_id = %event.id,
            name = %event.name,
            "Updating narrative event"
        );

        self.repository
            .update(&event)
            .await
            .context("Failed to update narrative event in repository")?;

        Ok(event)
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: NarrativeEventId) -> Result<bool> {
        info!(event_id = %id, "Deleting narrative event");
        self.repository
            .delete(id)
            .await
            .context("Failed to delete narrative event from repository")
    }

    #[instrument(skip(self))]
    async fn toggle_favorite(&self, id: NarrativeEventId) -> Result<bool> {
        debug!(event_id = %id, "Toggling favorite status for narrative event");
        self.repository
            .toggle_favorite(id)
            .await
            .context("Failed to toggle favorite status for narrative event")
    }

    #[instrument(skip(self))]
    async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> Result<bool> {
        info!(
            event_id = %id,
            is_active = is_active,
            "Setting active status for narrative event"
        );
        self.repository
            .set_active(id, is_active)
            .await
            .context("Failed to set active status for narrative event")
    }

    #[instrument(skip(self))]
    async fn mark_triggered(&self, id: NarrativeEventId, outcome_name: Option<String>) -> Result<bool> {
        info!(
            event_id = %id,
            outcome = ?outcome_name,
            "Marking narrative event as triggered"
        );
        self.repository
            .mark_triggered(id, outcome_name)
            .await
            .context("Failed to mark narrative event as triggered")
    }

    #[instrument(skip(self))]
    async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool> {
        info!(event_id = %id, "Resetting triggered status for narrative event");
        self.repository
            .reset_triggered(id)
            .await
            .context("Failed to reset triggered status for narrative event")
    }
}
