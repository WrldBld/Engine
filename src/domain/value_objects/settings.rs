//! Application settings value object
//!
//! # Architectural Note (ADR-002: Settings Serialization)
//!
//! AppSettings intentionally includes serde derives because:
//! 1. Settings are stored in SQLite as key-value pairs
//! 2. Settings are transmitted via REST API for UI configuration
//! 3. The JSON schema IS the API contract for settings

use serde::{Deserialize, Serialize};

/// All configurable application settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    // Session
    pub max_conversation_turns: usize,

    // Circuit breaker
    pub circuit_breaker_failure_threshold: u32,
    pub circuit_breaker_open_duration_secs: u64,

    // Cache
    pub health_check_cache_ttl_secs: u64,

    // Validation
    pub max_name_length: usize,
    pub max_description_length: usize,

    // Animation (synced to Player)
    pub typewriter_sentence_delay_ms: u64,
    pub typewriter_pause_delay_ms: u64,
    pub typewriter_char_delay_ms: u64,

    // Game defaults
    pub default_max_stat_value: i32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            max_conversation_turns: 30,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_open_duration_secs: 60,
            health_check_cache_ttl_secs: 30,
            max_name_length: 255,
            max_description_length: 10000,
            typewriter_sentence_delay_ms: 150,
            typewriter_pause_delay_ms: 80,
            typewriter_char_delay_ms: 30,
            default_max_stat_value: 20,
        }
    }
}

impl AppSettings {
    /// Load from environment variables, using defaults for missing values
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            max_conversation_turns: env_or("WRLDBLDR_MAX_CONVERSATION_TURNS", defaults.max_conversation_turns),
            circuit_breaker_failure_threshold: env_or("WRLDBLDR_CIRCUIT_BREAKER_FAILURES", defaults.circuit_breaker_failure_threshold),
            circuit_breaker_open_duration_secs: env_or("WRLDBLDR_CIRCUIT_BREAKER_OPEN_SECS", defaults.circuit_breaker_open_duration_secs),
            health_check_cache_ttl_secs: env_or("WRLDBLDR_HEALTH_CHECK_CACHE_TTL", defaults.health_check_cache_ttl_secs),
            max_name_length: env_or("WRLDBLDR_MAX_NAME_LENGTH", defaults.max_name_length),
            max_description_length: env_or("WRLDBLDR_MAX_DESCRIPTION_LENGTH", defaults.max_description_length),
            typewriter_sentence_delay_ms: env_or("WRLDBLDR_TYPEWRITER_SENTENCE_DELAY", defaults.typewriter_sentence_delay_ms),
            typewriter_pause_delay_ms: env_or("WRLDBLDR_TYPEWRITER_PAUSE_DELAY", defaults.typewriter_pause_delay_ms),
            typewriter_char_delay_ms: env_or("WRLDBLDR_TYPEWRITER_CHAR_DELAY", defaults.typewriter_char_delay_ms),
            default_max_stat_value: env_or("WRLDBLDR_DEFAULT_MAX_STAT", defaults.default_max_stat_value),
        }
    }
}

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
