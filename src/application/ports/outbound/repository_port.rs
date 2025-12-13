//! Repository ports - Interfaces for data persistence
//!
//! These traits define the contracts that infrastructure repositories must implement.
//! Application services depend on these traits, not concrete implementations.

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::entities::{
    Act, Character, GalleryAsset, GenerationBatch, GridMap, InteractionTemplate, Location,
    LocationConnection, Scene, Skill, StoryEvent, World,
};
use crate::domain::value_objects::{
    ActId, AssetId, BatchId, CharacterId, GridMapId, InteractionId, LocationId, Relationship,
    RelationshipId, SceneId, SessionId, SkillId, StoryEventId, WorldId,
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
    async fn get(&self, id: InteractionId) -> Result<Option<InteractionTemplate>>;

    /// List interaction templates in a scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>>;

    /// Update an interaction template
    async fn update(&self, interaction: &InteractionTemplate) -> Result<()>;

    /// Delete an interaction template
    async fn delete(&self, id: InteractionId) -> Result<()>;
}

// =============================================================================
// Relationship Repository Port
// =============================================================================

/// Repository port for Relationship (graph edge) operations
#[async_trait]
pub trait RelationshipRepositoryPort: Send + Sync {
    /// Create a relationship between characters
    async fn create(&self, relationship: &Relationship) -> Result<()>;

    /// Get a relationship by ID
    async fn get(&self, id: RelationshipId) -> Result<Option<Relationship>>;

    /// Get all relationships for a character (outgoing)
    async fn get_for_character(&self, character_id: CharacterId) -> Result<Vec<Relationship>>;

    /// Update a relationship
    async fn update(&self, relationship: &Relationship) -> Result<()>;

    /// Delete a relationship by ID
    async fn delete(&self, id: RelationshipId) -> Result<()>;
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
    async fn create(&self, skill: &Skill) -> Result<()>;

    /// Get a skill by ID
    async fn get(&self, id: SkillId) -> Result<Option<Skill>>;

    /// List skills for a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Skill>>;

    /// Update a skill
    async fn update(&self, skill: &Skill) -> Result<()>;

    /// Delete a skill
    async fn delete(&self, id: SkillId) -> Result<()>;
}

// =============================================================================
// Asset Repository Port
// =============================================================================

/// Repository port for GalleryAsset operations
#[async_trait]
pub trait AssetRepositoryPort: Send + Sync {
    /// Create an asset
    async fn create(&self, asset: &GalleryAsset) -> Result<()>;

    /// Get an asset by ID
    async fn get(&self, id: AssetId) -> Result<Option<GalleryAsset>>;

    /// List assets for an entity
    async fn list_for_entity(&self, entity_type: &str, entity_id: &str) -> Result<Vec<GalleryAsset>>;

    /// Activate an asset (set as current for its slot)
    async fn activate(&self, id: AssetId) -> Result<()>;

    /// Delete an asset
    async fn delete(&self, id: AssetId) -> Result<()>;

    /// Create a generation batch
    async fn create_batch(&self, batch: &GenerationBatch) -> Result<()>;

    /// Get a batch by ID
    async fn get_batch(&self, id: BatchId) -> Result<Option<GenerationBatch>>;

    /// Update batch status
    async fn update_batch_status(
        &self,
        id: BatchId,
        status: &crate::domain::entities::BatchStatus,
    ) -> Result<()>;

    /// Update the assets associated with a batch
    async fn update_batch_assets(&self, id: BatchId, assets: &[AssetId]) -> Result<()>;

    /// List all active (queued or generating) batches
    async fn list_active_batches(&self) -> Result<Vec<GenerationBatch>>;

    /// List batches ready for selection
    async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>>;

    /// Delete a batch
    async fn delete_batch(&self, id: BatchId) -> Result<()>;
}

// =============================================================================
// StoryEvent Repository Port
// =============================================================================

/// Repository port for StoryEvent operations
#[async_trait]
pub trait StoryEventRepositoryPort: Send + Sync {
    /// Create a new story event
    async fn create(&self, event: &StoryEvent) -> Result<()>;

    /// Get a story event by ID
    async fn get(&self, id: StoryEventId) -> Result<Option<StoryEvent>>;

    /// List story events for a session
    async fn list_by_session(&self, session_id: SessionId) -> Result<Vec<StoryEvent>>;

    /// List story events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<StoryEvent>>;

    /// List story events for a world with pagination
    async fn list_by_world_paginated(
        &self,
        world_id: WorldId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoryEvent>>;

    /// List visible (non-hidden) story events for a world
    async fn list_visible(&self, world_id: WorldId, limit: u32) -> Result<Vec<StoryEvent>>;

    /// Search story events by tags
    async fn search_by_tags(&self, world_id: WorldId, tags: Vec<String>) -> Result<Vec<StoryEvent>>;

    /// Search story events by text in summary
    async fn search_by_text(&self, world_id: WorldId, search_text: &str) -> Result<Vec<StoryEvent>>;

    /// List events involving a specific character
    async fn list_by_character(&self, character_id: CharacterId) -> Result<Vec<StoryEvent>>;

    /// List events at a specific location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<StoryEvent>>;

    /// Update story event summary
    async fn update_summary(&self, id: StoryEventId, summary: &str) -> Result<bool>;

    /// Update event visibility
    async fn set_hidden(&self, id: StoryEventId, is_hidden: bool) -> Result<bool>;

    /// Update event tags
    async fn update_tags(&self, id: StoryEventId, tags: Vec<String>) -> Result<bool>;

    /// Delete a story event
    async fn delete(&self, id: StoryEventId) -> Result<bool>;

    /// Count events for a world
    async fn count_by_world(&self, world_id: WorldId) -> Result<u64>;
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
