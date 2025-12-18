//! World Service - Application service for world management
//!
//! This service provides use case implementations for creating, updating,
//! and managing worlds, including export functionality for Player clients.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::{
    ExportOptions, PlayerWorldSnapshot, WorldExporterPort, WorldRepositoryPort,
};
use crate::application::services::SettingsService;
use crate::domain::entities::{Act, MonomythStage, World};
use crate::domain::value_objects::{AppSettings, RuleSystemConfig, WorldId};

/// Request to create a new world
#[derive(Debug, Clone)]
pub struct CreateWorldRequest {
    pub name: String,
    pub description: String,
    pub rule_system: Option<RuleSystemConfig>,
}

/// Request to update an existing world
#[derive(Debug, Clone)]
pub struct UpdateWorldRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub rule_system: Option<RuleSystemConfig>,
}

/// Request to create a new act within a world
#[derive(Debug, Clone)]
pub struct CreateActRequest {
    pub name: String,
    pub stage: MonomythStage,
    pub description: Option<String>,
    pub order: u32,
}

/// World with its associated acts
#[derive(Debug, Clone)]
pub struct WorldWithActs {
    pub world: World,
    pub acts: Vec<Act>,
}

/// World service trait defining the application use cases
#[async_trait]
pub trait WorldService: Send + Sync {
    /// Create a new world with validation
    async fn create_world(&self, request: CreateWorldRequest) -> Result<World>;

    /// Get a world by ID
    async fn get_world(&self, id: WorldId) -> Result<Option<World>>;

    /// Get a world with all its acts
    async fn get_world_with_acts(&self, id: WorldId) -> Result<Option<WorldWithActs>>;

    /// List all worlds
    async fn list_worlds(&self) -> Result<Vec<World>>;

    /// Update a world
    async fn update_world(&self, id: WorldId, request: UpdateWorldRequest) -> Result<World>;

    /// Delete a world with cascading cleanup of all related entities
    async fn delete_world(&self, id: WorldId) -> Result<()>;

    /// Create an act within a world
    async fn create_act(&self, world_id: WorldId, request: CreateActRequest) -> Result<Act>;

    /// Get all acts for a world
    async fn get_acts(&self, world_id: WorldId) -> Result<Vec<Act>>;

    /// Export a world snapshot for Player clients
    async fn export_world_snapshot(&self, world_id: WorldId) -> Result<PlayerWorldSnapshot>;

    /// Export a world snapshot with options
    async fn export_world_snapshot_with_options(
        &self,
        world_id: WorldId,
        include_inactive_characters: bool,
    ) -> Result<PlayerWorldSnapshot>;
}

/// Default implementation of WorldService using port abstractions
#[derive(Clone)]
pub struct WorldServiceImpl {
    repository: Arc<dyn WorldRepositoryPort>,
    exporter: Arc<dyn WorldExporterPort>,
    settings_service: Arc<SettingsService>,
}

impl WorldServiceImpl {
    /// Create a new WorldServiceImpl with the given repository and exporter
    pub fn new(
        repository: Arc<dyn WorldRepositoryPort>,
        exporter: Arc<dyn WorldExporterPort>,
        settings_service: Arc<SettingsService>,
    ) -> Self {
        Self {
            repository,
            exporter,
            settings_service,
        }
    }

    /// Validate a world creation request using settings
    fn validate_create_request(request: &CreateWorldRequest, settings: &AppSettings) -> Result<()> {
        if request.name.trim().is_empty() {
            anyhow::bail!("World name cannot be empty");
        }
        if request.name.len() > settings.max_name_length {
            anyhow::bail!("World name cannot exceed {} characters", settings.max_name_length);
        }
        if request.description.len() > settings.max_description_length {
            anyhow::bail!("World description cannot exceed {} characters", settings.max_description_length);
        }
        Ok(())
    }

