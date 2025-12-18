//! Data Transfer Objects (DTOs) - Application layer data structures
//!
//! DTOs are used for data transfer between layers (HTTP routes, services, etc.)
//! They provide a stable API that is decoupled from domain entities.

mod app_events;
mod asset;
mod challenge;
mod character;
mod comfyui_config;
mod event_chain;
mod export;
mod interaction;
mod item;
mod location;
mod narrative_event;
mod queue_items;
mod rule_system;
mod scene;
mod sheet_template;
mod session_info;
mod skill;
mod story_event;
mod suggestion;
mod workflow;
mod world;
mod world_snapshot;

// Application events (published through event bus)
pub use app_events::AppEvent;

// Queue items (used by queue services)
pub use queue_items::{
    ApprovalItem, AssetGenerationItem, ChallengeOutcomeApprovalItem, ChallengeSuggestionInfo,
    DMAction, DMActionItem, DecisionType, DecisionUrgency, LLMRequestItem,
    LLMRequestType, NarrativeEventSuggestionInfo, PlayerActionItem,
};

// Asset DTOs
pub use asset::{
    parse_asset_type, parse_entity_type, GalleryAssetResponseDto, GenerateAssetRequestDto,
    GenerationBatchResponseDto, SelectFromBatchRequestDto, UpdateAssetLabelRequestDto,
    UploadAssetRequestDto,
};

// Challenge DTOs
pub use challenge::{
    AdHocOutcomesDto, ChallengeOutcomeApprovalRequest, ChallengeOutcomeDecision,
    ChallengeOutcomePendingNotification, ChallengeResolvedNotification,
    ChallengeResponseDto, ChallengeRollSubmittedNotification, CreateChallengeRequestDto,
    DifficultyRequestDto, OutcomeBranchDto, OutcomeBranchResponse, OutcomeBranchSelectionRequest,
    OutcomeBranchesReadyNotification, OutcomeRequestDto, OutcomeSuggestionReadyNotification,
    OutcomeSuggestionRequest, OutcomeSuggestionResponse, OutcomesRequestDto,
    OutcomeTriggerRequestDto, PendingChallengeResolutionDto, TriggerConditionRequestDto,
    UpdateChallengeRequestDto,
};

// Rule system DTOs
pub use rule_system::{
    parse_system_type, parse_variant, RuleSystemConfigDto,
    RuleSystemPresetDetailsDto, RuleSystemPresetSummaryDto, RuleSystemSummaryDto,
    RuleSystemTypeDetailsDto, RuleSystemVariantDto,
};

// Workflow DTOs
pub use workflow::{
    parse_workflow_slot, AnalyzeWorkflowRequestDto, CreateWorkflowConfigRequestDto,
    ImportWorkflowsRequestDto, ImportWorkflowsResponseDto, InputDefaultDto,
    PromptMappingDto, TestWorkflowRequestDto, TestWorkflowResponseDto,
    UpdateWorkflowDefaultsRequestDto, WorkflowAnalysisResponseDto,
    WorkflowConfigExportDto, WorkflowConfigFullResponseDto, WorkflowConfigResponseDto,
    WorkflowSlotCategoryDto, WorkflowSlotStatusDto, WorkflowSlotsResponseDto,
};

// Scene DTOs
pub use scene::{CreateSceneRequestDto, SceneResponseDto, UpdateNotesRequestDto};

// Story event DTOs
pub use story_event::{
    CreateDmMarkerRequestDto,
    ListStoryEventsQueryDto, PaginatedStoryEventsResponseDto,
    StoryEventResponseDto, UpdateStoryEventRequestDto,
};

// Narrative event DTOs
pub use narrative_event::{
    CreateNarrativeEventRequestDto, ListNarrativeEventsQueryDto, NarrativeEventResponseDto,
    UpdateNarrativeEventRequestDto,
};

// Event chain DTOs
pub use event_chain::{
    AddEventRequestDto, ChainStatusResponseDto, CreateEventChainRequestDto,
    EventChainResponseDto, UpdateEventChainRequestDto,
};

// Location DTOs
pub use location::{
    parse_location_type, ConnectionResponseDto, CreateConnectionRequestDto,
    CreateLocationRequestDto, CreateRegionRequestDto, LocationResponseDto,
    MapBoundsDto, RegionResponseDto,
};

// Character DTOs
pub use character::{
    parse_archetype, parse_relationship_type, ChangeArchetypeRequestDto, CharacterResponseDto,
    CreateCharacterRequestDto, CreateRelationshipRequestDto, CreatedIdResponseDto,
};

// Item DTOs
pub use item::{
    parse_acquisition_method, AddInventoryItemRequestDto, CreateItemRequestDto,
    InventoryItemResponseDto, ItemResponseDto, UpdateInventoryItemRequestDto,
};

// ComfyUI config DTO
pub use comfyui_config::ComfyUIConfigDto;

// Skill DTOs
pub use skill::{
    CreateSkillRequestDto, SkillResponseDto, UpdateSkillRequestDto,
};

// Interaction DTOs
pub use interaction::{
    parse_interaction_type, parse_target, CreateInteractionRequestDto, InteractionResponseDto,
    SetAvailabilityRequestDto,
};

// Sheet template DTOs
pub use sheet_template::{
    CreateFieldRequestDto, CreateSectionRequestDto, SheetTemplateResponseDto,
    SheetTemplateStorageDto, SheetTemplateSummaryDto,
};

// Session DTOs
pub use session_info::SessionInfo;

// Export DTOs
pub use export::ExportQueryDto;

// Suggestion DTOs
pub use suggestion::{SuggestionRequestDto, UnifiedSuggestionRequestDto};

// World DTOs
pub use world::{
    parse_monomyth_stage, ActResponseDto, CreateActRequestDto, CreateWorldRequestDto,
    UpdateWorldRequestDto, WorldResponseDto,
};

// World snapshot DTO (for session management)
pub use world_snapshot::WorldSnapshot;
