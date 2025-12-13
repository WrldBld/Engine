//! ComfyUI port - Interface for image generation services

use anyhow::Result;
use async_trait::async_trait;

/// Response from queueing a prompt
#[derive(Debug, Clone)]
pub struct QueuePromptResponse {
    pub prompt_id: String,
}

/// Image info from generation history
#[derive(Debug, Clone)]
pub struct GeneratedImage {
    pub filename: String,
    pub subfolder: String,
    pub r#type: String,
}

/// Output from a generation node
#[derive(Debug, Clone)]
pub struct NodeOutput {
    pub images: Option<Vec<GeneratedImage>>,
}

/// Status of a prompt
#[derive(Debug, Clone)]
pub struct PromptStatus {
    pub completed: bool,
}

/// History entry for a prompt
#[derive(Debug, Clone)]
pub struct PromptHistory {
    pub status: PromptStatus,
    pub outputs: std::collections::HashMap<String, NodeOutput>,
}

/// Full history response
#[derive(Debug, Clone)]
pub struct HistoryResponse {
    pub prompts: std::collections::HashMap<String, PromptHistory>,
}

/// Port for ComfyUI image generation services
#[async_trait]
pub trait ComfyUIPort: Send + Sync {
    /// Queue a workflow prompt for generation
    async fn queue_prompt(&self, workflow: serde_json::Value) -> Result<QueuePromptResponse>;

    /// Get history/status for a prompt
    async fn get_history(&self, prompt_id: &str) -> Result<HistoryResponse>;

    /// Download a generated image
    async fn get_image(&self, filename: &str, subfolder: &str, folder_type: &str) -> Result<Vec<u8>>;
}
