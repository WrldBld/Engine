//! Neo4j persistence adapters
//!
//! This module implements the repository pattern for Neo4j,
//! providing CRUD operations for all domain entities.

mod asset_repository;
mod challenge_repository;
mod character_repository;
mod connection;
mod event_chain_repository;
mod interaction_repository;
mod location_repository;
mod narrative_event_repository;
mod observation_repository;
mod region_repository;
mod player_character_repository;
mod relationship_repository;
mod scene_repository;
mod settings_repository;
mod sheet_template_repository;
mod skill_repository;
mod story_event_repository;
mod workflow_repository;
mod world_repository;

pub use asset_repository::Neo4jAssetRepository;
pub use challenge_repository::Neo4jChallengeRepository;
pub use character_repository::Neo4jCharacterRepository;
// Re-export region types from domain layer
pub use crate::domain::value_objects::{
    RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift,
};
pub use connection::Neo4jConnection;
pub use event_chain_repository::Neo4jEventChainRepository;
pub use interaction_repository::Neo4jInteractionRepository;
pub use location_repository::Neo4jLocationRepository;
pub use narrative_event_repository::Neo4jNarrativeEventRepository;
pub use observation_repository::Neo4jObservationRepository;
pub use region_repository::Neo4jRegionRepository;
pub use player_character_repository::Neo4jPlayerCharacterRepository;
pub use relationship_repository::Neo4jRelationshipRepository;
pub use scene_repository::Neo4jSceneRepository;
pub use settings_repository::SqliteSettingsRepository;
pub use sheet_template_repository::Neo4jSheetTemplateRepository;
pub use skill_repository::Neo4jSkillRepository;
pub use story_event_repository::Neo4jStoryEventRepository;
pub use workflow_repository::Neo4jWorkflowRepository;
pub use world_repository::Neo4jWorldRepository;

use anyhow::Result;

/// Combined repository providing access to all domain repositories
#[derive(Clone)]
pub struct Neo4jRepository {
    connection: Neo4jConnection,
}

impl Neo4jRepository {
    pub async fn new(uri: &str, user: &str, password: &str, database: &str) -> Result<Self> {
        let connection = Neo4jConnection::new(uri, user, password, database).await?;
        connection.initialize_schema().await?;
        Ok(Self { connection })
    }

    pub fn worlds(&self) -> Neo4jWorldRepository {
        Neo4jWorldRepository::new(self.connection.clone())
    }

    pub fn characters(&self) -> Neo4jCharacterRepository {
        Neo4jCharacterRepository::new(self.connection.clone())
    }

    pub fn locations(&self) -> Neo4jLocationRepository {
        Neo4jLocationRepository::new(self.connection.clone())
    }

    pub fn scenes(&self) -> Neo4jSceneRepository {
        Neo4jSceneRepository::new(self.connection.clone())
    }

    pub fn relationships(&self) -> Neo4jRelationshipRepository {
        Neo4jRelationshipRepository::new(self.connection.clone())
    }

    pub fn interactions(&self) -> Neo4jInteractionRepository {
        Neo4jInteractionRepository::new(self.connection.clone())
    }

    pub fn assets(&self) -> Neo4jAssetRepository {
        Neo4jAssetRepository::new(self.connection.clone())
    }

    pub fn workflows(&self) -> Neo4jWorkflowRepository {
        Neo4jWorkflowRepository::new(self.connection.clone())
    }

    pub fn skills(&self) -> Neo4jSkillRepository {
        Neo4jSkillRepository::new(self.connection.clone())
    }

    pub fn sheet_templates(&self) -> Neo4jSheetTemplateRepository {
        Neo4jSheetTemplateRepository::new(self.connection.clone())
    }

    pub fn challenges(&self) -> Neo4jChallengeRepository {
        Neo4jChallengeRepository::new(self.connection.clone())
    }

    pub fn story_events(&self) -> Neo4jStoryEventRepository {
        Neo4jStoryEventRepository::new(self.connection.clone())
    }

    pub fn narrative_events(&self) -> Neo4jNarrativeEventRepository {
        Neo4jNarrativeEventRepository::new(self.connection.clone())
    }

    pub fn event_chains(&self) -> Neo4jEventChainRepository {
        Neo4jEventChainRepository::new(self.connection.clone())
    }

    pub fn player_characters(&self) -> Neo4jPlayerCharacterRepository {
        Neo4jPlayerCharacterRepository::new(self.connection.clone())
    }

    pub fn regions(&self) -> Neo4jRegionRepository {
        Neo4jRegionRepository::new(self.connection.clone())
    }

    pub fn observations(&self) -> Neo4jObservationRepository {
        Neo4jObservationRepository::new(self.connection.clone())
    }
}
