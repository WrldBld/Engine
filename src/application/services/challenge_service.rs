//! Challenge Service - Application service for challenge management
//!
//! This service provides use case implementations for creating, updating,
//! and managing challenges within a world.
//!
//! ## Graph-First Design (Phase 0.E)
//!
//! Challenge relationships are stored as Neo4j edges:
//! - `REQUIRES_SKILL` -> Skill required for this challenge
//! - `TIED_TO_SCENE` -> Scene this challenge appears in
//! - `REQUIRES_COMPLETION_OF` -> Prerequisite challenges
//! - `AVAILABLE_AT` -> Locations where challenge is available
//! - `ON_SUCCESS_UNLOCKS` -> Locations unlocked on success

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::ChallengeRepositoryPort;
use crate::domain::entities::{Challenge, ChallengeLocationAvailability, ChallengePrerequisite};
use crate::domain::value_objects::{ChallengeId, LocationId, SceneId, SkillId, WorldId};

/// Challenge service trait defining the application use cases
#[async_trait]
pub trait ChallengeService: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    /// Get a challenge by ID
    async fn get_challenge(&self, id: ChallengeId) -> Result<Option<Challenge>>;

    /// List all challenges for a world
    async fn list_challenges(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List active challenges for a world (for LLM context)
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List favorite challenges for quick access
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List challenges for a specific scene (via TIED_TO_SCENE edge)
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>>;

    /// List challenges available at a location (via AVAILABLE_AT edge)
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Challenge>>;

    /// Create a new challenge
    async fn create_challenge(&self, challenge: Challenge) -> Result<Challenge>;

    /// Update an existing challenge
    async fn update_challenge(&self, challenge: Challenge) -> Result<Challenge>;

    /// Delete a challenge
    async fn delete_challenge(&self, id: ChallengeId) -> Result<()>;

    /// Toggle favorite status for a challenge
    async fn toggle_favorite(&self, id: ChallengeId) -> Result<bool>;

    /// Set active status for a challenge
    async fn set_active(&self, id: ChallengeId, active: bool) -> Result<()>;

    // -------------------------------------------------------------------------
    // Skill Edge (REQUIRES_SKILL)
    // -------------------------------------------------------------------------

    /// Set the required skill for a challenge
    async fn set_required_skill(&self, challenge_id: ChallengeId, skill_id: SkillId) -> Result<()>;

    /// Get the required skill for a challenge
    async fn get_required_skill(&self, challenge_id: ChallengeId) -> Result<Option<SkillId>>;

    /// Remove the required skill from a challenge
    async fn remove_required_skill(&self, challenge_id: ChallengeId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Scene Edge (TIED_TO_SCENE)
    // -------------------------------------------------------------------------

    /// Tie a challenge to a scene
    async fn tie_to_scene(&self, challenge_id: ChallengeId, scene_id: SceneId) -> Result<()>;

    /// Get the scene a challenge is tied to
    async fn get_tied_scene(&self, challenge_id: ChallengeId) -> Result<Option<SceneId>>;

    /// Remove the scene tie from a challenge
    async fn untie_from_scene(&self, challenge_id: ChallengeId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Prerequisite Edges (REQUIRES_COMPLETION_OF)
    // -------------------------------------------------------------------------

    /// Add a prerequisite challenge
    async fn add_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite: ChallengePrerequisite,
    ) -> Result<()>;

    /// Get all prerequisites for a challenge
    async fn get_prerequisites(&self, challenge_id: ChallengeId) -> Result<Vec<ChallengePrerequisite>>;

    /// Remove a prerequisite from a challenge
    async fn remove_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite_id: ChallengeId,
    ) -> Result<()>;

    // -------------------------------------------------------------------------
    // Location Availability Edges (AVAILABLE_AT)
    // -------------------------------------------------------------------------

    /// Add a location where this challenge is available
    async fn add_location_availability(
        &self,
        challenge_id: ChallengeId,
        availability: ChallengeLocationAvailability,
    ) -> Result<()>;

    /// Get all locations where a challenge is available
    async fn get_location_availabilities(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeLocationAvailability>>;

    /// Remove a location availability from a challenge
    async fn remove_location_availability(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()>;

    // -------------------------------------------------------------------------
    // Unlock Edges (ON_SUCCESS_UNLOCKS)
    // -------------------------------------------------------------------------

    /// Add a location that gets unlocked on successful challenge completion
    async fn add_unlock_location(&self, challenge_id: ChallengeId, location_id: LocationId) -> Result<()>;

    /// Get locations that get unlocked when this challenge succeeds
    async fn get_unlock_locations(&self, challenge_id: ChallengeId) -> Result<Vec<LocationId>>;

    /// Remove an unlock from a challenge
    async fn remove_unlock_location(&self, challenge_id: ChallengeId, location_id: LocationId) -> Result<()>;
}

/// Default implementation of ChallengeService using port abstractions
#[derive(Clone)]
pub struct ChallengeServiceImpl {
    repository: Arc<dyn ChallengeRepositoryPort>,
}

impl ChallengeServiceImpl {
    /// Create a new ChallengeServiceImpl with the given repository
    pub fn new(repository: Arc<dyn ChallengeRepositoryPort>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ChallengeService for ChallengeServiceImpl {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    #[instrument(skip(self))]
    async fn get_challenge(&self, id: ChallengeId) -> Result<Option<Challenge>> {
        debug!(challenge_id = %id, "Fetching challenge");
        self.repository
            .get(id)
            .await
            .context("Failed to get challenge from repository")
    }

    #[instrument(skip(self))]
    async fn list_challenges(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        debug!(world_id = %world_id, "Listing all challenges for world");
        self.repository
            .list_by_world(world_id)
            .await
            .context("Failed to list challenges from repository")
    }

    #[instrument(skip(self))]
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        debug!(world_id = %world_id, "Listing active challenges for world");
        self.repository
            .list_active(world_id)
            .await
            .context("Failed to list active challenges from repository")
    }

    #[instrument(skip(self))]
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        debug!(world_id = %world_id, "Listing favorite challenges for world");
        self.repository
            .list_favorites(world_id)
            .await
            .context("Failed to list favorite challenges from repository")
    }

    #[instrument(skip(self))]
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>> {
        debug!(scene_id = %scene_id, "Listing challenges for scene");
        self.repository
            .list_by_scene(scene_id)
            .await
            .context("Failed to list challenges by scene from repository")
    }

    #[instrument(skip(self))]
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Challenge>> {
        debug!(location_id = %location_id, "Listing challenges for location");
        self.repository
            .list_by_location(location_id)
            .await
            .context("Failed to list challenges by location from repository")
    }

    #[instrument(skip(self), fields(challenge_name = %challenge.name))]
    async fn create_challenge(&self, challenge: Challenge) -> Result<Challenge> {
        debug!(challenge_id = %challenge.id, "Creating challenge");

        self.repository
            .create(&challenge)
            .await
            .context("Failed to create challenge in repository")?;

        info!(challenge_id = %challenge.id, "Created challenge: {}", challenge.name);
        Ok(challenge)
    }

    #[instrument(skip(self), fields(challenge_id = %challenge.id))]
    async fn update_challenge(&self, challenge: Challenge) -> Result<Challenge> {
        debug!(challenge_id = %challenge.id, "Updating challenge");

        self.repository
            .update(&challenge)
            .await
            .context("Failed to update challenge in repository")?;

        info!(challenge_id = %challenge.id, "Updated challenge: {}", challenge.name);
        Ok(challenge)
    }

    #[instrument(skip(self))]
    async fn delete_challenge(&self, id: ChallengeId) -> Result<()> {
        debug!(challenge_id = %id, "Deleting challenge");

        self.repository
            .delete(id)
            .await
            .context("Failed to delete challenge from repository")?;

        info!(challenge_id = %id, "Deleted challenge");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn toggle_favorite(&self, id: ChallengeId) -> Result<bool> {
        debug!(challenge_id = %id, "Toggling favorite status for challenge");

        let is_favorite = self
            .repository
            .toggle_favorite(id)
            .await
            .context("Failed to toggle favorite status")?;

        info!(challenge_id = %id, is_favorite, "Toggled favorite status");
        Ok(is_favorite)
    }

    #[instrument(skip(self))]
    async fn set_active(&self, id: ChallengeId, active: bool) -> Result<()> {
        debug!(challenge_id = %id, active, "Setting active status for challenge");

        self.repository
            .set_active(id, active)
            .await
            .context("Failed to set active status")?;

        info!(challenge_id = %id, active, "Set active status");
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Skill Edge (REQUIRES_SKILL)
    // -------------------------------------------------------------------------

    #[instrument(skip(self))]
    async fn set_required_skill(&self, challenge_id: ChallengeId, skill_id: SkillId) -> Result<()> {
        debug!(challenge_id = %challenge_id, skill_id = %skill_id, "Setting required skill");
        self.repository
            .set_required_skill(challenge_id, skill_id)
            .await
            .context("Failed to set required skill")
    }

    #[instrument(skip(self))]
    async fn get_required_skill(&self, challenge_id: ChallengeId) -> Result<Option<SkillId>> {
        debug!(challenge_id = %challenge_id, "Getting required skill");
        self.repository
            .get_required_skill(challenge_id)
            .await
            .context("Failed to get required skill")
    }

    #[instrument(skip(self))]
    async fn remove_required_skill(&self, challenge_id: ChallengeId) -> Result<()> {
        debug!(challenge_id = %challenge_id, "Removing required skill");
        self.repository
            .remove_required_skill(challenge_id)
            .await
            .context("Failed to remove required skill")
    }

    // -------------------------------------------------------------------------
    // Scene Edge (TIED_TO_SCENE)
    // -------------------------------------------------------------------------

    #[instrument(skip(self))]
    async fn tie_to_scene(&self, challenge_id: ChallengeId, scene_id: SceneId) -> Result<()> {
        debug!(challenge_id = %challenge_id, scene_id = %scene_id, "Tying challenge to scene");
        self.repository
            .tie_to_scene(challenge_id, scene_id)
            .await
            .context("Failed to tie challenge to scene")
    }

    #[instrument(skip(self))]
    async fn get_tied_scene(&self, challenge_id: ChallengeId) -> Result<Option<SceneId>> {
        debug!(challenge_id = %challenge_id, "Getting tied scene");
        self.repository
            .get_tied_scene(challenge_id)
            .await
            .context("Failed to get tied scene")
    }

    #[instrument(skip(self))]
    async fn untie_from_scene(&self, challenge_id: ChallengeId) -> Result<()> {
        debug!(challenge_id = %challenge_id, "Untying challenge from scene");
        self.repository
            .untie_from_scene(challenge_id)
            .await
            .context("Failed to untie challenge from scene")
    }

    // -------------------------------------------------------------------------
    // Prerequisite Edges (REQUIRES_COMPLETION_OF)
    // -------------------------------------------------------------------------

    #[instrument(skip(self))]
    async fn add_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite: ChallengePrerequisite,
    ) -> Result<()> {
        debug!(challenge_id = %challenge_id, prereq_id = %prerequisite.challenge_id, "Adding prerequisite");
        self.repository
            .add_prerequisite(challenge_id, prerequisite)
            .await
            .context("Failed to add prerequisite")
    }

    #[instrument(skip(self))]
    async fn get_prerequisites(&self, challenge_id: ChallengeId) -> Result<Vec<ChallengePrerequisite>> {
        debug!(challenge_id = %challenge_id, "Getting prerequisites");
        self.repository
            .get_prerequisites(challenge_id)
            .await
            .context("Failed to get prerequisites")
    }

    #[instrument(skip(self))]
    async fn remove_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite_id: ChallengeId,
    ) -> Result<()> {
        debug!(challenge_id = %challenge_id, prereq_id = %prerequisite_id, "Removing prerequisite");
        self.repository
            .remove_prerequisite(challenge_id, prerequisite_id)
            .await
            .context("Failed to remove prerequisite")
    }

    // -------------------------------------------------------------------------
    // Location Availability Edges (AVAILABLE_AT)
    // -------------------------------------------------------------------------

    #[instrument(skip(self))]
    async fn add_location_availability(
        &self,
        challenge_id: ChallengeId,
        availability: ChallengeLocationAvailability,
    ) -> Result<()> {
        debug!(challenge_id = %challenge_id, location_id = %availability.location_id, "Adding location availability");
        self.repository
            .add_location_availability(challenge_id, availability)
            .await
            .context("Failed to add location availability")
    }

    #[instrument(skip(self))]
    async fn get_location_availabilities(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeLocationAvailability>> {
        debug!(challenge_id = %challenge_id, "Getting location availabilities");
        self.repository
            .get_location_availabilities(challenge_id)
            .await
            .context("Failed to get location availabilities")
    }

    #[instrument(skip(self))]
    async fn remove_location_availability(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()> {
        debug!(challenge_id = %challenge_id, location_id = %location_id, "Removing location availability");
        self.repository
            .remove_location_availability(challenge_id, location_id)
            .await
            .context("Failed to remove location availability")
    }

    // -------------------------------------------------------------------------
    // Unlock Edges (ON_SUCCESS_UNLOCKS)
    // -------------------------------------------------------------------------

    #[instrument(skip(self))]
    async fn add_unlock_location(&self, challenge_id: ChallengeId, location_id: LocationId) -> Result<()> {
        debug!(challenge_id = %challenge_id, location_id = %location_id, "Adding unlock location");
        self.repository
            .add_unlock_location(challenge_id, location_id)
            .await
            .context("Failed to add unlock location")
    }

    #[instrument(skip(self))]
    async fn get_unlock_locations(&self, challenge_id: ChallengeId) -> Result<Vec<LocationId>> {
        debug!(challenge_id = %challenge_id, "Getting unlock locations");
        self.repository
            .get_unlock_locations(challenge_id)
            .await
            .context("Failed to get unlock locations")
    }

    #[instrument(skip(self))]
    async fn remove_unlock_location(&self, challenge_id: ChallengeId, location_id: LocationId) -> Result<()> {
        debug!(challenge_id = %challenge_id, location_id = %location_id, "Removing unlock location");
        self.repository
            .remove_unlock_location(challenge_id, location_id)
            .await
            .context("Failed to remove unlock location")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would use a mock repository implementation
    // For now, these are placeholder tests to show the structure
}
