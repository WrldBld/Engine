//! LLM port - Interface for language model interactions

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Port for LLM (Large Language Model) interactions
#[async_trait]
pub trait LlmPort: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Generate a response from the LLM
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, Self::Error>;

    /// Generate a response with tool/function calling support
    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, Self::Error>;
}

/// Request to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// The conversation history
    pub messages: Vec<ChatMessage>,
    /// System prompt / context
    pub system_prompt: Option<String>,
    /// Temperature for response generation (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Optional images for multimodal models
    pub images: Vec<ImageData>,
}

impl LlmRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            images: Vec::new(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_image(mut self, image: ImageData) -> Self {
        self.images.push(image);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: Option<u32>) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Image data for multimodal requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// Base64-encoded image data
    pub data: String,
    /// MIME type (e.g., "image/png")
    pub media_type: String,
}

/// Response from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// The generated text content
    pub content: String,
    /// Tool calls proposed by the model
    pub tool_calls: Vec<ToolCall>,
    /// Finish reason
    pub finish_reason: FinishReason,
    /// Token usage
    pub usage: Option<TokenUsage>,
}

/// Definition of a tool the LLM can call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A tool call proposed by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Reason the generation finished
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
