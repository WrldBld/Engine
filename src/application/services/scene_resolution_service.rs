//! Scene Resolution Service - Resolves scenes based on player character locations
//!
//! This service determines which scene to show based on where player characters
//! are located in the world. It handles single-location and split-party scenarios.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::{
    PlayerCharacterRepositoryPort, SceneRepositoryPort,
};
use crate::domain::entities::Scene;
use crate::domain::value_objects::{
    LocationId, PlayerCharacterId, SessionId,
};

/// Result of scene resolution
#[derive(Debug, Clone)]
pub struct SceneResolutionResult {
    /// The resolved scene (if any)
    pub scene: Option<Scene>,
    /// Whether the party is split across multiple locations
    pub is_split_party: bool,
    /// Locations where PCs are located (for split party scenarios)
    pub pc_locations: Vec<LocationId>,
}

/// Scene resolution service trait
#[async_trait]
pub trait SceneResolutionService: Send + Sync {
    /// Resolve the scene for a session based on PC locations
    async fn resolve_scene_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<SceneResolutionResult>;

    /// Resolve the scene for a specific player character
    async fn resolve_scene_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<Scene>>;
}

/// Default implementation of SceneResolutionService
#[derive(Clone)]
pub struct SceneResolutionServiceImpl {
    pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
    scene_repository: Arc<dyn SceneRepositoryPort>,
}

impl SceneResolutionServiceImpl {
    /// Create a new SceneResolutionServiceImpl with the given repositories
    pub fn new(
        pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
        scene_repository: Arc<dyn SceneRepositoryPort>,
    ) -> Self {
        Self {
            pc_repository,
            scene_repository,
        }
    }
}

#[async_trait]
impl SceneResolutionService for SceneResolutionServiceImpl {
    #[instrument(skip(self), fields(session_id = %session_id))]
    async fn resolve_scene_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<SceneResolutionResult> {
        // Get all PCs in the session
        let pcs = self
            .pc_repository
            .get_by_session(session_id)
            .await
            .context("Failed to get player characters for session")?;

        if pcs.is_empty() {
            debug!(session_id = %session_id, "No player characters in session, no scene to resolve");
            return Ok(SceneResolutionResult {
                scene: None,
                is_split_party: false,
                pc_locations: Vec::new(),
            });
        }

        // Group PCs by location
        let mut location_pcs: std::collections::HashMap<LocationId, Vec<_>> = std::collections::HashMap::new();
        for pc in &pcs {
            location_pcs
                .entry(pc.current_location_id)
                .or_insert_with(Vec::new)
                .push(pc);
        }

        let unique_locations: Vec<LocationId> = location_pcs.keys().cloned().collect();

        // If all PCs are at the same location
        if unique_locations.len() == 1 {
            let location_id = unique_locations[0];
            debug!(
                session_id = %session_id,
                location_id = %location_id,
                "All PCs at same location, resolving scene"
            );

            // Find active scene at that location
            let scenes = self
                .scene_repository
                .list_by_location(location_id)
                .await
                .context("Failed to list scenes by location")?;

            // Pick the first scene (could be enhanced to check entry conditions, order, etc.)
            let scene = scenes.into_iter().next();

            Ok(SceneResolutionResult {
                scene,
                is_split_party: false,
                pc_locations: unique_locations,
            })
        } else {
            // Party is split across multiple locations
            info!(
                session_id = %session_id,
                location_count = unique_locations.len(),
                "Party is split across {} locations",
                unique_locations.len()
            );

            // For split party, we could:
            // 1. Return None and let DM choose
            // 2. Return the first PC's location scene
            // 3. Return a special "split party" scene

            // For now, return the scene at the first location (first PC's location)
            let first_location = unique_locations[0];
            let scenes = self
                .scene_repository
                .list_by_location(first_location)
                .await
                .context("Failed to list scenes by location")?;

            let scene = scenes.into_iter().next();

            Ok(SceneResolutionResult {
                scene,
                is_split_party: true,
                pc_locations: unique_locations,
            })
        }
    }

    #[instrument(skip(self), fields(pc_id = %pc_id))]
    async fn resolve_scene_for_pc(&self, pc_id: PlayerCharacterId) -> Result<Option<Scene>> {
        // Get the PC
        let pc = self
            .pc_repository
            .get(pc_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", pc_id))?;

        // Find active scene at PC's location
        let scenes = self
            .scene_repository
            .list_by_location(pc.current_location_id)
            .await
            .context("Failed to list scenes by location")?;

        // Pick the first scene (could be enhanced to check entry conditions, order, etc.)
        Ok(scenes.into_iter().next())
    }
}

