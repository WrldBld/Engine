//! Outbound ports - Interfaces that the application requires from external systems

mod app_event_repository_port;
mod async_session_port;
mod comfyui_port;
mod event_bus_port;
mod llm_port;
mod queue_notification_port;
mod queue_port;
mod repository_port;
mod session_management_port;
mod settings_port;
mod world_exporter_port;
mod generation_read_state_port;

pub use app_event_repository_port::{AppEventRepositoryError, AppEventRepositoryPort};

pub use event_bus_port::{EventBusError, EventBusPort};

pub use comfyui_port::{
    ComfyUIPort, GeneratedImage, HistoryResponse, NodeOutput, PromptHistory,
    PromptStatus, QueuePromptResponse,
};

pub use llm_port::{
    ChatMessage, FinishReason, LlmPort, LlmRequest, LlmResponse, MessageRole,
    TokenUsage, ToolCall, ToolDefinition,
};

pub use repository_port::{
    AssetRepositoryPort, ChallengeRepositoryPort, CharacterNode, CharacterRepositoryPort,
    EventChainRepositoryPort, InteractionRepositoryPort, LocationRepositoryPort,
    NarrativeEventRepositoryPort, PlayerCharacterRepositoryPort, RegionRepositoryPort,
    RelationshipEdge, RelationshipRepositoryPort, SceneRepositoryPort, SheetTemplateRepositoryPort,
    SkillRepositoryPort, SocialNetwork, StoryEventRepositoryPort, WantRepositoryPort,
    WorkflowRepositoryPort, WorldRepositoryPort,
};

pub use session_management_port::{
    BroadcastMessage, CharacterContextInfo, PendingApprovalInfo, SessionManagementError,
    SessionManagementPort, SessionWorldContext,
};
// Note: ProposedToolInfo is now in domain::value_objects

pub use settings_port::{SettingsError, SettingsRepositoryPort};

pub use queue_notification_port::{QueueNotificationPort, WaitResult};

pub use queue_port::{
    ApprovalQueuePort, ProcessingQueuePort, QueueError, QueueItem, QueueItemStatus, QueuePort,
};
pub use crate::domain::value_objects::QueueItemId;

pub use world_exporter_port::{
    CharacterData, ExportOptions, LocationData, PlayerWorldSnapshot, SceneData,
    WorldData, WorldExporterPort,
};

pub use generation_read_state_port::{GenerationReadKind, GenerationReadStatePort};

pub use async_session_port::{
    AsyncSessionError, AsyncSessionPort, SessionJoinInfo,
    SessionParticipantInfo, SessionParticipantRole, SessionWorldData,
};
