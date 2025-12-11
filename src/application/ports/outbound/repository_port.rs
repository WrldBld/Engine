//! Repository port - Interface for data persistence

use async_trait::async_trait;

use crate::domain::entities::{Character, GridMap, Location, Scene, World};
use crate::domain::value_objects::{
    CharacterId, GridMapId, LocationId, Relationship, SceneId, WorldId,
};

/// Repository for World aggregate
#[async_trait]
pub trait WorldRepository: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    // World operations
    async fn save_world(&self, world: &World) -> Result<(), Self::Error>;
    async fn get_world(&self, id: WorldId) -> Result<Option<World>, Self::Error>;
    async fn list_worlds(&self) -> Result<Vec<World>, Self::Error>;
    async fn delete_world(&self, id: WorldId) -> Result<(), Self::Error>;

    // Location operations
    async fn save_location(&self, location: &Location) -> Result<(), Self::Error>;
    async fn get_location(&self, id: LocationId) -> Result<Option<Location>, Self::Error>;
    async fn list_locations(&self, world_id: WorldId) -> Result<Vec<Location>, Self::Error>;
    async fn delete_location(&self, id: LocationId) -> Result<(), Self::Error>;

    // Character operations
    async fn save_character(&self, character: &Character) -> Result<(), Self::Error>;
    async fn get_character(&self, id: CharacterId) -> Result<Option<Character>, Self::Error>;
    async fn list_characters(&self, world_id: WorldId) -> Result<Vec<Character>, Self::Error>;
    async fn delete_character(&self, id: CharacterId) -> Result<(), Self::Error>;

    // Scene operations
    async fn save_scene(&self, scene: &Scene) -> Result<(), Self::Error>;
    async fn get_scene(&self, id: SceneId) -> Result<Option<Scene>, Self::Error>;
    async fn list_scenes(&self, world_id: WorldId) -> Result<Vec<Scene>, Self::Error>;
    async fn delete_scene(&self, id: SceneId) -> Result<(), Self::Error>;

    // GridMap operations
    async fn save_grid_map(&self, grid_map: &GridMap) -> Result<(), Self::Error>;
    async fn get_grid_map(&self, id: GridMapId) -> Result<Option<GridMap>, Self::Error>;
    async fn delete_grid_map(&self, id: GridMapId) -> Result<(), Self::Error>;

    // Relationship operations (graph edges)
    async fn save_relationship(&self, relationship: &Relationship) -> Result<(), Self::Error>;
    async fn get_relationships(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<Relationship>, Self::Error>;
    async fn delete_relationship(
        &self,
        from: CharacterId,
        to: CharacterId,
    ) -> Result<(), Self::Error>;
}
