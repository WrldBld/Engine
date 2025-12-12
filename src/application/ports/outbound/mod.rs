//! Outbound ports - Interfaces that the application requires from external systems

mod llm_port;
mod repository_port;

pub use llm_port::{
    ChatMessage, FinishReason, LlmPort, LlmRequest, LlmResponse, MessageRole,
    TokenUsage, ToolCall, ToolDefinition,
};
