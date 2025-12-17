//! Core domain services for world building

use crate::application::services::{
    CharacterServiceImpl, InteractionServiceImpl, LocationServiceImpl,
    RelationshipServiceImpl, SceneServiceImpl, SkillServiceImpl, WorldServiceImpl,
};

/// Core services for fundamental world-building entities
///
/// This struct groups the primary domain services that handle the core
/// entities of the world-building system: worlds, characters, locations,
/// scenes, skills, interactions, and relationships.
pub struct CoreServices {
    pub world_service: WorldServiceImpl,
    pub character_service: CharacterServiceImpl,
    pub location_service: LocationServiceImpl,
    pub scene_service: SceneServiceImpl,
    pub skill_service: SkillServiceImpl,
    pub interaction_service: InteractionServiceImpl,
    pub relationship_service: RelationshipServiceImpl,
}

impl CoreServices {
    /// Creates a new CoreServices instance with all the core domain services
    pub fn new(
        world_service: WorldServiceImpl,
        character_service: CharacterServiceImpl,
        location_service: LocationServiceImpl,
        scene_service: SceneServiceImpl,
        skill_service: SkillServiceImpl,
        interaction_service: InteractionServiceImpl,
        relationship_service: RelationshipServiceImpl,
    ) -> Self {
        Self {
            world_service,
            character_service,
            location_service,
            scene_service,
            skill_service,
            interaction_service,
            relationship_service,
        }
    }
}
