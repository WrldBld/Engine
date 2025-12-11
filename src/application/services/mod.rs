//! Application services - Use case implementations
//!
//! This module contains the application services that implement the use cases
//! for the WrldBldr Engine. Each service follows hexagonal architecture principles,
//! accepting repository dependencies and returning domain entities or DTOs.

pub mod character_service;
pub mod generation_service;
pub mod llm_service;
pub mod location_service;
pub mod scene_service;
pub mod suggestion_service;
pub mod world_service;

// Re-export generation service types
pub use generation_service::{GenerationEvent, GenerationRequest, GenerationService};

// Re-export LLM service types
pub use llm_service::{
    CharacterContext, ConversationTurn, GamePromptRequest, LLMGameResponse, LLMService,
    LLMServiceError, PlayerActionContext, ProposedToolCall, SceneContext,
};

// Re-export world service types
pub use world_service::{
    CreateActRequest, CreateWorldRequest, UpdateWorldRequest, WorldService, WorldServiceImpl,
    WorldWithActs,
};

// Re-export scene service types
pub use scene_service::{
    CreateSceneRequest, SceneService, SceneServiceImpl, SceneWithRelations, UpdateSceneRequest,
};

// Re-export character service types
pub use character_service::{
    ChangeArchetypeRequest, CharacterService, CharacterServiceImpl, CharacterWithRelationships,
    CreateCharacterRequest, UpdateCharacterRequest,
};

// Re-export location service types
pub use location_service::{
    CreateConnectionRequest, CreateLocationRequest, LocationHierarchy, LocationService,
    LocationServiceImpl, LocationWithConnections, UpdateLocationRequest,
};

// Re-export suggestion service types
pub use suggestion_service::{
    SuggestionContext, SuggestionRequest, SuggestionResponse, SuggestionService, SuggestionType,
};
