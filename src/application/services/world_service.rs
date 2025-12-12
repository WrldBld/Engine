//! World Service - Application service for world management
//!
//! This service provides use case implementations for creating, updating,
//! and managing worlds, including export functionality for Player clients.

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::WorldRepositoryPort;
use crate::domain::entities::{Act, MonomythStage, World};
use crate::domain::value_objects::{RuleSystemConfig, WorldId};

// TODO: These infrastructure imports should be moved behind a port trait
// This is a known architecture violation that will be addressed in Phase 3
use crate::infrastructure::export::{PlayerWorldSnapshot, WorldSnapshotBuilder};
use crate::infrastructure::persistence::Neo4jRepository;

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

/// Default implementation of WorldService using Neo4j repository
pub struct WorldServiceImpl {
    repository: Neo4jRepository,
}

impl WorldServiceImpl {
    /// Create a new WorldServiceImpl with the given repository
    pub fn new(repository: Neo4jRepository) -> Self {
        Self { repository }
    }

    /// Validate a world creation request
    fn validate_create_request(request: &CreateWorldRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            anyhow::bail!("World name cannot be empty");
        }
        if request.name.len() > 255 {
            anyhow::bail!("World name cannot exceed 255 characters");
        }
        if request.description.len() > 10000 {
            anyhow::bail!("World description cannot exceed 10000 characters");
        }
        Ok(())
    }

    /// Validate a world update request
    fn validate_update_request(request: &UpdateWorldRequest) -> Result<()> {
        if let Some(ref name) = request.name {
            if name.trim().is_empty() {
                anyhow::bail!("World name cannot be empty");
            }
            if name.len() > 255 {
                anyhow::bail!("World name cannot exceed 255 characters");
            }
        }
        if let Some(ref description) = request.description {
            if description.len() > 10000 {
                anyhow::bail!("World description cannot exceed 10000 characters");
            }
        }
        Ok(())
    }
}

#[async_trait]
impl WorldService for WorldServiceImpl {
    #[instrument(skip(self), fields(name = %request.name))]
    async fn create_world(&self, request: CreateWorldRequest) -> Result<World> {
        Self::validate_create_request(&request)?;

        let mut world = World::new(&request.name, &request.description);

        if let Some(rule_system) = request.rule_system {
            world = world.with_rule_system(rule_system);
        }

        self.repository
            .worlds()
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
            .worlds()
            .get(id)
            .await
            .context("Failed to get world from repository")
    }

    #[instrument(skip(self))]
    async fn get_world_with_acts(&self, id: WorldId) -> Result<Option<WorldWithActs>> {
        debug!(world_id = %id, "Fetching world with acts");

        let world = match self.repository.worlds().get(id).await? {
            Some(w) => w,
            None => return Ok(None),
        };

        let acts = self
            .repository
            .worlds()
            .get_acts(id)
            .await
            .context("Failed to get acts for world")?;

        Ok(Some(WorldWithActs { world, acts }))
    }

    #[instrument(skip(self))]
    async fn list_worlds(&self) -> Result<Vec<World>> {
        debug!("Listing all worlds");
        self.repository
            .worlds()
            .list()
            .await
            .context("Failed to list worlds from repository")
    }

    #[instrument(skip(self), fields(world_id = %id))]
    async fn update_world(&self, id: WorldId, request: UpdateWorldRequest) -> Result<World> {
        Self::validate_update_request(&request)?;

        let mut world = self
            .repository
            .worlds()
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
            .worlds()
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
            .worlds()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", id))?;

        // The repository handles cascading deletion
        self.repository
            .worlds()
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
            .worlds()
            .get(world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", world_id))?;

        let mut act = Act::new(world_id, &request.name, request.stage, request.order);

        if let Some(description) = request.description {
            act = act.with_description(description);
        }

        self.repository
            .worlds()
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
            .worlds()
            .get_acts(world_id)
            .await
            .context("Failed to get acts from repository")
    }

    #[instrument(skip(self))]
    async fn export_world_snapshot(&self, world_id: WorldId) -> Result<PlayerWorldSnapshot> {
        debug!(world_id = %world_id, "Exporting world snapshot");

        WorldSnapshotBuilder::new(world_id, &self.repository)
            .build()
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

        let mut builder = WorldSnapshotBuilder::new(world_id, &self.repository);

        if include_inactive_characters {
            builder = builder.include_inactive_characters();
        }

        builder
            .build()
            .await
            .context("Failed to export world snapshot")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_world_request_validation() {
        // Empty name should fail
        let request = CreateWorldRequest {
            name: "".to_string(),
            description: "Test description".to_string(),
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_create_request(&request).is_err());

        // Valid request should pass
        let request = CreateWorldRequest {
            name: "Test World".to_string(),
            description: "A test world".to_string(),
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_create_request(&request).is_ok());

        // Too long name should fail
        let request = CreateWorldRequest {
            name: "x".repeat(256),
            description: "Test".to_string(),
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_create_request(&request).is_err());
    }

    #[test]
    fn test_update_world_request_validation() {
        // Empty name should fail
        let request = UpdateWorldRequest {
            name: Some("".to_string()),
            description: None,
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_update_request(&request).is_err());

        // No updates is valid
        let request = UpdateWorldRequest {
            name: None,
            description: None,
            rule_system: None,
        };
        assert!(WorldServiceImpl::validate_update_request(&request).is_ok());
    }
}
