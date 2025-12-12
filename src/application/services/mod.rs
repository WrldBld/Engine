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
pub mod story_event_service;
pub mod suggestion_service;
pub mod tool_execution_service;
pub mod workflow_service;
pub mod world_service;

// Re-export LLM service types
#[allow(unused_imports)]
pub use llm_service::{
    ActiveNarrativeEventContext, GamePromptRequest, LLMGameResponse, LLMService,
    NarrativeEventSuggestion,
};

// Re-export world service types
#[allow(unused_imports)]
pub use world_service::{
    CreateActRequest, CreateWorldRequest, UpdateWorldRequest, WorldService, WorldServiceImpl,
};

// Re-export scene service types
#[allow(unused_imports)]
pub use scene_service::{
    CreateSceneRequest, SceneService, SceneServiceImpl, UpdateSceneRequest,
};

// Re-export character service types
#[allow(unused_imports)]
pub use character_service::{
    ChangeArchetypeRequest, CharacterService, CharacterServiceImpl,
    CreateCharacterRequest, UpdateCharacterRequest,
};

// Re-export location service types
#[allow(unused_imports)]
pub use location_service::{
    CreateConnectionRequest, CreateLocationRequest, LocationService, LocationServiceImpl,
    UpdateLocationRequest,
};

// Re-export suggestion service types
pub use suggestion_service::{
    SuggestionContext, SuggestionService, SuggestionType,
};

// Re-export workflow service
pub use workflow_service::WorkflowService;

// Re-export tool execution service types
#[allow(unused_imports)]
pub use tool_execution_service::{
    ToolExecutionService,
};

// Re-export story event service
pub use story_event_service::StoryEventService;
