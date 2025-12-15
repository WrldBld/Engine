//! Interaction Service - Application service for interaction management
//!
//! This service provides use case implementations for creating, updating,
//! and managing interaction templates within scenes.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::InteractionRepositoryPort;
use crate::domain::entities::InteractionTemplate;
use crate::domain::value_objects::{InteractionId, SceneId};

/// Interaction service trait defining the application use cases
#[async_trait]
pub trait InteractionService: Send + Sync {
    /// List all interactions for a scene
    async fn list_interactions(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>>;

    /// Get a single interaction by ID
    async fn get_interaction(&self, id: InteractionId) -> Result<Option<InteractionTemplate>>;

    /// Create a new interaction in a scene
    async fn create_interaction(&self, interaction: &InteractionTemplate) -> Result<()>;

    /// Update an existing interaction
    async fn update_interaction(&self, interaction: &InteractionTemplate) -> Result<()>;

    /// Delete an interaction
    async fn delete_interaction(&self, id: InteractionId) -> Result<()>;

    /// Set availability of an interaction
    async fn set_interaction_availability(&self, id: InteractionId, available: bool) -> Result<()>;
}

/// Default implementation of InteractionService using port abstractions
#[derive(Clone)]
pub struct InteractionServiceImpl {
    repository: Arc<dyn InteractionRepositoryPort>,
}

impl InteractionServiceImpl {
    /// Create a new InteractionServiceImpl with the given repository
    pub fn new(repository: Arc<dyn InteractionRepositoryPort>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl InteractionService for InteractionServiceImpl {
    #[instrument(skip(self))]
    async fn list_interactions(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>> {
        debug!(scene_id = %scene_id, "Listing interactions for scene");
        self.repository
            .list_by_scene(scene_id)
            .await
            .context("Failed to list interactions from repository")
    }

    #[instrument(skip(self))]
    async fn get_interaction(&self, id: InteractionId) -> Result<Option<InteractionTemplate>> {
        debug!(interaction_id = %id, "Fetching interaction");
        self.repository
            .get(id)
            .await
            .context("Failed to get interaction from repository")
    }

    #[instrument(skip(self, interaction), fields(interaction_id = %interaction.id, scene_id = %interaction.scene_id))]
    async fn create_interaction(&self, interaction: &InteractionTemplate) -> Result<()> {
        self.repository
            .create(interaction)
            .await
            .context("Failed to create interaction in repository")?;

        info!(
            interaction_id = %interaction.id,
            "Created interaction: {} in scene {}",
            interaction.name,
            interaction.scene_id
        );
        Ok(())
    }

    #[instrument(skip(self, interaction), fields(interaction_id = %interaction.id))]
    async fn update_interaction(&self, interaction: &InteractionTemplate) -> Result<()> {
        self.repository
            .update(interaction)
            .await
            .context("Failed to update interaction in repository")?;

        info!(interaction_id = %interaction.id, "Updated interaction: {}", interaction.name);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_interaction(&self, id: InteractionId) -> Result<()> {
        self.repository
            .delete(id)
            .await
            .context("Failed to delete interaction from repository")?;

        info!(interaction_id = %id, "Deleted interaction");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn set_interaction_availability(&self, id: InteractionId, available: bool) -> Result<()> {
        // Note: The repository implementation needs to implement this method
        // For now, we'll fetch, modify, and update
        let mut interaction = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Interaction not found: {}", id))?;

        interaction.is_available = available;

        self.repository
            .update(&interaction)
            .await
            .context("Failed to update interaction availability")?;

        info!(
            interaction_id = %id,
            available = available,
            "Updated interaction availability"
        );
        Ok(())
    }
}
