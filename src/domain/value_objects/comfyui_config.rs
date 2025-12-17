//! ComfyUI Configuration - Value object for ComfyUI retry and timeout settings
//!
//! # Architectural Note (ADR-001: Domain Serialization)
//!
//! `ComfyUIConfig` includes serde derives because:
//! 1. It is loaded from configuration files at startup
//! 2. The config file format IS the domain specification
//! 3. This is a pure data structure with no behavior
//!
//! This is an accepted exception to the "no serde in domain" rule.

use serde::{Deserialize, Serialize};

/// Configuration for ComfyUI retry behavior and timeouts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComfyUIConfig {
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

impl Default for ComfyUIConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_seconds: 5,
            queue_timeout_seconds: 30,
            history_timeout_seconds: 10,
            image_timeout_seconds: 60,
        }
    }
}

impl ComfyUIConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=5).contains(&self.max_retries) {
            return Err("max_retries must be between 1 and 5".to_string());
        }
        if !(1..=30).contains(&self.base_delay_seconds) {
            return Err("base_delay_seconds must be between 1 and 30".to_string());
        }
        if self.queue_timeout_seconds == 0 {
            return Err("queue_timeout_seconds must be greater than 0".to_string());
        }
        if self.history_timeout_seconds == 0 {
            return Err("history_timeout_seconds must be greater than 0".to_string());
        }
        if self.image_timeout_seconds == 0 {
            return Err("image_timeout_seconds must be greater than 0".to_string());
        }
        Ok(())
    }
}

