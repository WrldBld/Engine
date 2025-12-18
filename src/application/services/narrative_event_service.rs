//! NarrativeEvent Service - Application service for narrative event management
//!
//! This service provides use case implementations for creating, updating,
//! and managing narrative events within a world.
//!
//! # Graph-First Architecture
//!
//! NarrativeEvent relationships are stored as graph edges:
//! - Scene tie: `TIED_TO_SCENE` edge via `tie_to_scene()`
//! - Location tie: `TIED_TO_LOCATION` edge via `tie_to_location()`
//! - Act assignment: `BELONGS_TO_ACT` edge via `assign_to_act()`
//! - Featured NPCs: `FEATURES_NPC` edge via `add_featured_npc()`
//!
//! Triggers and outcomes remain as JSON (complex nested non-relational data).

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::NarrativeEventRepositoryPort;
use crate::domain::entities::{EventChainMembership, FeaturedNpc, NarrativeEvent};
use crate::domain::value_objects::{ActId, CharacterId, LocationId, NarrativeEventId, SceneId, WorldId};

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

    // =========================================================================
    // TIED_TO_SCENE Edge Methods
    // =========================================================================

    /// Tie event to a scene (creates TIED_TO_SCENE edge)
    async fn tie_to_scene(&self, event_id: NarrativeEventId, scene_id: SceneId) -> Result<bool>;

    /// Get the scene this event is tied to (if any)
    async fn get_tied_scene(&self, event_id: NarrativeEventId) -> Result<Option<SceneId>>;

    /// Remove scene tie (deletes TIED_TO_SCENE edge)
    async fn untie_from_scene(&self, event_id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // TIED_TO_LOCATION Edge Methods
    // =========================================================================

    /// Tie event to a location (creates TIED_TO_LOCATION edge)
    async fn tie_to_location(&self, event_id: NarrativeEventId, location_id: LocationId) -> Result<bool>;

    /// Get the location this event is tied to (if any)
    async fn get_tied_location(&self, event_id: NarrativeEventId) -> Result<Option<LocationId>>;

    /// Remove location tie (deletes TIED_TO_LOCATION edge)
    async fn untie_from_location(&self, event_id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // BELONGS_TO_ACT Edge Methods
    // =========================================================================

    /// Assign event to an act (creates BELONGS_TO_ACT edge)
    async fn assign_to_act(&self, event_id: NarrativeEventId, act_id: ActId) -> Result<bool>;

    /// Get the act this event belongs to (if any)
    async fn get_act(&self, event_id: NarrativeEventId) -> Result<Option<ActId>>;

    /// Remove act assignment (deletes BELONGS_TO_ACT edge)
    async fn unassign_from_act(&self, event_id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // FEATURES_NPC Edge Methods
    // =========================================================================

    /// Add a featured NPC to the event (creates FEATURES_NPC edge)
    async fn add_featured_npc(&self, event_id: NarrativeEventId, featured_npc: FeaturedNpc) -> Result<bool>;

    /// Get all featured NPCs for an event
    async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> Result<Vec<FeaturedNpc>>;

    /// Remove a featured NPC from the event (deletes FEATURES_NPC edge)
    async fn remove_featured_npc(&self, event_id: NarrativeEventId, character_id: CharacterId) -> Result<bool>;

    /// Update featured NPC role
    async fn update_featured_npc_role(&self, event_id: NarrativeEventId, character_id: CharacterId, role: Option<String>) -> Result<bool>;

    // =========================================================================
    // Chain Membership Query Methods
    // =========================================================================

    /// Get chain membership info for an event
    async fn get_chain_memberships(&self, event_id: NarrativeEventId) -> Result<Vec<EventChainMembership>>;

    // =========================================================================
    // Query Methods for Events by Edge Relationships
    // =========================================================================

    /// List events tied to a specific scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<NarrativeEvent>>;

    /// List events tied to a specific location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<NarrativeEvent>>;

    /// List events belonging to a specific act
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<NarrativeEvent>>;

    /// List events featuring a specific NPC
    async fn list_by_featured_npc(&self, character_id: CharacterId) -> Result<Vec<NarrativeEvent>>;
}

use crate::application::dto::AppEvent;
use crate::application::ports::outbound::EventBusPort;

/// Default implementation of NarrativeEventService using port abstractions
#[derive(Clone)]
pub struct NarrativeEventServiceImpl {
    repository: Arc<dyn NarrativeEventRepositoryPort>,
    event_bus: Arc<dyn EventBusPort<AppEvent>>,
}

impl NarrativeEventServiceImpl {
    /// Create a new NarrativeEventServiceImpl with the given repository
    pub fn new(repository: Arc<dyn NarrativeEventRepositoryPort>, event_bus: Arc<dyn EventBusPort<AppEvent>>) -> Self {
        Self { repository, event_bus }
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
        
        // Get the event details before marking as triggered
        let event = self.repository.get(id).await?;
        
        let result = self.repository
            .mark_triggered(id, outcome_name.clone())
            .await
            .context("Failed to mark narrative event as triggered")?;
        
        // Publish AppEvent if we have the event details
        if let Some(event) = event {
            let app_event = AppEvent::NarrativeEventTriggered {
                event_id: id.to_string(),
                world_id: event.world_id.to_string(),
                event_name: event.name.clone(),
                outcome_name: outcome_name.unwrap_or_else(|| "default".to_string()),
                // Narrative events are world-scoped for now; session_id will be
                // attached by higher-level orchestration when triggered from
                // a specific live session.
                session_id: None,
            };

            if let Err(e) = self.event_bus.publish(app_event).await {
                tracing::error!("Failed to publish NarrativeEventTriggered for {}: {}", id, e);
            }
        }
        
        Ok(result)
    }

    #[instrument(skip(self))]
    async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool> {
        info!(event_id = %id, "Resetting triggered status for narrative event");
        self.repository
            .reset_triggered(id)
            .await
            .context("Failed to reset triggered status for narrative event")
    }

    // =========================================================================
    // TIED_TO_SCENE Edge Methods
    // =========================================================================

    #[instrument(skip(self))]
    async fn tie_to_scene(&self, event_id: NarrativeEventId, scene_id: SceneId) -> Result<bool> {
        debug!(event_id = %event_id, scene_id = %scene_id, "Tying narrative event to scene");
        self.repository
            .tie_to_scene(event_id, scene_id)
            .await
            .context("Failed to tie narrative event to scene")
    }

    #[instrument(skip(self))]
    async fn get_tied_scene(&self, event_id: NarrativeEventId) -> Result<Option<SceneId>> {
        debug!(event_id = %event_id, "Getting tied scene for narrative event");
        self.repository
            .get_tied_scene(event_id)
            .await
            .context("Failed to get tied scene for narrative event")
    }

    #[instrument(skip(self))]
    async fn untie_from_scene(&self, event_id: NarrativeEventId) -> Result<bool> {
        debug!(event_id = %event_id, "Untying narrative event from scene");
        self.repository
            .untie_from_scene(event_id)
            .await
            .context("Failed to untie narrative event from scene")
    }

    // =========================================================================
    // TIED_TO_LOCATION Edge Methods
    // =========================================================================

    #[instrument(skip(self))]
    async fn tie_to_location(&self, event_id: NarrativeEventId, location_id: LocationId) -> Result<bool> {
        debug!(event_id = %event_id, location_id = %location_id, "Tying narrative event to location");
        self.repository
            .tie_to_location(event_id, location_id)
            .await
            .context("Failed to tie narrative event to location")
    }

    #[instrument(skip(self))]
    async fn get_tied_location(&self, event_id: NarrativeEventId) -> Result<Option<LocationId>> {
        debug!(event_id = %event_id, "Getting tied location for narrative event");
        self.repository
            .get_tied_location(event_id)
            .await
            .context("Failed to get tied location for narrative event")
    }

    #[instrument(skip(self))]
    async fn untie_from_location(&self, event_id: NarrativeEventId) -> Result<bool> {
        debug!(event_id = %event_id, "Untying narrative event from location");
        self.repository
            .untie_from_location(event_id)
            .await
            .context("Failed to untie narrative event from location")
    }

    // =========================================================================
    // BELONGS_TO_ACT Edge Methods
    // =========================================================================

    #[instrument(skip(self))]
    async fn assign_to_act(&self, event_id: NarrativeEventId, act_id: ActId) -> Result<bool> {
        debug!(event_id = %event_id, act_id = %act_id, "Assigning narrative event to act");
        self.repository
            .assign_to_act(event_id, act_id)
            .await
            .context("Failed to assign narrative event to act")
    }

    #[instrument(skip(self))]
    async fn get_act(&self, event_id: NarrativeEventId) -> Result<Option<ActId>> {
        debug!(event_id = %event_id, "Getting act for narrative event");
        self.repository
            .get_act(event_id)
            .await
            .context("Failed to get act for narrative event")
    }

    #[instrument(skip(self))]
    async fn unassign_from_act(&self, event_id: NarrativeEventId) -> Result<bool> {
        debug!(event_id = %event_id, "Unassigning narrative event from act");
        self.repository
            .unassign_from_act(event_id)
            .await
            .context("Failed to unassign narrative event from act")
    }

    // =========================================================================
    // FEATURES_NPC Edge Methods
    // =========================================================================

    #[instrument(skip(self, featured_npc))]
    async fn add_featured_npc(&self, event_id: NarrativeEventId, featured_npc: FeaturedNpc) -> Result<bool> {
        debug!(event_id = %event_id, character_id = %featured_npc.character_id, "Adding featured NPC to narrative event");
        self.repository
            .add_featured_npc(event_id, featured_npc)
            .await
            .context("Failed to add featured NPC to narrative event")
    }

    #[instrument(skip(self))]
    async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> Result<Vec<FeaturedNpc>> {
        debug!(event_id = %event_id, "Getting featured NPCs for narrative event");
        self.repository
            .get_featured_npcs(event_id)
            .await
            .context("Failed to get featured NPCs for narrative event")
    }

    #[instrument(skip(self))]
    async fn remove_featured_npc(&self, event_id: NarrativeEventId, character_id: CharacterId) -> Result<bool> {
        debug!(event_id = %event_id, character_id = %character_id, "Removing featured NPC from narrative event");
        self.repository
            .remove_featured_npc(event_id, character_id)
            .await
            .context("Failed to remove featured NPC from narrative event")
    }

    #[instrument(skip(self))]
    async fn update_featured_npc_role(&self, event_id: NarrativeEventId, character_id: CharacterId, role: Option<String>) -> Result<bool> {
        debug!(event_id = %event_id, character_id = %character_id, role = ?role, "Updating featured NPC role");
        self.repository
            .update_featured_npc_role(event_id, character_id, role)
            .await
            .context("Failed to update featured NPC role")
    }

    // =========================================================================
    // Chain Membership Query Methods
    // =========================================================================

    #[instrument(skip(self))]
    async fn get_chain_memberships(&self, event_id: NarrativeEventId) -> Result<Vec<EventChainMembership>> {
        debug!(event_id = %event_id, "Getting chain memberships for narrative event");
        self.repository
            .get_chain_memberships(event_id)
            .await
            .context("Failed to get chain memberships for narrative event")
    }

    // =========================================================================
    // Query Methods for Events by Edge Relationships
    // =========================================================================

    #[instrument(skip(self))]
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<NarrativeEvent>> {
        debug!(scene_id = %scene_id, "Listing narrative events by scene");
        self.repository
            .list_by_scene(scene_id)
            .await
            .context("Failed to list narrative events by scene")
    }

    #[instrument(skip(self))]
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<NarrativeEvent>> {
        debug!(location_id = %location_id, "Listing narrative events by location");
        self.repository
            .list_by_location(location_id)
            .await
            .context("Failed to list narrative events by location")
    }

    #[instrument(skip(self))]
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<NarrativeEvent>> {
        debug!(act_id = %act_id, "Listing narrative events by act");
        self.repository
            .list_by_act(act_id)
            .await
            .context("Failed to list narrative events by act")
    }

    #[instrument(skip(self))]
    async fn list_by_featured_npc(&self, character_id: CharacterId) -> Result<Vec<NarrativeEvent>> {
        debug!(character_id = %character_id, "Listing narrative events by featured NPC");
        self.repository
            .list_by_featured_npc(character_id)
            .await
            .context("Failed to list narrative events by featured NPC")
    }
}
