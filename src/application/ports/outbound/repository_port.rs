//! Repository ports - Interfaces for data persistence
//!
//! These traits define the contracts that infrastructure repositories must implement.
//! Application services depend on these traits, not concrete implementations.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    Act, ActantialRole, ActantialView, AcquisitionMethod, ChainStatus, Challenge,
    ChallengeLocationAvailability, ChallengePrerequisite, Character, CharacterSheetTemplate,
    CharacterWant, EventChain, EventChainMembership, FeaturedNpc, FrequencyLevel, GalleryAsset,
    GenerationBatch, Goal, GridMap, InteractionRequirement, InteractionTargetType,
    InteractionTemplate, InventoryItem, InvolvedCharacter, Item, Location, LocationConnection,
    NarrativeEvent, PlayerCharacter, Region, Scene, SceneCharacter, SceneCharacterRole, SheetTemplateId,
    Skill, StoryEvent, Want, World, WorkflowConfiguration,
};
use crate::domain::value_objects::{
    ActId, AssetId, BatchId, ChallengeId, CharacterId, EventChainId, GoalId, GridMapId,
    InteractionId, ItemId, LocationId, NarrativeEventId, PlayerCharacterId, RegionId,
    RegionRelationshipType, Relationship, RelationshipId, SceneId, SessionId, SkillId,
    StoryEventId, WantId, WorldId,
};
use crate::domain::entities::WorkflowSlot;

// =============================================================================
// Social Network DTOs
// =============================================================================

/// Representation of the social network graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialNetwork {
    pub characters: Vec<CharacterNode>,
    pub relationships: Vec<RelationshipEdge>,
}

/// A node in the social network (character)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterNode {
    pub id: String,
    pub name: String,
    pub archetype: String,
}

/// An edge in the social network (relationship)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEdge {
    pub from_id: String,
    pub to_id: String,
    pub relationship_type: String,
    pub sentiment: f32,
}

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
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // Wants (HAS_WANT edges to Want nodes, TARGETS edges from Want)
    // -------------------------------------------------------------------------

    /// Create a want and attach it to a character
    async fn create_want(&self, character_id: CharacterId, want: &Want, priority: u32)
        -> Result<()>;

    /// Get all wants for a character
    async fn get_wants(&self, character_id: CharacterId) -> Result<Vec<CharacterWant>>;

    /// Update a want
    async fn update_want(&self, want: &Want) -> Result<()>;

    /// Delete a want
    async fn delete_want(&self, want_id: WantId) -> Result<()>;

    /// Set a want's target (creates TARGETS edge)
    /// target_type: "Character", "Item", or "Goal"
    async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: &str,
        target_type: &str,
    ) -> Result<()>;

    /// Remove a want's target (deletes TARGETS edge)
    async fn remove_want_target(&self, want_id: WantId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Actantial Views (VIEWS_AS_* edges)
    // -------------------------------------------------------------------------

    /// Add an actantial view (Helper, Opponent, Sender, Receiver)
    async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        view: &ActantialView,
    ) -> Result<()>;

    /// Get all actantial views for a character (as subject)
    async fn get_actantial_views(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<(ActantialRole, CharacterId, ActantialView)>>;

    /// Remove an actantial view
    async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        want_id: WantId,
    ) -> Result<()>;

    // -------------------------------------------------------------------------
    // Inventory (POSSESSES edges to Item nodes)
    // -------------------------------------------------------------------------

    /// Add an item to character's inventory
    async fn add_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()>;

    /// Get character's inventory
    async fn get_inventory(&self, character_id: CharacterId) -> Result<Vec<InventoryItem>>;

    /// Update inventory item (quantity, equipped status)
    async fn update_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
    ) -> Result<()>;

    /// Remove an item from inventory
    async fn remove_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<()>;

    // -------------------------------------------------------------------------
    // Character-Location Relationships
    // -------------------------------------------------------------------------

    /// Set character's home location
    async fn set_home_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        description: Option<String>,
    ) -> Result<()>;

    /// Remove character's home location
    async fn remove_home_location(&self, character_id: CharacterId) -> Result<()>;

    /// Set character's work location
    async fn set_work_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        role: String,
        schedule: Option<String>,
    ) -> Result<()>;

    /// Remove character's work location
    async fn remove_work_location(&self, character_id: CharacterId) -> Result<()>;

    /// Add a frequented location
    async fn add_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        frequency: FrequencyLevel,
        time_of_day: String,
        day_of_week: Option<String>,
        reason: Option<String>,
    ) -> Result<()>;

    /// Remove a frequented location
    async fn remove_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Add an avoided location
    async fn add_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        reason: String,
    ) -> Result<()>;

    /// Remove an avoided location
    async fn remove_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Get NPCs who might be at a location (based on home, work, frequents)
    async fn get_npcs_at_location(
        &self,
        location_id: LocationId,
        time_of_day: Option<&str>,
    ) -> Result<Vec<Character>>;
}

