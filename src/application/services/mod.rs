//! Application services - Use case implementations
//!
//! This module contains the application services that implement the use cases
//! for the WrldBldr Engine. Each service follows hexagonal architecture principles,
//! accepting repository dependencies and returning domain entities or DTOs.

pub mod asset_generation_queue_service;
pub mod asset_service;
pub mod challenge_outcome_approval_service;
pub mod challenge_resolution_service;
pub mod session_join_service;
pub mod challenge_service;
pub mod dm_approval_queue_service;
pub mod character_service;
pub mod dm_action_queue_service;
pub mod event_chain_service;
pub mod generation_event_publisher;
pub mod generation_service;
pub mod generation_queue_projection_service;
pub mod interaction_service;
pub mod llm_queue_service;
pub mod llm;
pub mod llm_context_service;
pub mod location_service;

// Re-export LLM service types for backward compatibility
pub mod narrative_event_service;
pub mod narrative_event_approval_service;
pub mod outcome_suggestion_service;
pub mod outcome_trigger_service;
pub mod player_action_queue_service;
pub mod player_character_service;
pub mod relationship_service;
pub mod scene_resolution_service;
pub mod scene_service;
pub mod settings_service;
pub mod sheet_template_service;
pub mod skill_service;
pub mod story_event_service;
pub mod suggestion_service;
pub mod tool_execution_service;
pub mod trigger_evaluation_service;
pub mod event_effect_executor;
pub mod presence_service;
pub mod workflow_config_service;
pub mod workflow_service;
pub mod world_service;

// Note: ActiveNarrativeEventContext and GamePromptRequest are now in domain::value_objects

// Re-export world service types (used in HTTP routes and websocket)
pub use world_service::{
    CreateActRequest, CreateWorldRequest, UpdateWorldRequest, WorldService, WorldServiceImpl,
};

// Re-export session join service types
pub use session_join_service::SessionJoinService;

// Re-export scene service types
pub use scene_service::{
    CreateSceneRequest, SceneService, SceneServiceImpl, UpdateSceneRequest,
};

// Re-export scene resolution service types
pub use scene_resolution_service::{
    SceneResolutionService, SceneResolutionServiceImpl,
};

// Re-export character service types
pub use character_service::{
    ChangeArchetypeRequest, CharacterService, CharacterServiceImpl,
    CreateCharacterRequest, UpdateCharacterRequest,
};

// Re-export player character service types
pub use player_character_service::{
    CreatePlayerCharacterRequest, PlayerCharacterService, PlayerCharacterServiceImpl,
    UpdatePlayerCharacterRequest,
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

// Re-export narrative event approval service
pub use narrative_event_approval_service::NarrativeEventApprovalService;

// Re-export skill service types
pub use skill_service::{
    CreateSkillRequest, SkillService, SkillServiceImpl, UpdateSkillRequest,
};

// Re-export interaction service types (used in HTTP routes)
pub use interaction_service::{InteractionService, InteractionServiceImpl};

// Re-export challenge service types (used in HTTP routes)
pub use challenge_service::{ChallengeService, ChallengeServiceImpl};

// Re-export challenge resolution service

// Re-export relationship service types
pub use relationship_service::{RelationshipService, RelationshipServiceImpl};

// Re-export asset service types (used in HTTP routes)
pub use asset_service::{AssetService, AssetServiceImpl, CreateAssetRequest};

// Re-export sheet template service types
pub use sheet_template_service::SheetTemplateService;

// Re-export settings service types
pub use settings_service::SettingsService;

// Re-export narrative event service types (used in HTTP routes)
pub use narrative_event_service::{NarrativeEventService, NarrativeEventServiceImpl};

// Re-export event chain service types (used in HTTP routes)
pub use event_chain_service::{EventChainService, EventChainServiceImpl};

// Re-export queue service types (used in infrastructure layer)
pub use asset_generation_queue_service::AssetGenerationQueueService;
pub use dm_action_queue_service::DMActionQueueService;
pub use dm_approval_queue_service::DMApprovalQueueService;
pub use generation_event_publisher::GenerationEventPublisher;
pub use llm_queue_service::LLMQueueService;
pub use player_action_queue_service::PlayerActionQueueService;

// Re-export generation queue projection service (snapshot types are used by HTTP layer)
pub use generation_queue_projection_service::{
    GenerationQueueProjectionService,
    GenerationQueueSnapshot,
};

// Re-export outcome trigger service
pub use outcome_trigger_service::OutcomeTriggerService;

// Re-export challenge outcome approval service (P3.3)
pub use challenge_outcome_approval_service::{
    ChallengeOutcomeApprovalService, ChallengeOutcomeError,
};

// Re-export outcome suggestion service (P3.3)
pub use outcome_suggestion_service::{OutcomeSuggestionService, SuggestionError};

// Re-export LLM context service (Phase 1)
pub use llm_context_service::{
    LLMContextService, LLMContextError,
    SummarizationPlanner, SummarizationRequest, SummarizationResult, SummarizationPrompts,
};

// Re-export trigger evaluation service (Phase 2)
pub use trigger_evaluation_service::{
    TriggerEvaluationService, TriggerEvaluationError, TriggerEvaluationResult,
    TriggeredEventCandidate, TriggerSource, GameStateSnapshot, ImmediateContext,
    CompletedChallenge, CompletedNarrativeEvent,
};

// Re-export event effect executor (Phase 2)
pub use event_effect_executor::{
    EventEffectExecutor, EffectExecutionError, EffectExecutionResult, OutcomeExecutionResult,
};

// Re-export presence service (Phase 23C)
pub use presence_service::{NpcPresenceResult, PresenceService, PresenceServiceConfig};

// Note: PlayerActionService and ApprovalService were removed - functionality moved to queue services
