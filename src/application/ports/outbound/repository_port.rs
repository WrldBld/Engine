//! Repository ports - Interfaces for data persistence
//!
//! These traits define the contracts that infrastructure repositories must implement.
//! Application services depend on these traits, not concrete implementations.

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::entities::{
    Act, Character, GalleryAsset, GenerationBatch, GridMap, InteractionTemplate, Location,
    LocationConnection, Scene, Skill, World,
};
use crate::domain::value_objects::{
    ActId, AssetId, BatchId, CharacterId, GridMapId, LocationId, Relationship, SceneId, WorldId,
};

// =============================================================================
// World Repository Port
// =============================================================================

/// Repository port for World aggregate operations
#[async_trait]
pub trait WorldRepositoryPort: Send + Sync {
    /// Create a new world
    async fn create(&self, world: &World) -> Result<()>;

    /// Get a world by ID
    async fn get(&self, id: WorldId) -> Result<Option<World>>;

    /// List all worlds
    async fn list(&self) -> Result<Vec<World>>;

    /// Update a world
    async fn update(&self, world: &World) -> Result<()>;

    /// Delete a world and all its contents (cascading)
    async fn delete(&self, id: WorldId) -> Result<()>;

    /// Create an act within a world
    async fn create_act(&self, act: &Act) -> Result<()>;

    /// Get acts for a world
    async fn get_acts(&self, world_id: WorldId) -> Result<Vec<Act>>;
}

// =============================================================================
// Character Repository Port
// =============================================================================

/// Repository port for Character operations
#[async_trait]
pub trait CharacterRepositoryPort: Send + Sync {
    /// Create a new character
    async fn create(&self, character: &Character) -> Result<()>;

    /// Get a character by ID
    async fn get(&self, id: CharacterId) -> Result<Option<Character>>;

    /// List all characters in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Character>>;

    /// Update a character
    async fn update(&self, character: &Character) -> Result<()>;

    /// Delete a character
    async fn delete(&self, id: CharacterId) -> Result<()>;

    /// Get characters by scene
    async fn get_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>>;
}

// =============================================================================
// Location Repository Port
// =============================================================================

/// Repository port for Location operations
#[async_trait]
pub trait LocationRepositoryPort: Send + Sync {
    /// Create a new location
    async fn create(&self, location: &Location) -> Result<()>;

    /// Get a location by ID
    async fn get(&self, id: LocationId) -> Result<Option<Location>>;

    /// List all locations in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Location>>;

    /// Update a location
    async fn update(&self, location: &Location) -> Result<()>;

    /// Delete a location
    async fn delete(&self, id: LocationId) -> Result<()>;

    /// Create a connection between locations
    async fn create_connection(&self, connection: &LocationConnection) -> Result<()>;

    /// Get connections for a location
    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>>;

    /// Delete a connection between locations
    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()>;
}

// =============================================================================
// Scene Repository Port
// =============================================================================

/// Repository port for Scene operations
#[async_trait]
pub trait SceneRepositoryPort: Send + Sync {
    /// Create a new scene
    async fn create(&self, scene: &Scene) -> Result<()>;

    /// Get a scene by ID
    async fn get(&self, id: SceneId) -> Result<Option<Scene>>;

    /// List scenes by act
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<Scene>>;

    /// List scenes by location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>>;

    /// Update a scene
    async fn update(&self, scene: &Scene) -> Result<()>;

    /// Delete a scene
    async fn delete(&self, id: SceneId) -> Result<()>;

    /// Update directorial notes for a scene
    async fn update_directorial_notes(&self, id: SceneId, notes: &str) -> Result<()>;
}

// =============================================================================
// Interaction Repository Port
// =============================================================================

/// Repository port for InteractionTemplate operations
#[async_trait]
pub trait InteractionRepositoryPort: Send + Sync {
    /// Create a new interaction template
    async fn create(&self, interaction: &InteractionTemplate) -> Result<()>;

    /// Get an interaction template by ID
    async fn get(&self, id: &str) -> Result<Option<InteractionTemplate>>;

    /// List interaction templates in a scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>>;

