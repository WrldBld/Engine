//! ComfyUI client for AI asset generation

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::ports::outbound::{
    ComfyUIPort, GeneratedImage, HistoryResponse as PortHistoryResponse, NodeOutput as PortNodeOutput,
    PromptHistory as PortPromptHistory, PromptStatus as PortPromptStatus, QueuePromptResponse,
};

/// Client for ComfyUI API
pub struct ComfyUIClient {
    client: Client,
    base_url: String,
}

impl ComfyUIClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Queue a workflow for execution
    pub async fn queue_prompt(
        &self,
        workflow: serde_json::Value,
    ) -> Result<QueueResponse, ComfyUIError> {
        let client_id = Uuid::new_v4().to_string();

        let request = QueuePromptRequest {
            prompt: workflow,
            client_id: client_id.clone(),
        };

        let response = self
            .client
            .post(format!("{}/prompt", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ComfyUIError::ApiError(error_text));
        }

        let queue_response: QueueResponse = response.json().await?;
        Ok(queue_response)
    }

    /// Get the history of a completed prompt
    pub async fn get_history(&self, prompt_id: &str) -> Result<HistoryResponse, ComfyUIError> {
        let response = self
            .client
            .get(format!("{}/history/{}", self.base_url, prompt_id))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ComfyUIError::ApiError(error_text));
        }

        let history: HistoryResponse = response.json().await?;
        Ok(history)
    }

    /// Download a generated image
    pub async fn get_image(
        &self,
        filename: &str,
        subfolder: &str,
        folder_type: &str,
    ) -> Result<Vec<u8>, ComfyUIError> {
        let response = self
            .client
            .get(format!("{}/view", self.base_url))
            .query(&[
                ("filename", filename),
                ("subfolder", subfolder),
                ("type", folder_type),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ComfyUIError::ApiError(error_text));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Check if the server is available
    pub async fn health_check(&self) -> Result<bool, ComfyUIError> {
        let response = self
            .client
            .get(format!("{}/system_stats", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ComfyUIError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API error: {0}")]
    ApiError(String),
}

#[derive(Debug, Serialize)]
struct QueuePromptRequest {
    prompt: serde_json::Value,
    client_id: String,
}

#[derive(Debug, Deserialize)]
pub struct QueueResponse {
    pub prompt_id: String,
    pub number: u32,
}

#[derive(Debug, Deserialize)]
pub struct HistoryResponse {
    #[serde(flatten)]
    pub prompts: std::collections::HashMap<String, PromptHistory>,
}

#[derive(Debug, Deserialize)]
pub struct PromptHistory {
    pub outputs: std::collections::HashMap<String, NodeOutput>,
    pub status: PromptStatus,
}

#[derive(Debug, Deserialize)]
pub struct NodeOutput {
    pub images: Option<Vec<ImageOutput>>,
}

#[derive(Debug, Deserialize)]
pub struct ImageOutput {
    pub filename: String,
    pub subfolder: String,
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct PromptStatus {
    pub status_str: String,
    pub completed: bool,
}

/// Types of workflows for asset generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowType {
    CharacterSprite,
    CharacterPortrait,
    SceneBackdrop,
    Tilesheet,
}

impl WorkflowType {
    /// Get the default workflow filename for this type
    pub fn workflow_file(&self) -> &'static str {
        match self {
            Self::CharacterSprite => "character_sprite.json",
            Self::CharacterPortrait => "character_portrait.json",
            Self::SceneBackdrop => "scene_backdrop.json",
            Self::Tilesheet => "tilesheet.json",
        }
    }
}

/// Request for asset generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub workflow_type: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    pub seed: Option<i64>,
}

impl GenerationRequest {
    pub fn character_sprite(prompt: impl Into<String>) -> Self {
        Self {
            workflow_type: "character_sprite".to_string(),
            prompt: prompt.into(),
            negative_prompt: None,
            width: 512,
            height: 512,
            seed: None,
        }
    }

    pub fn character_portrait(prompt: impl Into<String>) -> Self {
        Self {
            workflow_type: "character_portrait".to_string(),
            prompt: prompt.into(),
            negative_prompt: None,
            width: 256,
            height: 256,
            seed: None,
        }
    }

    pub fn scene_backdrop(prompt: impl Into<String>) -> Self {
        Self {
            workflow_type: "scene_backdrop".to_string(),
            prompt: prompt.into(),
            negative_prompt: None,
            width: 1920,
            height: 1080,
            seed: None,
        }
    }

    pub fn tilesheet(prompt: impl Into<String>) -> Self {
        Self {
            workflow_type: "tilesheet".to_string(),
            prompt: prompt.into(),
            negative_prompt: None,
            width: 512,
            height: 512,
            seed: None,
        }
    }
}

// =============================================================================
// ComfyUIPort Implementation
// =============================================================================

#[async_trait]
impl ComfyUIPort for ComfyUIClient {
    async fn queue_prompt(&self, workflow: serde_json::Value) -> Result<QueuePromptResponse> {
        // Call the inherent method using ComfyUIClient:: syntax to avoid recursion
        let response = ComfyUIClient::queue_prompt(self, workflow).await?;
        Ok(QueuePromptResponse {
            prompt_id: response.prompt_id,
        })
    }

    async fn get_history(&self, prompt_id: &str) -> Result<PortHistoryResponse> {
        // Call the inherent method using ComfyUIClient:: syntax to avoid recursion
        let response = ComfyUIClient::get_history(self, prompt_id).await?;

        // Convert infrastructure types to port types
        let prompts = response
            .prompts
            .into_iter()
            .map(|(id, history)| {
                let port_history = PortPromptHistory {
                    status: PortPromptStatus {
                        completed: history.status.completed,
                    },
                    outputs: history
                        .outputs
                        .into_iter()
                        .map(|(node_id, output)| {
                            let port_output = PortNodeOutput {
                                images: output.images.map(|images| {
                                    images
                                        .into_iter()
                                        .map(|img| GeneratedImage {
                                            filename: img.filename,
                                            subfolder: img.subfolder,
                                            r#type: img.r#type,
                                        })
                                        .collect()
                                }),
                            };
                            (node_id, port_output)
                        })
                        .collect(),
                };
                (id, port_history)
            })
            .collect();

        Ok(PortHistoryResponse { prompts })
    }

    async fn get_image(&self, filename: &str, subfolder: &str, folder_type: &str) -> Result<Vec<u8>> {
        // Call the inherent method using ComfyUIClient:: syntax to avoid recursion
        let image_data = ComfyUIClient::get_image(self, filename, subfolder, folder_type).await?;
        Ok(image_data)
    }
}
