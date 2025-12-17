//! ComfyUI configuration DTO for HTTP boundary.
//!
//! The domain `ComfyUIConfig` has serde for internal use, but HTTP handlers
//! should use this DTO to maintain boundary separation and follow hexagonal
//! architecture principles.

use serde::{Deserialize, Serialize};
use crate::domain::value_objects::ComfyUIConfig;

/// DTO for ComfyUI retry behavior and timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComfyUIConfigDto {
    /// Maximum number of retries (1-5)
    pub max_retries: u8,
    /// Base delay in seconds for exponential backoff (1-30)
    pub base_delay_seconds: u8,
    /// Timeout for queue_prompt operations in seconds
    pub queue_timeout_seconds: u16,
    /// Timeout for get_history operations in seconds
    pub history_timeout_seconds: u16,
    /// Timeout for get_image operations in seconds
    pub image_timeout_seconds: u16,
}

impl From<ComfyUIConfig> for ComfyUIConfigDto {
    fn from(config: ComfyUIConfig) -> Self {
        Self {
            max_retries: config.max_retries,
            base_delay_seconds: config.base_delay_seconds,
            queue_timeout_seconds: config.queue_timeout_seconds,
            history_timeout_seconds: config.history_timeout_seconds,
            image_timeout_seconds: config.image_timeout_seconds,
        }
    }
}

impl From<ComfyUIConfigDto> for ComfyUIConfig {
    fn from(dto: ComfyUIConfigDto) -> Self {
        Self {
            max_retries: dto.max_retries,
            base_delay_seconds: dto.base_delay_seconds,
            queue_timeout_seconds: dto.queue_timeout_seconds,
            history_timeout_seconds: dto.history_timeout_seconds,
            image_timeout_seconds: dto.image_timeout_seconds,
        }
    }
}
