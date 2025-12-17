//! Asset management and generation services

use std::sync::Arc;

use crate::application::services::generation_service::GenerationService;
use crate::application::services::{AssetServiceImpl, GenerationQueueProjectionService, WorkflowConfigService};

/// Services for managing assets, workflows, and generation
///
/// This struct groups services related to asset management, workflow configuration,
/// and the generation pipeline for creating game assets.
pub struct AssetServices {
    pub asset_service: AssetServiceImpl,
    pub workflow_config_service: WorkflowConfigService,
    #[allow(dead_code)] // Kept for potential future direct generation access (currently event-driven via queue)
    pub generation_service: Arc<GenerationService>,
    pub generation_queue_projection_service: Arc<GenerationQueueProjectionService>,
}

impl AssetServices {
    /// Creates a new AssetServices instance with all asset-related services
    pub fn new(
        asset_service: AssetServiceImpl,
        workflow_config_service: WorkflowConfigService,
        generation_service: Arc<GenerationService>,
        generation_queue_projection_service: Arc<GenerationQueueProjectionService>,
    ) -> Self {
        Self {
            asset_service,
            workflow_config_service,
            generation_service,
            generation_queue_projection_service,
        }
    }
}
