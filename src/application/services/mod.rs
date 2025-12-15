//! Application services - Use case implementations
//!
//! This module contains the application services that implement the use cases
//! for the WrldBldr Engine. Each service follows hexagonal architecture principles,
//! accepting repository dependencies and returning domain entities or DTOs.

pub mod approval_service;
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
pub mod player_action_service;
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

// Re-export workflow services
pub use workflow_config_service::WorkflowConfigService;
pub use workflow_service::WorkflowService;

// Re-export tool execution service types
#[allow(unused_imports)]
pub use tool_execution_service::{
    ToolExecutionService,
};

// Re-export story event service
pub use story_event_service::StoryEventService;

// Re-export skill service types
#[allow(unused_imports)]
pub use skill_service::{
    CreateSkillRequest, SkillService, SkillServiceImpl, UpdateSkillRequest,
};

// Re-export interaction service types
#[allow(unused_imports)]
pub use interaction_service::{InteractionService, InteractionServiceImpl};

// Re-export challenge service types
#[allow(unused_imports)]
pub use challenge_service::{ChallengeService, ChallengeServiceImpl};

// Re-export relationship service types
#[allow(unused_imports)]
pub use relationship_service::{RelationshipService, RelationshipServiceImpl};

// Re-export asset service types
#[allow(unused_imports)]
pub use asset_service::{
    AssetService, AssetServiceImpl, CreateAssetRequest, UpdateAssetLabelRequest,
};

// Re-export sheet template service types
pub use sheet_template_service::SheetTemplateService;

// Re-export narrative event service types
#[allow(unused_imports)]
pub use narrative_event_service::{NarrativeEventService, NarrativeEventServiceImpl};

// Re-export event chain service types
#[allow(unused_imports)]
pub use event_chain_service::{EventChainService, EventChainServiceImpl};

// Re-export player action service types
#[allow(unused_imports)]
pub use player_action_service::{PlayerActionError, PlayerActionResult, PlayerActionService};

// Re-export approval service types
#[allow(unused_imports)]
pub use approval_service::{ApprovalDecision, ApprovalError, ApprovalResult, ApprovalService};
