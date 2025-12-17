//! ComfyUI client for AI asset generation

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use crate::application::ports::outbound::{
    ComfyUIPort, GeneratedImage, HistoryResponse as PortHistoryResponse, NodeOutput as PortNodeOutput,
    PromptHistory as PortPromptHistory, PromptStatus as PortPromptStatus, QueuePromptResponse,
};
use crate::domain::value_objects::ComfyUIConfig;

// =============================================================================
// Circuit Breaker Constants
// =============================================================================

/// Number of consecutive failures before opening the circuit breaker
const CIRCUIT_BREAKER_FAILURE_THRESHOLD: u8 = 5;

/// Duration in seconds to keep circuit breaker open before allowing retry
const CIRCUIT_BREAKER_OPEN_DURATION_SECS: i64 = 60;

/// Duration in seconds for health check cache validity
const HEALTH_CHECK_CACHE_TTL_SECS: i64 = 30;

/// Connection state for ComfyUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ComfyUIConnectionState {
    Connected,
    Degraded { consecutive_failures: u8 },
    Disconnected,
    CircuitOpen { until: DateTime<Utc> },
}

/// Circuit breaker state
#[derive(Debug, Clone)]
enum CircuitBreakerState {
    Closed,
    Open { until: DateTime<Utc> },
    HalfOpen,
}

