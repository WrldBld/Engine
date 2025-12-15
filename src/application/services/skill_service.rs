//! Skill Service - Application service for skill management
//!
//! This service provides use case implementations for creating, updating,
//! and managing skills within a world, including initialization of default
//! skills based on the world's rule system.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::{SkillRepositoryPort, WorldRepositoryPort};
use crate::domain::entities::{default_skills_for_variant, Skill, SkillCategory};
use crate::domain::value_objects::{SkillId, WorldId};

/// Request to create a new skill
#[derive(Debug, Clone)]
pub struct CreateSkillRequest {
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub base_attribute: Option<String>,
}

/// Request to update an existing skill
#[derive(Debug, Clone)]
pub struct UpdateSkillRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<SkillCategory>,
    pub base_attribute: Option<String>,
    pub is_hidden: Option<bool>,
    pub order: Option<u32>,
}

/// Skill service trait defining the application use cases
#[async_trait]
pub trait SkillService: Send + Sync {
    /// List all skills for a world
    /// If the world has no skills yet, returns the default skills for the world's rule system
    async fn list_skills(&self, world_id: WorldId) -> Result<Vec<Skill>>;

    /// Get a single skill by ID
    async fn get_skill(&self, skill_id: SkillId) -> Result<Option<Skill>>;

    /// Create a custom skill for a world
    async fn create_skill(&self, world_id: WorldId, request: CreateSkillRequest) -> Result<Skill>;

    /// Update a skill
    async fn update_skill(&self, skill_id: SkillId, request: UpdateSkillRequest) -> Result<Skill>;

    /// Update skill visibility
    async fn update_visibility(&self, skill_id: SkillId, is_hidden: bool) -> Result<Skill>;

    /// Delete a custom skill
    async fn delete_skill(&self, skill_id: SkillId) -> Result<()>;

    /// Initialize default skills for a world based on its rule system
    async fn initialize_defaults(&self, world_id: WorldId) -> Result<Vec<Skill>>;
}

/// Default implementation of SkillService using port abstractions
#[derive(Clone)]
pub struct SkillServiceImpl {
    skill_repository: Arc<dyn SkillRepositoryPort>,
    world_repository: Arc<dyn WorldRepositoryPort>,
}

impl SkillServiceImpl {
    /// Create a new SkillServiceImpl with the given repositories
    pub fn new(
        skill_repository: Arc<dyn SkillRepositoryPort>,
        world_repository: Arc<dyn WorldRepositoryPort>,
    ) -> Self {
        Self {
            skill_repository,
            world_repository,
        }
    }

    /// Validate a skill creation request
    fn validate_create_request(request: &CreateSkillRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            anyhow::bail!("Skill name cannot be empty");
        }
        if request.name.len() > 255 {
            anyhow::bail!("Skill name cannot exceed 255 characters");
        }
        if request.description.len() > 2000 {
            anyhow::bail!("Skill description cannot exceed 2000 characters");
        }
        Ok(())
    }
}

#[async_trait]
impl SkillService for SkillServiceImpl {
    #[instrument(skip(self))]
    async fn list_skills(&self, world_id: WorldId) -> Result<Vec<Skill>> {
        debug!(world_id = %world_id, "Listing skills for world");

        // Get the world to check its rule system
        let world = self
            .world_repository
            .get(world_id)
            .await
            .context("Failed to get world from repository")?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", world_id))?;

        // Try to get skills from repository
        let skills = self
            .skill_repository
            .list(world_id)
            .await
            .context("Failed to list skills from repository")?;

        // If no skills exist, generate default skills based on rule system variant
        let skills = if skills.is_empty() {
            debug!(
                world_id = %world_id,
                variant = ?world.rule_system.variant,
                "No skills found, generating defaults"
            );
            default_skills_for_variant(world_id, &world.rule_system.variant)
        } else {
            skills
        };

        Ok(skills)
    }

    #[instrument(skip(self))]
    async fn get_skill(&self, skill_id: SkillId) -> Result<Option<Skill>> {
        debug!(skill_id = %skill_id, "Fetching skill");
        self.skill_repository
            .get(skill_id)
            .await
            .context("Failed to get skill from repository")
    }

