//! Workflow configuration CRUD service
//!
//! This service handles persistence operations for workflow configurations.
//! It provides a clean interface between the HTTP layer and the repository.

use std::sync::Arc;

use anyhow::Result;

use crate::application::ports::outbound::WorkflowRepositoryPort;
use crate::domain::entities::{WorkflowConfiguration, WorkflowSlot};

/// Service for managing workflow configuration persistence
pub struct WorkflowConfigService {
    repository: Arc<dyn WorkflowRepositoryPort>,
}

impl WorkflowConfigService {
    /// Create a new workflow configuration service
    pub fn new(repository: Arc<dyn WorkflowRepositoryPort>) -> Self {
        Self { repository }
    }

    /// Get a workflow configuration by slot
    pub async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>> {
        self.repository.get_by_slot(slot).await
    }

    /// Save a workflow configuration (create or update)
    pub async fn save(&self, config: &WorkflowConfiguration) -> Result<()> {
        self.repository.save(config).await
    }

    /// Delete a workflow configuration by slot
    pub async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool> {
        self.repository.delete_by_slot(slot).await
    }

    /// List all workflow configurations
    pub async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>> {
        self.repository.list_all().await
    }
}
