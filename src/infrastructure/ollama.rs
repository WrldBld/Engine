//! Ollama LLM client (OpenAI-compatible API)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::application::ports::outbound::{
    FinishReason, LlmPort, LlmRequest, LlmResponse, MessageRole, TokenUsage, ToolCall,
    ToolDefinition,
};

/// Client for Ollama's OpenAI-compatible API
#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OllamaError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),
}

#[async_trait]
impl LlmPort for OllamaClient {
    type Error = OllamaError;

    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, Self::Error> {
        let api_request = OpenAIChatRequest {
            model: self.model.clone(),
            messages: build_messages(&request),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: None,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&api_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(OllamaError::ApiError(error_text));
        }

        let api_response: OpenAIChatResponse = response.json().await?;

        Ok(convert_response(api_response))
    }

    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, Self::Error> {
        let api_tools: Vec<OpenAITool> = tools
            .into_iter()
            .map(|t| OpenAITool {
                r#type: "function".to_string(),
                function: OpenAIFunction {
                    name: t.name,
                    description: t.description,
                    parameters: t.parameters,
                },
            })
            .collect();

        let api_request = OpenAIChatRequest {
            model: self.model.clone(),
            messages: build_messages(&request),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: Some(api_tools),
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&api_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(OllamaError::ApiError(error_text));
        }

        let api_response: OpenAIChatResponse = response.json().await?;

        Ok(convert_response(api_response))
    }
}

fn build_messages(request: &LlmRequest) -> Vec<OpenAIMessage> {
    let mut messages = Vec::new();

    if let Some(system) = &request.system_prompt {
        messages.push(OpenAIMessage {
            role: "system".to_string(),
            content: Some(system.clone()),
            tool_calls: None,
        });
    }

    for msg in &request.messages {
        messages.push(OpenAIMessage {
            role: match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
            }
            .to_string(),
            content: Some(msg.content.clone()),
            tool_calls: None,
        });
    }

    messages
}

fn convert_response(response: OpenAIChatResponse) -> LlmResponse {
    let choice = response.choices.into_iter().next().unwrap_or_default();

    let tool_calls = choice
        .message
        .tool_calls
        .unwrap_or_default()
        .into_iter()
        .map(|tc| ToolCall {
            id: tc.id,
            name: tc.function.name,
            arguments: serde_json::from_str(&tc.function.arguments).unwrap_or_default(),
        })
        .collect();

    let finish_reason = match choice.finish_reason.as_deref() {
        Some("stop") => FinishReason::Stop,
        Some("length") => FinishReason::Length,
        Some("tool_calls") => FinishReason::ToolCalls,
        Some("content_filter") => FinishReason::ContentFilter,
        _ => FinishReason::Stop,
    };

    LlmResponse {
        content: choice.message.content.unwrap_or_default(),
        tool_calls,
        finish_reason,
        usage: response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
    }
}

// OpenAI API types

#[derive(Debug, Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Serialize)]
struct OpenAITool {
    r#type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Serialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}