    #[instrument(skip(self), fields(world_id = %world_id, skill_name = %request.name))]
    async fn create_skill(&self, world_id: WorldId, request: CreateSkillRequest) -> Result<Skill> {
        Self::validate_create_request(&request)?;

        // Verify world exists
        let _ = self
            .world_repository
            .get(world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", world_id))?;

        // Create the custom skill
        let mut skill = Skill::custom(world_id, &request.name, request.category)
            .with_description(&request.description);

        if let Some(attr) = request.base_attribute {
            skill = skill.with_base_attribute(attr);
        }

        // Save to repository
        self.skill_repository
            .create(&skill)
            .await
            .context("Failed to create skill in repository")?;

        info!(skill_id = %skill.id, "Created custom skill: {}", skill.name);
        Ok(skill)
    }

    #[instrument(skip(self), fields(skill_id = %skill_id))]
    async fn update_skill(&self, skill_id: SkillId, request: UpdateSkillRequest) -> Result<Skill> {
        // Get existing skill
        let mut skill = self
            .skill_repository
            .get(skill_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_id))?;

        // Apply updates
        if let Some(name) = request.name {
            if name.trim().is_empty() {
                anyhow::bail!("Skill name cannot be empty");
            }
            if name.len() > 255 {
                anyhow::bail!("Skill name cannot exceed 255 characters");
            }
            skill.name = name;
        }
        if let Some(description) = request.description {
            if description.len() > 2000 {
                anyhow::bail!("Skill description cannot exceed 2000 characters");
            }
            skill.description = description;
        }
        if let Some(category) = request.category {
            skill.category = category;
        }
        if let Some(base_attribute) = request.base_attribute {
            skill.base_attribute = Some(base_attribute);
        }
        if let Some(is_hidden) = request.is_hidden {
            skill.is_hidden = is_hidden;
        }
        if let Some(order) = request.order {
            skill.order = order;
        }

        // Save updates
        self.skill_repository
            .update(&skill)
            .await
            .context("Failed to update skill in repository")?;

        info!(skill_id = %skill_id, "Updated skill: {}", skill.name);
        Ok(skill)
    }

    #[instrument(skip(self), fields(skill_id = %skill_id, is_hidden = is_hidden))]
    async fn update_visibility(&self, skill_id: SkillId, is_hidden: bool) -> Result<Skill> {
        let mut skill = self
            .skill_repository
            .get(skill_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_id))?;

        skill.is_hidden = is_hidden;

        self.skill_repository
            .update(&skill)
            .await
            .context("Failed to update skill visibility in repository")?;

        info!(skill_id = %skill_id, "Updated skill visibility: {}", is_hidden);
        Ok(skill)
    }

    #[instrument(skip(self))]
    async fn delete_skill(&self, skill_id: SkillId) -> Result<()> {
        // Get the skill to verify it exists and is custom
        let skill = self
            .skill_repository
            .get(skill_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_id))?;

        // Only allow deleting custom skills
        if !skill.is_custom {
            anyhow::bail!("Cannot delete default skills. Hide them instead.");
        }

        // Delete the skill
        self.skill_repository
            .delete(skill_id)
            .await
            .context("Failed to delete skill from repository")?;

        info!(skill_id = %skill_id, "Deleted custom skill: {}", skill.name);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn initialize_defaults(&self, world_id: WorldId) -> Result<Vec<Skill>> {
        debug!(world_id = %world_id, "Initializing default skills for world");

        // Get the world
        let world = self
            .world_repository
            .get(world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", world_id))?;

        // Generate default skills
        let skills = default_skills_for_variant(world_id, &world.rule_system.variant);

        // Save all skills
        for skill in &skills {
            self.skill_repository
                .create(skill)
                .await
                .context("Failed to create default skill in repository")?;
        }

        info!(
            world_id = %world_id,
            count = skills.len(),
            "Initialized {} default skills",
            skills.len()
        );
        Ok(skills)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_skill_request_validation() {
        // Empty name should fail
        let request = CreateSkillRequest {
            name: "".to_string(),
            description: "Test description".to_string(),
            category: SkillCategory::Combat,
            base_attribute: None,
        };
        assert!(SkillServiceImpl::validate_create_request(&request).is_err());

        // Valid request should pass
        let request = CreateSkillRequest {
            name: "Sword Fighting".to_string(),
            description: "The art of combat with a blade".to_string(),
            category: SkillCategory::Combat,
            base_attribute: Some("STR".to_string()),
        };
        assert!(SkillServiceImpl::validate_create_request(&request).is_ok());

        // Too long name should fail
        let request = CreateSkillRequest {
            name: "x".repeat(256),
            description: "Test".to_string(),
            category: SkillCategory::Combat,
            base_attribute: None,
        };
        assert!(SkillServiceImpl::validate_create_request(&request).is_err());

        // Too long description should fail
        let request = CreateSkillRequest {
            name: "Test Skill".to_string(),
            description: "x".repeat(2001),
            category: SkillCategory::Combat,
            base_attribute: None,
        };
        assert!(SkillServiceImpl::validate_create_request(&request).is_err());
    }
}
