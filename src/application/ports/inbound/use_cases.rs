//! Use case traits - Inbound ports that define the application's capabilities
//!
//! These traits define what the application can do, without specifying how.
//! They are implemented by application services.
//!
//! **Status**: Planned for Phase 3.1 DDD implementation
//! Currently unused - services will implement these traits in the future

#![allow(dead_code)]

use async_trait::async_trait;

use crate::domain::value_objects::{WorldId, CharacterId, LocationId, SceneId};

// ============================================================================
// Error Types
// ============================================================================

/// Common error type for use case operations
#[derive(Debug, Clone)]
pub enum UseCaseError {
    /// Entity not found
    NotFound(String),
    /// Validation error
    ValidationError(String),
    /// Conflict (e.g., duplicate name)
    Conflict(String),
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for UseCaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UseCaseError::NotFound(msg) => write!(f, "Not found: {}", msg),
            UseCaseError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            UseCaseError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            UseCaseError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for UseCaseError {}

// ============================================================================
// World Use Cases
// ============================================================================

/// Request to create a new world
pub struct CreateWorldRequest {
    pub name: String,
    pub description: Option<String>,
    pub rule_system: Option<serde_json::Value>,
}

/// Summary of a world for list views
pub struct WorldSummaryDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Detailed world data
pub struct WorldDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub rule_system: Option<serde_json::Value>,
}

/// Use cases for managing worlds
#[async_trait]
pub trait ManageWorldUseCase: Send + Sync {
    /// List all worlds
    async fn list_worlds(&self) -> Result<Vec<WorldSummaryDto>, UseCaseError>;

    /// Get a world by ID
    async fn get_world(&self, id: WorldId) -> Result<WorldDto, UseCaseError>;

    /// Create a new world
    async fn create_world(&self, request: CreateWorldRequest) -> Result<WorldDto, UseCaseError>;

    /// Delete a world
    async fn delete_world(&self, id: WorldId) -> Result<(), UseCaseError>;
}

// ============================================================================
// Character Use Cases
// ============================================================================

/// Request to create a character
pub struct CreateCharacterRequest {
    pub name: String,
    pub description: Option<String>,
    pub archetype: Option<String>,
}

/// Character summary for list views
pub struct CharacterSummaryDto {
    pub id: String,
    pub name: String,
    pub archetype: Option<String>,
}

/// Use cases for managing characters
#[async_trait]
pub trait ManageCharacterUseCase: Send + Sync {
    /// List characters in a world
    async fn list_characters(&self, world_id: WorldId) -> Result<Vec<CharacterSummaryDto>, UseCaseError>;

    /// Get a character by ID
    async fn get_character(&self, id: CharacterId) -> Result<CharacterSummaryDto, UseCaseError>;

    /// Create a new character
    async fn create_character(
        &self,
        world_id: WorldId,
        request: CreateCharacterRequest,
    ) -> Result<CharacterSummaryDto, UseCaseError>;

    /// Delete a character
    async fn delete_character(&self, id: CharacterId) -> Result<(), UseCaseError>;
}

// ============================================================================
// Location Use Cases
// ============================================================================

/// Request to create a location
pub struct CreateLocationRequest {
    pub name: String,
    pub description: Option<String>,
}

/// Location summary for list views
pub struct LocationSummaryDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Use cases for managing locations
#[async_trait]
pub trait ManageLocationUseCase: Send + Sync {
    /// List locations in a world
    async fn list_locations(&self, world_id: WorldId) -> Result<Vec<LocationSummaryDto>, UseCaseError>;

    /// Get a location by ID
    async fn get_location(&self, id: LocationId) -> Result<LocationSummaryDto, UseCaseError>;

    /// Create a new location
    async fn create_location(
        &self,
        world_id: WorldId,
        request: CreateLocationRequest,
    ) -> Result<LocationSummaryDto, UseCaseError>;

    /// Delete a location
    async fn delete_location(&self, id: LocationId) -> Result<(), UseCaseError>;
}

// ============================================================================
// Scene Use Cases
// ============================================================================

/// Scene summary for list views
pub struct SceneSummaryDto {
    pub id: String,
    pub name: String,
    pub location_name: Option<String>,
}

/// Use cases for managing scenes
#[async_trait]
pub trait ManageSceneUseCase: Send + Sync {
    /// List scenes in a world
    async fn list_scenes(&self, world_id: WorldId) -> Result<Vec<SceneSummaryDto>, UseCaseError>;

    /// Get a scene by ID
    async fn get_scene(&self, id: SceneId) -> Result<SceneSummaryDto, UseCaseError>;
}
