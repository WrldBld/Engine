//! Application services - Use case implementations
//!
//! This module contains the application services that implement the use cases
//! for the WrldBldr Engine. Each service follows hexagonal architecture principles,
//! accepting repository dependencies and returning domain entities or DTOs.

pub mod asset_generation_queue_service;
pub mod asset_service;
pub mod challenge_service;
pub mod dm_approval_queue_service;
pub mod character_service;
pub mod dm_action_queue_service;
pub mod event_chain_service;
pub mod generation_service;
pub mod interaction_service;
pub mod llm_queue_service;
pub mod llm_service;
pub mod location_service;
pub mod narrative_event_service;
pub mod player_action_queue_service;
pub mod relationship_service;
pub mod scene_service;
pub mod sheet_template_service;
pub mod skill_service;
pub mod story_event_service;
pub mod suggestion_service;
pub mod tool_execution_service;
pub mod workflow_config_service;
pub mod workflow_service;
pub mod world_service;

// Note: ActiveNarrativeEventContext and GamePromptRequest are now in domain::value_objects

// Re-export world service types (used in HTTP routes and websocket)
pub use world_service::{
    CreateActRequest, CreateWorldRequest, UpdateWorldRequest, WorldService, WorldServiceImpl,
};

// Re-export scene service types
pub use scene_service::{
    CreateSceneRequest, SceneService, SceneServiceImpl, UpdateSceneRequest,
};

// Re-export character service types
pub use character_service::{
    ChangeArchetypeRequest, CharacterService, CharacterServiceImpl,
    CreateCharacterRequest, UpdateCharacterRequest,
};

// Re-export location service types
pub use location_service::{
    CreateConnectionRequest, CreateLocationRequest, LocationService, LocationServiceImpl,
    UpdateLocationRequest,
};

// Re-export suggestion service types
pub use suggestion_service::{
    SuggestionContext, SuggestionService, SuggestionType,
};

// Re-export workflow services
pub use workflow_config_service::WorkflowConfigService;
pub use workflow_service::WorkflowService;

// ToolExecutionService is only used internally within services module, not re-exported

// Re-export story event service
pub use story_event_service::StoryEventService;

// Re-export skill service types
pub use skill_service::{
    CreateSkillRequest, SkillService, SkillServiceImpl, UpdateSkillRequest,
};

// Re-export interaction service types (used in HTTP routes)
pub use interaction_service::{InteractionService, InteractionServiceImpl};

// Re-export challenge service types (used in HTTP routes)
pub use challenge_service::{ChallengeService, ChallengeServiceImpl};

// Re-export relationship service types
pub use relationship_service::{RelationshipService, RelationshipServiceImpl};

// Re-export asset service types (used in HTTP routes)
pub use asset_service::{AssetService, AssetServiceImpl, CreateAssetRequest};

// Re-export sheet template service types
pub use sheet_template_service::SheetTemplateService;

// Re-export narrative event service types (used in HTTP routes)
pub use narrative_event_service::{NarrativeEventService, NarrativeEventServiceImpl};

// Re-export event chain service types (used in HTTP routes)
pub use event_chain_service::{EventChainService, EventChainServiceImpl};

// Re-export queue service types (used in infrastructure layer)
pub use asset_generation_queue_service::AssetGenerationQueueService;
pub use dm_action_queue_service::DMActionQueueService;
pub use dm_approval_queue_service::DMApprovalQueueService;
pub use llm_queue_service::LLMQueueService;
pub use player_action_queue_service::PlayerActionQueueService;

// Note: PlayerActionService and ApprovalService were removed - functionality moved to queue services
