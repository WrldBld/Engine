//! Outbound ports - Interfaces that the application requires from external systems

mod comfyui_port;
mod game_session_port;
mod llm_port;
mod repository_port;
mod session_management_port;
mod world_exporter_port;

pub use comfyui_port::{
    ComfyUIPort, GeneratedImage, HistoryResponse, NodeOutput, PromptHistory,
    PromptStatus, QueuePromptResponse,
};

pub use game_session_port::GameSessionPort;

pub use llm_port::{
    ChatMessage, FinishReason, LlmPort, LlmRequest, LlmResponse, MessageRole,
    TokenUsage, ToolCall, ToolDefinition,
};

pub use repository_port::{
    AssetRepositoryPort, ChallengeRepositoryPort, CharacterNode, CharacterRepositoryPort,
    EventChainRepositoryPort, GridMapRepositoryPort, InteractionRepositoryPort,
    LocationRepositoryPort, NarrativeEventRepositoryPort, RelationshipEdge,
    RelationshipRepositoryPort, RepositoryProvider, SceneRepositoryPort,
    SheetTemplateRepositoryPort, SkillRepositoryPort, SocialNetwork, StoryEventRepositoryPort,
    WorkflowRepositoryPort, WorldRepositoryPort,
};

pub use session_management_port::{
    BroadcastMessage, CharacterContextInfo, ParticipantRoleDto, ParticipantSummary,
    PendingApprovalInfo, ProposedToolInfo, SessionJoinResult, SessionLifecyclePort,
    SessionManagementError, SessionManagementPort, SessionWorldContext,
};

pub use world_exporter_port::{
    CharacterData, ExportOptions, LocationData, PlayerWorldSnapshot, SceneData,
    WorldData, WorldExporterPort,
};