/// Circuit breaker for ComfyUI operations
#[derive(Debug, Clone)]
struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    failure_count: Arc<Mutex<u8>>,
    last_failure: Arc<Mutex<Option<DateTime<Utc>>>>,
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            last_failure: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if circuit is open (should reject requests)
    fn is_open(&self) -> bool {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        match *state {
            CircuitBreakerState::Open { until } => {
                if Utc::now() < until {
                    true
                } else {
                    // Circuit should transition to half-open, but we'll check on next call
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if circuit allows requests (closed or half-open)
    fn check_circuit(&self) -> Result<(), ComfyUIError> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        match *state {
            CircuitBreakerState::Open { until } => {
                if Utc::now() >= until {
                    // Transition to half-open
                    *state = CircuitBreakerState::HalfOpen;
                    Ok(())
                } else {
                    Err(ComfyUIError::CircuitOpen)
                }
            }
            _ => Ok(()),
        }
    }

    /// Record a successful operation
    fn record_success(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let mut failure_count = self.failure_count.lock().unwrap_or_else(|p| p.into_inner());
        *failure_count = 0;
        *state = CircuitBreakerState::Closed;
    }

    /// Record a failed operation
    fn record_failure(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let mut failure_count = self.failure_count.lock().unwrap_or_else(|p| p.into_inner());
        *failure_count += 1;
        *self.last_failure.lock().unwrap_or_else(|p| p.into_inner()) = Some(Utc::now());

        if *failure_count >= CIRCUIT_BREAKER_FAILURE_THRESHOLD {
            *state = CircuitBreakerState::Open {
                until: Utc::now() + chrono::Duration::seconds(CIRCUIT_BREAKER_OPEN_DURATION_SECS),
            };
        }
    }
}

/// Client for ComfyUI API
#[derive(Clone)]
pub struct ComfyUIClient {
    client: Client,
    base_url: String,
    config: Arc<Mutex<ComfyUIConfig>>,
    circuit_breaker: Arc<CircuitBreaker>,
    last_health_check: Arc<Mutex<Option<(DateTime<Utc>, bool)>>>,
}

impl ComfyUIClient {
    pub fn new(base_url: &str) -> Self {
        Self::with_config(base_url, ComfyUIConfig::default())
    }

    pub fn with_config(base_url: &str, config: ComfyUIConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            config: Arc::new(Mutex::new(config)),
            circuit_breaker: Arc::new(CircuitBreaker::new()),
            last_health_check: Arc::new(Mutex::new(None)),
        }
    }

    /// Get a copy of the current config
    pub fn config(&self) -> ComfyUIConfig {
        self.config.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Update the config
    pub fn update_config(&self, new_config: ComfyUIConfig) -> Result<(), String> {
        new_config.validate()?;
        *self.config.lock().unwrap_or_else(|p| p.into_inner()) = new_config;
        Ok(())
    }

    /// Get cached health check result or perform new check
    async fn cached_health_check(&self) -> Result<bool, ComfyUIError> {
        // Check cache first (5 second TTL)
        {
            let cache = self.last_health_check.lock().unwrap_or_else(|p| p.into_inner());
            if let Some((timestamp, result)) = cache.as_ref() {
                let age = Utc::now().signed_duration_since(*timestamp);
                if age.num_seconds() < 5 {
                    return Ok(*result);
                }
            }
        }

        // Perform health check
        let result = self.health_check_internal().await;
        let is_healthy = result.is_ok() && result.unwrap_or(false);

        // Update cache
        {
            let mut cache = self.last_health_check.lock().unwrap_or_else(|p| p.into_inner());
            *cache = Some((Utc::now(), is_healthy));
        }

        if is_healthy {
            Ok(true)
        } else {
            Err(ComfyUIError::ServiceUnavailable)
        }
    }

    /// Internal health check implementation
    async fn health_check_internal(&self) -> Result<bool, ComfyUIError> {
        let response = self
            .client
            .get(format!("{}/system_stats", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Retry wrapper with exponential backoff
    async fn with_retry<T, F, Fut>(&self, operation: F) -> Result<T, ComfyUIError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, ComfyUIError>>,
    {
        let mut last_error: Option<String> = None;

        let config = self.config.lock().unwrap_or_else(|p| p.into_inner()).clone();
        for attempt in 0..=config.max_retries {
            // Check circuit breaker
            if let Err(e) = self.circuit_breaker.check_circuit() {
                return Err(e);
            }

            match operation().await {
                Ok(result) => {
                    self.circuit_breaker.record_success();
                    return Ok(result);
                }
                Err(e) => {
                    // Don't retry on non-transient errors
                    match &e {
                        ComfyUIError::ApiError(_) => {
                            // 4xx errors are not retryable
                            self.circuit_breaker.record_failure();
                            return Err(e);
                        }
                        ComfyUIError::ServiceUnavailable
                        | ComfyUIError::HttpError(_)
                        | ComfyUIError::Timeout(_) => {
                            // These are retryable
                            last_error = Some(format!("{:?}", e));
                        }
                        ComfyUIError::CircuitOpen => {
                            return Err(e);
                        }
                        ComfyUIError::MaxRetriesExceeded(_) => {
                            return Err(e);
                        }
                    }

                    // If this was the last attempt, return error
                    if attempt >= config.max_retries {
                        self.circuit_breaker.record_failure();
                        return Err(ComfyUIError::MaxRetriesExceeded(config.max_retries));
                    }

                    // Exponential backoff: base * 3^attempt
                    let delay_seconds = config.base_delay_seconds as u64 * 3_u64.pow(attempt as u32);
                    sleep(Duration::from_secs(delay_seconds)).await;
                }
            }
        }

        let config = self.config.lock().unwrap_or_else(|p| p.into_inner()).clone();
        self.circuit_breaker.record_failure();
        Err(ComfyUIError::MaxRetriesExceeded(config.max_retries))
    }

    /// Get current connection state
    pub fn connection_state(&self) -> ComfyUIConnectionState {
        if self.circuit_breaker.is_open() {
            let state = self.circuit_breaker.state.lock().unwrap_or_else(|p| p.into_inner());
            if let CircuitBreakerState::Open { until } = *state {
                return ComfyUIConnectionState::CircuitOpen { until };
            }
        }

        let cache = self.last_health_check.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((timestamp, is_healthy)) = cache.as_ref() {
            let age = Utc::now().signed_duration_since(*timestamp);
            if age.num_seconds() < HEALTH_CHECK_CACHE_TTL_SECS {
                if *is_healthy {
                    ComfyUIConnectionState::Connected
                } else {
                    ComfyUIConnectionState::Disconnected
                }
            } else {
                ComfyUIConnectionState::Disconnected
            }
        } else {
            ComfyUIConnectionState::Disconnected
        }
    }

    /// Queue a workflow for execution
    pub async fn queue_prompt(
        &self,
        workflow: serde_json::Value,
    ) -> Result<QueueResponse, ComfyUIError> {
        // Health check before queuing
        self.cached_health_check().await?;

        let config = self.config.lock().unwrap_or_else(|p| p.into_inner()).clone();
        self.with_retry(|| {
            let client = self.client.clone();
            let base_url = self.base_url.clone();
            let config = config.clone();
            let workflow = workflow.clone();
            async move {
                let client_id = Uuid::new_v4().to_string();
                let request = QueuePromptRequest {
                    prompt: workflow,
                    client_id: client_id.clone(),
                };

                let response = client
                    .post(format!("{}/prompt", base_url))
                    .json(&request)
                    .timeout(Duration::from_secs(config.queue_timeout_seconds as u64))
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_timeout() {
                            ComfyUIError::Timeout(config.queue_timeout_seconds)
                        } else {
                            ComfyUIError::HttpError(e)
                        }
                    })?;

                if !response.status().is_success() {
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(ComfyUIError::ApiError(error_text));
                }

                let queue_response: QueueResponse = response.json().await.map_err(|e| {
                    ComfyUIError::HttpError(reqwest::Error::from(e))
                })?;
                Ok(queue_response)
            }
        })
        .await
    }

    /// Get the history of a completed prompt
    pub async fn get_history(&self, prompt_id: &str) -> Result<HistoryResponse, ComfyUIError> {
        let prompt_id = prompt_id.to_string();
        let config = self.config.lock().unwrap_or_else(|p| p.into_inner()).clone();
        self.with_retry(|| {
            let client = self.client.clone();
            let base_url = self.base_url.clone();
            let config = config.clone();
            let prompt_id = prompt_id.clone();
            async move {
                let response = client
                    .get(format!("{}/history/{}", base_url, prompt_id))
                    .timeout(Duration::from_secs(config.history_timeout_seconds as u64))
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_timeout() {
                            ComfyUIError::Timeout(config.history_timeout_seconds)
                        } else {
                            ComfyUIError::HttpError(e)
                        }
                    })?;

                if !response.status().is_success() {
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(ComfyUIError::ApiError(error_text));
                }

                let history: HistoryResponse = response.json().await.map_err(|e| {
                    ComfyUIError::HttpError(reqwest::Error::from(e))
                })?;
                Ok(history)
            }
        })
        .await
    }

    /// Download a generated image
    pub async fn get_image(
        &self,
        filename: &str,
        subfolder: &str,
        folder_type: &str,
    ) -> Result<Vec<u8>, ComfyUIError> {
        let filename = filename.to_string();
        let subfolder = subfolder.to_string();
        let folder_type = folder_type.to_string();
        let config = self.config.lock().unwrap_or_else(|p| p.into_inner()).clone();
        self.with_retry(|| {
            let client = self.client.clone();
            let base_url = self.base_url.clone();
            let config = config.clone();
            let filename = filename.clone();
            let subfolder = subfolder.clone();
            let folder_type = folder_type.clone();
            async move {
                let response = client
                    .get(format!("{}/view", base_url))
                    .query(&[
                        ("filename", filename.as_str()),
                        ("subfolder", subfolder.as_str()),
                        ("type", folder_type.as_str()),
                    ])
                    .timeout(Duration::from_secs(config.image_timeout_seconds as u64))
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_timeout() {
                            ComfyUIError::Timeout(config.image_timeout_seconds)
                        } else {
                            ComfyUIError::HttpError(e)
                        }
                    })?;

                if !response.status().is_success() {
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(ComfyUIError::ApiError(error_text));
                }

                let bytes = response.bytes().await.map_err(|e| {
                    ComfyUIError::HttpError(reqwest::Error::from(e))
                })?;
                Ok(bytes.to_vec())
            }
        })
        .await
    }

    /// Check if the server is available (public method, uses caching)
    pub async fn health_check(&self) -> Result<bool, ComfyUIError> {
        self.cached_health_check().await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ComfyUIError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("ComfyUI service unavailable")]
    ServiceUnavailable,
    #[error("Circuit breaker open - ComfyUI failing")]
    CircuitOpen,
    #[error("Request timeout after {0} seconds")]
    Timeout(u16),
    #[error("Max retries ({0}) exceeded")]
    MaxRetriesExceeded(u8),
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
