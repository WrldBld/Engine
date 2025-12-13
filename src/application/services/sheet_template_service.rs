//! Sheet Template Service
//!
//! Application service for managing character sheet templates.
//! This service wraps the SheetTemplateRepositoryPort and provides
//! the business logic layer for sheet template operations.

use anyhow::Result;
use std::sync::Arc;

use crate::application::ports::outbound::SheetTemplateRepositoryPort;
use crate::domain::entities::{CharacterSheetTemplate, SheetTemplateId};
use crate::domain::value_objects::WorldId;

/// Service for managing character sheet templates
pub struct SheetTemplateService {
    repository: Arc<dyn SheetTemplateRepositoryPort>,
}

impl SheetTemplateService {
    /// Create a new sheet template service
    pub fn new(repository: Arc<dyn SheetTemplateRepositoryPort>) -> Self {
        Self { repository }
    }

    /// Create a new sheet template
    pub async fn create(&self, template: &CharacterSheetTemplate) -> Result<()> {
        self.repository.create(template).await
    }

    /// Get a sheet template by ID
    pub async fn get(&self, id: &SheetTemplateId) -> Result<Option<CharacterSheetTemplate>> {
        self.repository.get(id).await
    }

    /// Get the default template for a world
    pub async fn get_default_for_world(
        &self,
        world_id: &WorldId,
    ) -> Result<Option<CharacterSheetTemplate>> {
        self.repository.get_default_for_world(world_id).await
    }

    /// List all templates for a world
    pub async fn list_by_world(&self, world_id: &WorldId) -> Result<Vec<CharacterSheetTemplate>> {
        self.repository.list_by_world(world_id).await
    }

    /// Update a sheet template
    pub async fn update(&self, template: &CharacterSheetTemplate) -> Result<()> {
        self.repository.update(template).await
    }

    /// Delete a sheet template
    pub async fn delete(&self, id: &SheetTemplateId) -> Result<()> {
        self.repository.delete(id).await
    }

    /// Delete all templates for a world
    pub async fn delete_all_for_world(&self, world_id: &WorldId) -> Result<()> {
        self.repository.delete_all_for_world(world_id).await
    }

    /// Check if a world has any templates
    pub async fn has_templates(&self, world_id: &WorldId) -> Result<bool> {
        self.repository.has_templates(world_id).await
    }
}