    /// Validate a world update request using settings
    fn validate_update_request(request: &UpdateWorldRequest, settings: &AppSettings) -> Result<()> {
        if let Some(ref name) = request.name {
            if name.trim().is_empty() {
                anyhow::bail!("World name cannot be empty");
            }
            if name.len() > settings.max_name_length {
                anyhow::bail!("World name cannot exceed {} characters", settings.max_name_length);
            }
        }
        if let Some(ref description) = request.description {
            if description.len() > settings.max_description_length {
                anyhow::bail!("World description cannot exceed {} characters", settings.max_description_length);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl WorldService for WorldServiceImpl {
    #[instrument(skip(self), fields(name = %request.name))]
    async fn create_world(&self, request: CreateWorldRequest) -> Result<World> {
        // For world creation, use global settings (no world_id yet)
        let settings = self.settings_service.get().await;
        Self::validate_create_request(&request, &settings)?;

        let mut world = World::new(&request.name, &request.description);

        if let Some(rule_system) = request.rule_system {
            world = world.with_rule_system(rule_system);
        }

        self.repository
            .create(&world)
            .await
            .context("Failed to create world in repository")?;

        info!(world_id = %world.id, "Created new world: {}", world.name);
        Ok(world)
    }

    #[instrument(skip(self))]
    async fn get_world(&self, id: WorldId) -> Result<Option<World>> {
        debug!(world_id = %id, "Fetching world");
        self.repository
            .get(id)
            .await
            .context("Failed to get world from repository")
    }

    #[instrument(skip(self))]
    async fn get_world_with_acts(&self, id: WorldId) -> Result<Option<WorldWithActs>> {
        debug!(world_id = %id, "Fetching world with acts");

        let world = match self.repository.get(id).await? {
            Some(w) => w,
            None => return Ok(None),
        };

        let acts = self
            .repository
            .get_acts(id)
            .await
            .context("Failed to get acts for world")?;

        Ok(Some(WorldWithActs { world, acts }))
    }

    #[instrument(skip(self))]
    async fn list_worlds(&self) -> Result<Vec<World>> {
        debug!("Listing all worlds");
        self.repository
            .list()
            .await
            .context("Failed to list worlds from repository")
    }

    #[instrument(skip(self), fields(world_id = %id))]
    async fn update_world(&self, id: WorldId, request: UpdateWorldRequest) -> Result<World> {
        // Get per-world settings for validation
        let settings = self.settings_service.get_for_world(id).await;
        Self::validate_update_request(&request, &settings)?;

        let mut world = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", id))?;

        if let Some(name) = request.name {
            world.update_name(name);
        }
        if let Some(description) = request.description {
            world.update_description(description);
        }
        if let Some(rule_system) = request.rule_system {
            world.rule_system = rule_system;
            world.updated_at = chrono::Utc::now();
        }

        self.repository
            .update(&world)
            .await
            .context("Failed to update world in repository")?;

        info!(world_id = %id, "Updated world: {}", world.name);
        Ok(world)
    }

    #[instrument(skip(self))]
    async fn delete_world(&self, id: WorldId) -> Result<()> {
        // Verify the world exists before deletion
        let world = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", id))?;

        // The repository handles cascading deletion
        self.repository
            .delete(id)
            .await
            .context("Failed to delete world from repository")?;

        info!(world_id = %id, "Deleted world: {}", world.name);
        Ok(())
    }

    #[instrument(skip(self), fields(world_id = %world_id, act_name = %request.name))]
    async fn create_act(&self, world_id: WorldId, request: CreateActRequest) -> Result<Act> {
        // Verify the world exists
        let _ = self
            .repository
            .get(world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", world_id))?;

        let mut act = Act::new(world_id, &request.name, request.stage, request.order);

        if let Some(description) = request.description {
            act = act.with_description(description);
        }

        self.repository
            .create_act(&act)
            .await
            .context("Failed to create act in repository")?;

        info!(act_id = %act.id, "Created act: {} in world {}", act.name, world_id);
        Ok(act)
    }

    #[instrument(skip(self))]
    async fn get_acts(&self, world_id: WorldId) -> Result<Vec<Act>> {
        debug!(world_id = %world_id, "Fetching acts for world");
        self.repository
            .get_acts(world_id)
            .await
            .context("Failed to get acts from repository")
    }

    #[instrument(skip(self))]
    async fn export_world_snapshot(&self, world_id: WorldId) -> Result<PlayerWorldSnapshot> {
        debug!(world_id = %world_id, "Exporting world snapshot");

        self.exporter
            .export_snapshot(world_id)
            .await
            .context("Failed to export world snapshot")
    }

    #[instrument(skip(self))]
    async fn export_world_snapshot_with_options(
        &self,
        world_id: WorldId,
        include_inactive_characters: bool,
    ) -> Result<PlayerWorldSnapshot> {
        debug!(
            world_id = %world_id,
            include_inactive = include_inactive_characters,
            "Exporting world snapshot with options"
        );

        let options = ExportOptions {
            current_scene_id: None,
            include_inactive_characters,
        };

        self.exporter
            .export_snapshot_with_options(world_id, options)
            .await
            .context("Failed to export world snapshot")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_world_request_validation() {
        let settings = AppSettings::default();

        // Empty name should fail
        let request = CreateWorldRequest {
            name: "".to_string(),
            description: "Test description".to_string(),
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_create_request(&request, &settings).is_err());

        // Valid request should pass
        let request = CreateWorldRequest {
            name: "Test World".to_string(),
            description: "A test world".to_string(),
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_create_request(&request, &settings).is_ok());

        // Too long name should fail (256 > 255 default max)
        let request = CreateWorldRequest {
            name: "x".repeat(256),
            description: "Test".to_string(),
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_create_request(&request, &settings).is_err());
    }

    #[test]
    fn test_update_world_request_validation() {
        let settings = AppSettings::default();

        // Empty name should fail
        let request = UpdateWorldRequest {
            name: Some("".to_string()),
            description: None,
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_update_request(&request, &settings).is_err());

        // No updates is valid
        let request = UpdateWorldRequest {
            name: None,
            description: None,
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_update_request(&request, &settings).is_ok());
    }
}
