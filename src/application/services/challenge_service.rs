//! Challenge Service - Application service for challenge management
//!
//! This service provides use case implementations for creating, updating,
//! and managing challenges within a world.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::ChallengeRepositoryPort;
use crate::domain::entities::Challenge;
use crate::domain::value_objects::{ChallengeId, SceneId, WorldId};

/// Challenge service trait defining the application use cases
#[async_trait]
pub trait ChallengeService: Send + Sync {
    /// Get a challenge by ID
    async fn get_challenge(&self, id: ChallengeId) -> Result<Option<Challenge>>;

    /// List all challenges for a world
    async fn list_challenges(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List active challenges for a world (for LLM context)
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List favorite challenges for quick access
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List challenges for a specific scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>>;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would use a mock repository implementation
    // For now, these are placeholder tests to show the structure
}
