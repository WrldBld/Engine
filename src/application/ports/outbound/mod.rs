//! Outbound ports - Interfaces that the application requires from external systems

mod comfyui_port;
mod game_session_port;
mod llm_port;
mod repository_port;
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
    AssetRepositoryPort, CharacterRepositoryPort, GridMapRepositoryPort,
    InteractionRepositoryPort, LocationRepositoryPort, RelationshipRepositoryPort,
    RepositoryProvider, SceneRepositoryPort, SkillRepositoryPort, StoryEventRepositoryPort,
    WorldRepositoryPort,
};

pub use world_exporter_port::{
    CharacterData, ExportOptions, LocationData, PlayerWorldSnapshot, SceneData,
    WorldData, WorldExporterPort,
};