// =============================================================================
// Player Character Repository Port
// =============================================================================

/// Repository port for PlayerCharacter operations
#[async_trait]
pub trait PlayerCharacterRepositoryPort: Send + Sync {
    /// Create a new player character
    async fn create(&self, pc: &PlayerCharacter) -> Result<()>;

    /// Get a player character by ID
    async fn get(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>>;

    /// Get all player characters in a session
    async fn get_by_session(&self, session_id: SessionId) -> Result<Vec<PlayerCharacter>>;

    /// Get a player character by user ID and session ID
    async fn get_by_user_and_session(
        &self,
        user_id: &str,
        session_id: SessionId,
    ) -> Result<Option<PlayerCharacter>>;

    /// Get all player characters at a specific location
    async fn get_by_location(&self, location_id: LocationId) -> Result<Vec<PlayerCharacter>>;

    /// Get all player characters for a user in a world (for PC selection)
    async fn get_by_user_and_world(
        &self,
        user_id: &str,
        world_id: WorldId,
    ) -> Result<Vec<PlayerCharacter>>;

    /// Get all unbound player characters for a user (no session)
    async fn get_unbound_by_user(&self, user_id: &str) -> Result<Vec<PlayerCharacter>>;

    /// Update a player character
    async fn update(&self, pc: &PlayerCharacter) -> Result<()>;

    /// Update a player character's location (clears region)
    async fn update_location(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Update a player character's region (within current location)
    async fn update_region(
        &self,
        id: PlayerCharacterId,
        region_id: RegionId,
    ) -> Result<()>;

    /// Update both location and region at once
    async fn update_position(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
        region_id: Option<RegionId>,
    ) -> Result<()>;

    /// Bind a player character to a session
    async fn bind_to_session(
        &self,
        id: PlayerCharacterId,
        session_id: SessionId,
    ) -> Result<()>;

    /// Unbind a player character from its session
    async fn unbind_from_session(&self, id: PlayerCharacterId) -> Result<()>;

    /// Delete a player character
    async fn delete(&self, id: PlayerCharacterId) -> Result<()>;
}

// =============================================================================
// Location Repository Port
// =============================================================================

/// Repository port for Location operations
#[async_trait]
pub trait LocationRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // Location Hierarchy (CONTAINS_LOCATION edges)
    // -------------------------------------------------------------------------

    /// Set a location's parent (creates CONTAINS_LOCATION edge)
    async fn set_parent(&self, child_id: LocationId, parent_id: LocationId) -> Result<()>;

    /// Remove a location's parent (deletes CONTAINS_LOCATION edge)
    async fn remove_parent(&self, child_id: LocationId) -> Result<()>;

    /// Get a location's parent
    async fn get_parent(&self, location_id: LocationId) -> Result<Option<Location>>;

    /// Get a location's children
    async fn get_children(&self, location_id: LocationId) -> Result<Vec<Location>>;

    // -------------------------------------------------------------------------
    // Location Connections (CONNECTED_TO edges)
    // -------------------------------------------------------------------------

    /// Create a connection between locations
    async fn create_connection(&self, connection: &LocationConnection) -> Result<()>;

    /// Get all connections from a location
    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>>;

    /// Update a connection's properties
    async fn update_connection(&self, connection: &LocationConnection) -> Result<()>;

    /// Delete a connection between locations
    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()>;

    /// Unlock a connection (set is_locked = false)
    async fn unlock_connection(&self, from: LocationId, to: LocationId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Grid Map (HAS_TACTICAL_MAP edge)
    // -------------------------------------------------------------------------

    /// Set a location's tactical map
    async fn set_grid_map(&self, location_id: LocationId, grid_map_id: GridMapId) -> Result<()>;

    /// Remove a location's tactical map
    async fn remove_grid_map(&self, location_id: LocationId) -> Result<()>;

    /// Get a location's tactical map ID
    async fn get_grid_map_id(&self, location_id: LocationId) -> Result<Option<GridMapId>>;

    // -------------------------------------------------------------------------
    // Regions (HAS_REGION edges to Region nodes)
    // -------------------------------------------------------------------------

    /// Create a region within a location
    async fn create_region(&self, location_id: LocationId, region: &Region) -> Result<()>;

    /// Get all regions in a location
    async fn get_regions(&self, location_id: LocationId) -> Result<Vec<Region>>;
}

// =============================================================================
// Scene Repository Port
// =============================================================================

/// Repository port for Scene operations
#[async_trait]
pub trait SceneRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    /// Create a new scene
    async fn create(&self, scene: &Scene) -> Result<()>;

    /// Get a scene by ID
    async fn get(&self, id: SceneId) -> Result<Option<Scene>>;

    /// List scenes by act
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<Scene>>;

    /// List scenes by location (via AT_LOCATION edge)
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>>;

    /// Update a scene
    async fn update(&self, scene: &Scene) -> Result<()>;

    /// Delete a scene
    async fn delete(&self, id: SceneId) -> Result<()>;

    /// Update directorial notes for a scene
    async fn update_directorial_notes(&self, id: SceneId, notes: &str) -> Result<()>;

    // -------------------------------------------------------------------------
    // Location (AT_LOCATION edge)
    // -------------------------------------------------------------------------

    /// Set scene's location (creates AT_LOCATION edge)
    async fn set_location(&self, scene_id: SceneId, location_id: LocationId) -> Result<()>;

    /// Get scene's location
    async fn get_location(&self, scene_id: SceneId) -> Result<Option<LocationId>>;

    // -------------------------------------------------------------------------
    // Featured Characters (FEATURES_CHARACTER edges)
    // -------------------------------------------------------------------------

    /// Add a featured character to the scene
    async fn add_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()>;

    /// Get all featured characters for a scene
    async fn get_featured_characters(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<(CharacterId, SceneCharacter)>>;

    /// Update a featured character's role/cue
    async fn update_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()>;

    /// Remove a featured character from the scene
    async fn remove_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<()>;

    /// Get scenes featuring a specific character
    async fn get_scenes_for_character(&self, character_id: CharacterId) -> Result<Vec<Scene>>;
}

// =============================================================================
// Interaction Repository Port
// =============================================================================

/// Repository port for InteractionTemplate operations
#[async_trait]
pub trait InteractionRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // Target Edges (TARGETS_CHARACTER, TARGETS_ITEM, TARGETS_REGION)
    // -------------------------------------------------------------------------

    /// Set interaction target to a character
    async fn set_target_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()>;

    /// Set interaction target to an item
    async fn set_target_item(
        &self,
        interaction_id: InteractionId,
        item_id: ItemId,
    ) -> Result<()>;

    /// Set interaction target to a backdrop region
    async fn set_target_region(
        &self,
        interaction_id: InteractionId,
        region_id: RegionId,
    ) -> Result<()>;

    /// Remove any target from the interaction
    async fn remove_target(&self, interaction_id: InteractionId) -> Result<()>;

    /// Get the target type and ID for an interaction
    async fn get_target(
        &self,
        interaction_id: InteractionId,
    ) -> Result<Option<(InteractionTargetType, String)>>;

    // -------------------------------------------------------------------------
    // Requirement Edges (REQUIRES_ITEM, REQUIRES_CHARACTER_PRESENT)
    // -------------------------------------------------------------------------

    /// Add a required item for the interaction
    async fn add_required_item(
        &self,
        interaction_id: InteractionId,
        item_id: ItemId,
        requirement: &InteractionRequirement,
    ) -> Result<()>;

    /// Remove a required item
    async fn remove_required_item(
        &self,
        interaction_id: InteractionId,
        item_id: ItemId,
    ) -> Result<()>;

    /// Add a required character presence
    async fn add_required_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()>;

    /// Remove a required character presence
    async fn remove_required_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()>;
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

    /// Get the social network graph for a world
    async fn get_social_network(&self, world_id: WorldId) -> Result<SocialNetwork>;
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
// Item Repository Port
// =============================================================================

/// Repository port for Item operations
#[async_trait]
pub trait ItemRepositoryPort: Send + Sync {
    /// Create a new item
    async fn create(&self, item: &Item) -> Result<()>;

    /// Get an item by ID
    async fn get(&self, id: ItemId) -> Result<Option<Item>>;

    /// List all items in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Item>>;

    /// Update an item
    async fn update(&self, item: &Item) -> Result<()>;

    /// Delete an item
    async fn delete(&self, id: ItemId) -> Result<()>;

    /// Get items by type
    async fn get_by_type(&self, world_id: WorldId, item_type: &str) -> Result<Vec<Item>>;
}

// =============================================================================
// Goal Repository Port
// =============================================================================

/// Repository port for Goal operations
#[async_trait]
pub trait GoalRepositoryPort: Send + Sync {
    /// Create a new goal
    async fn create(&self, goal: &Goal) -> Result<()>;

    /// Get a goal by ID
    async fn get(&self, id: GoalId) -> Result<Option<Goal>>;

    /// List all goals in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Goal>>;

    /// Update a goal
    async fn update(&self, goal: &Goal) -> Result<()>;

    /// Delete a goal
    async fn delete(&self, id: GoalId) -> Result<()>;
}

// =============================================================================
// Want Repository Port
// =============================================================================

/// Repository port for standalone Want operations
#[async_trait]
pub trait WantRepositoryPort: Send + Sync {
    /// Get a want by ID
    async fn get(&self, id: WantId) -> Result<Option<Want>>;

    /// Get the target of a want (returns type and ID)
    async fn get_target(&self, want_id: WantId) -> Result<Option<(String, String)>>;
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

    /// Update an asset's label
    async fn update_label(&self, id: AssetId, label: Option<String>) -> Result<()>;

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

    /// List all active (queued or generating) batches for a specific world
    async fn list_active_batches_by_world(&self, world_id: WorldId) -> Result<Vec<GenerationBatch>>;

    /// List batches ready for selection
    async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>>;

    /// Delete a batch
    async fn delete_batch(&self, id: BatchId) -> Result<()>;
}

// =============================================================================
// Challenge Repository Port
// =============================================================================

/// Repository port for Challenge operations
#[async_trait]
pub trait ChallengeRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    /// Create a new challenge
    async fn create(&self, challenge: &Challenge) -> Result<()>;

    /// Get a challenge by ID
    async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>>;

    /// List all challenges for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List challenges for a specific scene (via TIED_TO_SCENE edge)
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>>;

    /// List challenges available at a location (via AVAILABLE_AT edge)
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Challenge>>;

    /// List active challenges for a world (for LLM context)
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List favorite challenges for quick access
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// Update a challenge
    async fn update(&self, challenge: &Challenge) -> Result<()>;

    /// Delete a challenge
    async fn delete(&self, id: ChallengeId) -> Result<()>;

    /// Set active status for a challenge
    async fn set_active(&self, id: ChallengeId, active: bool) -> Result<()>;

    /// Toggle favorite status
    async fn toggle_favorite(&self, id: ChallengeId) -> Result<bool>;

    // -------------------------------------------------------------------------
    // Skill Edge (REQUIRES_SKILL)
    // -------------------------------------------------------------------------

    /// Set the required skill for a challenge (creates REQUIRES_SKILL edge)
    async fn set_required_skill(
        &self,
        challenge_id: ChallengeId,
        skill_id: SkillId,
    ) -> Result<()>;

    /// Get the required skill for a challenge
    async fn get_required_skill(&self, challenge_id: ChallengeId) -> Result<Option<SkillId>>;

    /// Remove the required skill from a challenge
    async fn remove_required_skill(&self, challenge_id: ChallengeId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Scene Edge (TIED_TO_SCENE)
    // -------------------------------------------------------------------------

    /// Tie a challenge to a scene (creates TIED_TO_SCENE edge)
    async fn tie_to_scene(&self, challenge_id: ChallengeId, scene_id: SceneId) -> Result<()>;

    /// Get the scene a challenge is tied to
    async fn get_tied_scene(&self, challenge_id: ChallengeId) -> Result<Option<SceneId>>;

    /// Remove the scene tie from a challenge
    async fn untie_from_scene(&self, challenge_id: ChallengeId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Prerequisite Edges (REQUIRES_COMPLETION_OF)
    // -------------------------------------------------------------------------

    /// Add a prerequisite challenge (creates REQUIRES_COMPLETION_OF edge)
    async fn add_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite: ChallengePrerequisite,
    ) -> Result<()>;

    /// Get all prerequisites for a challenge
    async fn get_prerequisites(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengePrerequisite>>;

    /// Remove a prerequisite from a challenge
    async fn remove_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite_id: ChallengeId,
    ) -> Result<()>;

    /// Get challenges that require this challenge as a prerequisite
    async fn get_dependent_challenges(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeId>>;

    // -------------------------------------------------------------------------
    // Location Availability Edges (AVAILABLE_AT)
    // -------------------------------------------------------------------------

    /// Add a location where this challenge is available (creates AVAILABLE_AT edge)
    async fn add_location_availability(
        &self,
        challenge_id: ChallengeId,
        availability: ChallengeLocationAvailability,
    ) -> Result<()>;

    /// Get all locations where a challenge is available
    async fn get_location_availabilities(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeLocationAvailability>>;

    /// Remove a location availability from a challenge
    async fn remove_location_availability(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()>;

    // -------------------------------------------------------------------------
    // Unlock Edges (ON_SUCCESS_UNLOCKS)
    // -------------------------------------------------------------------------

    /// Add a location that gets unlocked on successful challenge completion
    async fn add_unlock_location(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Get locations that get unlocked when this challenge succeeds
    async fn get_unlock_locations(&self, challenge_id: ChallengeId) -> Result<Vec<LocationId>>;

    /// Remove an unlock from a challenge
    async fn remove_unlock_location(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()>;
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

    // =========================================================================
    // OCCURRED_IN_SESSION Edge Methods
    // =========================================================================

    /// Set the session for a story event (creates OCCURRED_IN_SESSION edge)
    async fn set_session(&self, event_id: StoryEventId, session_id: SessionId) -> Result<bool>;

    /// Get the session for a story event
    async fn get_session(&self, event_id: StoryEventId) -> Result<Option<SessionId>>;

    // =========================================================================
    // OCCURRED_AT Edge Methods (Location)
    // =========================================================================

    /// Set the location where event occurred (creates OCCURRED_AT edge)
    async fn set_location(&self, event_id: StoryEventId, location_id: LocationId) -> Result<bool>;

    /// Get the location where event occurred
    async fn get_location(&self, event_id: StoryEventId) -> Result<Option<LocationId>>;

    /// Remove location association (deletes OCCURRED_AT edge)
    async fn remove_location(&self, event_id: StoryEventId) -> Result<bool>;

    // =========================================================================
    // OCCURRED_IN_SCENE Edge Methods
    // =========================================================================

    /// Set the scene where event occurred (creates OCCURRED_IN_SCENE edge)
    async fn set_scene(&self, event_id: StoryEventId, scene_id: SceneId) -> Result<bool>;

    /// Get the scene where event occurred
    async fn get_scene(&self, event_id: StoryEventId) -> Result<Option<SceneId>>;

    /// Remove scene association (deletes OCCURRED_IN_SCENE edge)
    async fn remove_scene(&self, event_id: StoryEventId) -> Result<bool>;

    // =========================================================================
    // INVOLVES Edge Methods
    // =========================================================================

    /// Add an involved character (creates INVOLVES edge with role)
    async fn add_involved_character(
        &self,
        event_id: StoryEventId,
        involved: InvolvedCharacter,
    ) -> Result<bool>;

    /// Get all involved characters for an event
    async fn get_involved_characters(
        &self,
        event_id: StoryEventId,
    ) -> Result<Vec<InvolvedCharacter>>;

    /// Remove an involved character (deletes INVOLVES edge)
    async fn remove_involved_character(
        &self,
        event_id: StoryEventId,
        character_id: CharacterId,
    ) -> Result<bool>;

    // =========================================================================
    // TRIGGERED_BY_NARRATIVE Edge Methods
    // =========================================================================

    /// Set the narrative event that triggered this story event
    async fn set_triggered_by(
        &self,
        event_id: StoryEventId,
        narrative_event_id: NarrativeEventId,
    ) -> Result<bool>;

    /// Get the narrative event that triggered this story event
    async fn get_triggered_by(&self, event_id: StoryEventId) -> Result<Option<NarrativeEventId>>;

    /// Remove the triggered_by association
    async fn remove_triggered_by(&self, event_id: StoryEventId) -> Result<bool>;

    // =========================================================================
    // RECORDS_CHALLENGE Edge Methods
    // =========================================================================

    /// Set the challenge this event records (creates RECORDS_CHALLENGE edge)
    async fn set_recorded_challenge(
        &self,
        event_id: StoryEventId,
        challenge_id: ChallengeId,
    ) -> Result<bool>;

    /// Get the challenge this event records
    async fn get_recorded_challenge(&self, event_id: StoryEventId) -> Result<Option<ChallengeId>>;

    /// Remove the recorded challenge association
    async fn remove_recorded_challenge(&self, event_id: StoryEventId) -> Result<bool>;

    // =========================================================================
    // Query Methods by Edge Relationships
    // =========================================================================

    /// List events triggered by a specific narrative event
    async fn list_by_narrative_event(
        &self,
        narrative_event_id: NarrativeEventId,
    ) -> Result<Vec<StoryEvent>>;

    /// List events recording a specific challenge
    async fn list_by_challenge(&self, challenge_id: ChallengeId) -> Result<Vec<StoryEvent>>;

    /// List events that occurred in a specific scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<StoryEvent>>;
}

// =============================================================================
// NarrativeEvent Repository Port
// =============================================================================

/// Repository port for NarrativeEvent operations
#[async_trait]
pub trait NarrativeEventRepositoryPort: Send + Sync {
    /// Create a new narrative event
    async fn create(&self, event: &NarrativeEvent) -> Result<()>;

    /// Get a narrative event by ID
    async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>>;

    /// Update a narrative event
    async fn update(&self, event: &NarrativeEvent) -> Result<bool>;

    /// List all narrative events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List active narrative events for a world
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List favorite narrative events for a world
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List untriggered active events (for LLM context)
    async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// Toggle favorite status
    async fn toggle_favorite(&self, id: NarrativeEventId) -> Result<bool>;

    /// Set active status
    async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> Result<bool>;

    /// Mark event as triggered
    async fn mark_triggered(&self, id: NarrativeEventId, outcome_name: Option<String>) -> Result<bool>;

    /// Reset triggered status (for repeatable events)
    async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool>;

    /// Delete a narrative event
    async fn delete(&self, id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // TIED_TO_SCENE Edge Methods
    // =========================================================================

    /// Tie event to a scene (creates TIED_TO_SCENE edge)
    async fn tie_to_scene(&self, event_id: NarrativeEventId, scene_id: SceneId) -> Result<bool>;

    /// Get the scene this event is tied to (if any)
    async fn get_tied_scene(&self, event_id: NarrativeEventId) -> Result<Option<SceneId>>;

    /// Remove scene tie (deletes TIED_TO_SCENE edge)
    async fn untie_from_scene(&self, event_id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // TIED_TO_LOCATION Edge Methods
    // =========================================================================

    /// Tie event to a location (creates TIED_TO_LOCATION edge)
    async fn tie_to_location(
        &self,
        event_id: NarrativeEventId,
        location_id: LocationId,
    ) -> Result<bool>;

    /// Get the location this event is tied to (if any)
    async fn get_tied_location(&self, event_id: NarrativeEventId) -> Result<Option<LocationId>>;

    /// Remove location tie (deletes TIED_TO_LOCATION edge)
    async fn untie_from_location(&self, event_id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // BELONGS_TO_ACT Edge Methods
    // =========================================================================

    /// Assign event to an act (creates BELONGS_TO_ACT edge)
    async fn assign_to_act(&self, event_id: NarrativeEventId, act_id: ActId) -> Result<bool>;

    /// Get the act this event belongs to (if any)
    async fn get_act(&self, event_id: NarrativeEventId) -> Result<Option<ActId>>;

    /// Remove act assignment (deletes BELONGS_TO_ACT edge)
    async fn unassign_from_act(&self, event_id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // FEATURES_NPC Edge Methods
    // =========================================================================

    /// Add a featured NPC to the event (creates FEATURES_NPC edge)
    async fn add_featured_npc(
        &self,
        event_id: NarrativeEventId,
        featured_npc: FeaturedNpc,
    ) -> Result<bool>;

    /// Get all featured NPCs for an event
    async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> Result<Vec<FeaturedNpc>>;

    /// Remove a featured NPC from the event (deletes FEATURES_NPC edge)
    async fn remove_featured_npc(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
    ) -> Result<bool>;

    /// Update featured NPC role
    async fn update_featured_npc_role(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
        role: Option<String>,
    ) -> Result<bool>;

    // =========================================================================
    // Chain Membership Query Methods (CONTAINS_EVENT edge is on EventChain side)
    // =========================================================================

    /// Get chain membership info for an event (queries CONTAINS_EVENT edges pointing to this event)
    async fn get_chain_memberships(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<Vec<EventChainMembership>>;

    // =========================================================================
    // Query Methods for Events by Edge Relationships
    // =========================================================================

    /// List events tied to a specific scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<NarrativeEvent>>;

    /// List events tied to a specific location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<NarrativeEvent>>;

    /// List events belonging to a specific act
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<NarrativeEvent>>;

    /// List events featuring a specific NPC
    async fn list_by_featured_npc(&self, character_id: CharacterId) -> Result<Vec<NarrativeEvent>>;
}

// =============================================================================
// EventChain Repository Port
// =============================================================================

/// Repository port for EventChain operations
#[async_trait]
pub trait EventChainRepositoryPort: Send + Sync {
    /// Create a new event chain
    async fn create(&self, chain: &EventChain) -> Result<()>;

    /// Get an event chain by ID
    async fn get(&self, id: EventChainId) -> Result<Option<EventChain>>;

    /// Update an event chain
    async fn update(&self, chain: &EventChain) -> Result<bool>;

    /// List all event chains for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List active event chains for a world
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List favorite event chains for a world
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// Get chains containing a specific narrative event
    async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>>;

    /// Add an event to a chain
    async fn add_event_to_chain(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<bool>;

    /// Remove an event from a chain
    async fn remove_event_from_chain(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<bool>;

    /// Mark an event as completed in a chain
    async fn complete_event(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<bool>;

    /// Toggle favorite status
    async fn toggle_favorite(&self, id: EventChainId) -> Result<bool>;

    /// Set active status
    async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<bool>;

    /// Reset chain progress
    async fn reset(&self, id: EventChainId) -> Result<bool>;

    /// Delete an event chain
    async fn delete(&self, id: EventChainId) -> Result<bool>;

    /// Get chain status summary
    async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>>;

    /// Get all chain statuses for a world
    async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>>;
}

// =============================================================================
// SheetTemplate Repository Port
// =============================================================================

/// Repository port for CharacterSheetTemplate operations
#[async_trait]
pub trait SheetTemplateRepositoryPort: Send + Sync {
    /// Create a new sheet template
    async fn create(&self, template: &CharacterSheetTemplate) -> Result<()>;

    /// Get a sheet template by ID
    async fn get(&self, id: &SheetTemplateId) -> Result<Option<CharacterSheetTemplate>>;

    /// Get the default template for a world
    async fn get_default_for_world(&self, world_id: &WorldId) -> Result<Option<CharacterSheetTemplate>>;

    /// List all templates for a world
    async fn list_by_world(&self, world_id: &WorldId) -> Result<Vec<CharacterSheetTemplate>>;

    /// Update a sheet template
    async fn update(&self, template: &CharacterSheetTemplate) -> Result<()>;

    /// Delete a sheet template
    async fn delete(&self, id: &SheetTemplateId) -> Result<()>;

    /// Delete all templates for a world
    async fn delete_all_for_world(&self, world_id: &WorldId) -> Result<()>;

    /// Check if a world has any templates
    async fn has_templates(&self, world_id: &WorldId) -> Result<bool>;
}

// =============================================================================
// Workflow Repository Port
// =============================================================================

/// Repository port for WorkflowConfiguration operations
#[async_trait]
pub trait WorkflowRepositoryPort: Send + Sync {
    /// Save a workflow configuration (create or update)
    async fn save(&self, config: &WorkflowConfiguration) -> Result<()>;

    /// Get a workflow configuration by slot
    async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>>;

    /// Delete a workflow configuration by slot
    async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool>;

    /// List all workflow configurations
    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>>;
}

// =============================================================================
// Region Repository Port
// =============================================================================

/// Repository port for Region operations
#[async_trait]
pub trait RegionRepositoryPort: Send + Sync {
    /// Get a region by ID
    async fn get(&self, id: RegionId) -> Result<Option<Region>>;

    /// List all regions in a location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Region>>;

    /// List all spawn point regions in a world
    async fn list_spawn_points(&self, world_id: WorldId) -> Result<Vec<Region>>;

    /// Get all NPCs with relationships to a region (for presence determination)
    async fn get_npcs_related_to_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>>;
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