    /// Delete an interaction template
    async fn delete(&self, id: &str) -> Result<()>;
}

// =============================================================================
// Relationship Repository Port
// =============================================================================

/// Repository port for Relationship (graph edge) operations
#[async_trait]
pub trait RelationshipRepositoryPort: Send + Sync {
    /// Save a relationship between characters
    async fn save(&self, relationship: &Relationship) -> Result<()>;

    /// Get all relationships for a character
    async fn get_for_character(&self, character_id: CharacterId) -> Result<Vec<Relationship>>;

    /// Delete a relationship between two characters
    async fn delete(&self, from: CharacterId, to: CharacterId) -> Result<()>;
}

// =============================================================================
// Grid Map Repository Port
// =============================================================================

/// Repository port for GridMap operations
#[async_trait]
pub trait GridMapRepositoryPort: Send + Sync {
    /// Save a grid map
    async fn save(&self, grid_map: &GridMap) -> Result<()>;

    /// Get a grid map by ID
    async fn get(&self, id: GridMapId) -> Result<Option<GridMap>>;

    /// Delete a grid map
    async fn delete(&self, id: GridMapId) -> Result<()>;
}

// =============================================================================
// Skill Repository Port
// =============================================================================

/// Repository port for Skill operations
#[async_trait]
pub trait SkillRepositoryPort: Send + Sync {
    /// Create a skill
    async fn create(&self, skill: &Skill) -> Result<Skill>;

    /// Get a skill by ID
    async fn get(&self, id: &str) -> Result<Option<Skill>>;

    /// List skills for a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Skill>>;

    /// Update a skill
    async fn update(&self, skill: &Skill) -> Result<Skill>;

    /// Delete a skill
    async fn delete(&self, id: &str) -> Result<()>;
}

// =============================================================================
// Asset Repository Port
// =============================================================================

/// Repository port for GalleryAsset operations
#[async_trait]
pub trait AssetRepositoryPort: Send + Sync {
    /// Create an asset
    async fn create(&self, asset: &GalleryAsset) -> Result<GalleryAsset>;

    /// Get an asset by ID
    async fn get(&self, id: AssetId) -> Result<Option<GalleryAsset>>;

    /// List assets for an entity
    async fn list_for_entity(&self, entity_type: &str, entity_id: &str) -> Result<Vec<GalleryAsset>>;

    /// Update an asset
    async fn update(&self, asset: &GalleryAsset) -> Result<GalleryAsset>;

    /// Delete an asset
    async fn delete(&self, id: AssetId) -> Result<()>;

    /// Create a generation batch
    async fn create_batch(&self, batch: &GenerationBatch) -> Result<GenerationBatch>;

    /// Get a batch by ID
    async fn get_batch(&self, id: BatchId) -> Result<Option<GenerationBatch>>;

    /// Update a batch
    async fn update_batch(&self, batch: &GenerationBatch) -> Result<GenerationBatch>;
}

// =============================================================================
// Repository Provider Port (Facade)
// =============================================================================

/// Facade trait providing access to all repository ports
///
/// This allows application services to receive a single dependency
/// that provides access to all needed repositories.
pub trait RepositoryProvider: Send + Sync {
    type WorldRepo: WorldRepositoryPort;
    type CharacterRepo: CharacterRepositoryPort;
    type LocationRepo: LocationRepositoryPort;
    type SceneRepo: SceneRepositoryPort;
    type InteractionRepo: InteractionRepositoryPort;
    type RelationshipRepo: RelationshipRepositoryPort;
    type SkillRepo: SkillRepositoryPort;
    type AssetRepo: AssetRepositoryPort;

    fn worlds(&self) -> Self::WorldRepo;
    fn characters(&self) -> Self::CharacterRepo;
    fn locations(&self) -> Self::LocationRepo;
    fn scenes(&self) -> Self::SceneRepo;
    fn interactions(&self) -> Self::InteractionRepo;
    fn relationships(&self) -> Self::RelationshipRepo;
    fn skills(&self) -> Self::SkillRepo;
    fn assets(&self) -> Self::AssetRepo;
}
