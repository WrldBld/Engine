//! Application settings value object
//!
//! # Architectural Note (ADR-002: Settings Serialization)
//!
//! AppSettings intentionally includes serde derives because:
//! 1. Settings are stored in SQLite as key-value pairs
//! 2. Settings are transmitted via REST API for UI configuration
//! 3. The JSON schema IS the API contract for settings
//!
//! # Per-World Settings
//!
//! Settings can be global (applied to all worlds) or per-world (override global).
//! The `world_id` field determines scope:
//! - `None` = global settings (default)
//! - `Some(world_id)` = per-world override

use serde::{Deserialize, Serialize};

use super::context_budget::ContextBudgetConfig;
use super::WorldId;

/// All configurable application settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    /// World ID for per-world settings. None = global defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_id: Option<WorldId>,

    // ============================================================================
    // Session & Conversation
    // ============================================================================
    
    /// Maximum conversation turns to store in session memory
    pub max_conversation_turns: usize,
    
    /// Number of conversation turns to include in LLM prompt context
    #[serde(default = "default_conversation_history_turns")]
    pub conversation_history_turns: usize,

    // ============================================================================
    // Circuit Breaker & Health
    // ============================================================================
    
    pub circuit_breaker_failure_threshold: u32,
    pub circuit_breaker_open_duration_secs: u64,
    pub health_check_cache_ttl_secs: u64,

    // ============================================================================
    // Validation Limits
    // ============================================================================
    
    /// Maximum length for entity names (characters, locations, etc.)
    pub max_name_length: usize,
    /// Maximum length for descriptions
    pub max_description_length: usize,

    // ============================================================================
    // Animation (synced to Player)
    // ============================================================================
    
    pub typewriter_sentence_delay_ms: u64,
    pub typewriter_pause_delay_ms: u64,
    pub typewriter_char_delay_ms: u64,

    // ============================================================================
    // Game Defaults
    // ============================================================================
    
    pub default_max_stat_value: i32,

    // ============================================================================
    // Challenge System
    // ============================================================================
    
    /// Number of outcome branches to generate for each challenge result tier
    #[serde(default = "default_outcome_branch_count")]
    pub outcome_branch_count: usize,
    /// Minimum allowed branch count (for UI validation)
    #[serde(default = "default_outcome_branch_min")]
    pub outcome_branch_min: usize,
    /// Maximum allowed branch count (for UI validation)
    #[serde(default = "default_outcome_branch_max")]
    pub outcome_branch_max: usize,

    // ============================================================================
    // LLM Settings
    // ============================================================================
    
    /// Max tokens per outcome branch when generating suggestions
    #[serde(default = "default_suggestion_tokens_per_branch")]
    pub suggestion_tokens_per_branch: u32,

    /// Token budget configuration for LLM context building
    #[serde(default)]
    pub context_budget: ContextBudgetConfig,
}

fn default_outcome_branch_count() -> usize { 2 }
fn default_outcome_branch_min() -> usize { 1 }
fn default_outcome_branch_max() -> usize { 4 }
fn default_conversation_history_turns() -> usize { 20 }
fn default_suggestion_tokens_per_branch() -> u32 { 200 }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            world_id: None,
            max_conversation_turns: 30,
            conversation_history_turns: 20,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_open_duration_secs: 60,
            health_check_cache_ttl_secs: 30,
            max_name_length: 255,
            max_description_length: 10000,
            typewriter_sentence_delay_ms: 150,
            typewriter_pause_delay_ms: 80,
            typewriter_char_delay_ms: 30,
            default_max_stat_value: 20,
            outcome_branch_count: 2,
            outcome_branch_min: 1,
            outcome_branch_max: 4,
            suggestion_tokens_per_branch: 200,
            context_budget: ContextBudgetConfig::default(),
        }
    }
}

impl AppSettings {
    /// Load from environment variables, using defaults for missing values
    pub fn from_env() -> Self {
        let defaults = Self::default();
        let context_budget_defaults = ContextBudgetConfig::default();
        
        Self {
            world_id: None, // Global settings from env
            max_conversation_turns: env_or("WRLDBLDR_MAX_CONVERSATION_TURNS", defaults.max_conversation_turns),
            conversation_history_turns: env_or("WRLDBLDR_CONVERSATION_HISTORY_TURNS", defaults.conversation_history_turns),
            circuit_breaker_failure_threshold: env_or("WRLDBLDR_CIRCUIT_BREAKER_FAILURES", defaults.circuit_breaker_failure_threshold),
            circuit_breaker_open_duration_secs: env_or("WRLDBLDR_CIRCUIT_BREAKER_OPEN_SECS", defaults.circuit_breaker_open_duration_secs),
            health_check_cache_ttl_secs: env_or("WRLDBLDR_HEALTH_CHECK_CACHE_TTL", defaults.health_check_cache_ttl_secs),
            max_name_length: env_or("WRLDBLDR_MAX_NAME_LENGTH", defaults.max_name_length),
            max_description_length: env_or("WRLDBLDR_MAX_DESCRIPTION_LENGTH", defaults.max_description_length),
            typewriter_sentence_delay_ms: env_or("WRLDBLDR_TYPEWRITER_SENTENCE_DELAY", defaults.typewriter_sentence_delay_ms),
            typewriter_pause_delay_ms: env_or("WRLDBLDR_TYPEWRITER_PAUSE_DELAY", defaults.typewriter_pause_delay_ms),
            typewriter_char_delay_ms: env_or("WRLDBLDR_TYPEWRITER_CHAR_DELAY", defaults.typewriter_char_delay_ms),
            default_max_stat_value: env_or("WRLDBLDR_DEFAULT_MAX_STAT", defaults.default_max_stat_value),
            outcome_branch_count: env_or("WRLDBLDR_OUTCOME_BRANCH_COUNT", defaults.outcome_branch_count),
            outcome_branch_min: env_or("WRLDBLDR_OUTCOME_BRANCH_MIN", defaults.outcome_branch_min),
            outcome_branch_max: env_or("WRLDBLDR_OUTCOME_BRANCH_MAX", defaults.outcome_branch_max),
            suggestion_tokens_per_branch: env_or("WRLDBLDR_SUGGESTION_TOKENS_PER_BRANCH", defaults.suggestion_tokens_per_branch),
            // Load context budget from environment variables
            context_budget: ContextBudgetConfig {
                total_budget_tokens: env_or("WRLDBLDR_LLM_TOTAL_BUDGET_TOKENS", context_budget_defaults.total_budget_tokens),
                scene_tokens: env_or("WRLDBLDR_LLM_SCENE_TOKENS", context_budget_defaults.scene_tokens),
                character_tokens: env_or("WRLDBLDR_LLM_CHARACTER_TOKENS", context_budget_defaults.character_tokens),
                conversation_history_tokens: env_or("WRLDBLDR_LLM_CONVERSATION_HISTORY_TOKENS", context_budget_defaults.conversation_history_tokens),
                challenges_tokens: env_or("WRLDBLDR_LLM_CHALLENGES_TOKENS", context_budget_defaults.challenges_tokens),
                narrative_events_tokens: env_or("WRLDBLDR_LLM_NARRATIVE_EVENTS_TOKENS", context_budget_defaults.narrative_events_tokens),
                directorial_notes_tokens: env_or("WRLDBLDR_LLM_DIRECTORIAL_NOTES_TOKENS", context_budget_defaults.directorial_notes_tokens),
                location_context_tokens: env_or("WRLDBLDR_LLM_LOCATION_CONTEXT_TOKENS", context_budget_defaults.location_context_tokens),
                player_context_tokens: env_or("WRLDBLDR_LLM_PLAYER_CONTEXT_TOKENS", context_budget_defaults.player_context_tokens),
                enable_summarization: env_or("WRLDBLDR_LLM_ENABLE_SUMMARIZATION", context_budget_defaults.enable_summarization),
                summarization_model: std::env::var("WRLDBLDR_LLM_SUMMARIZATION_MODEL").ok(),
            },
        }
    }

    /// Create settings for a specific world, starting from global defaults
    pub fn for_world(world_id: WorldId) -> Self {
        let mut settings = Self::from_env();
        settings.world_id = Some(world_id);
        settings
    }

    /// Merge per-world settings with global settings.
    /// Per-world values override global where present.
    pub fn merge_with_global(&self, global: &AppSettings) -> AppSettings {
        // If this is global settings, just return a clone
        if self.world_id.is_none() {
            return self.clone();
        }
        
        // Start with global, then override with per-world values
        // For now, per-world settings completely override global
        // In the future, we could make this more granular with Option<T> fields
        self.clone()
    }
}

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Settings field metadata for UI rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsFieldMetadata {
    /// Field key (matches JSON field name)
    pub key: String,
    /// Display name for UI
    pub display_name: String,
    /// Description/help text
    pub description: String,
    /// Field type: "integer", "float", "boolean", "string"
    pub field_type: String,
    /// Default value
    pub default_value: serde_json::Value,
    /// Minimum value (for numeric fields)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<serde_json::Value>,
    /// Maximum value (for numeric fields)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<serde_json::Value>,
    /// Category for grouping in UI
    pub category: String,
    /// Whether changing this setting requires a restart
    pub requires_restart: bool,
}

/// Get metadata for all settings fields
pub fn settings_metadata() -> Vec<SettingsFieldMetadata> {
    vec![
        // Session & Conversation
        SettingsFieldMetadata {
            key: "max_conversation_turns".into(),
            display_name: "Max Stored Conversation Turns".into(),
            description: "Maximum number of conversation turns to store in session memory".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(5)),
            max_value: Some(serde_json::json!(100)),
            category: "Conversation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "conversation_history_turns".into(),
            display_name: "LLM Context History Turns".into(),
            description: "Number of conversation turns to include in LLM prompt context".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(20),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(50)),
            category: "Conversation".into(),
            requires_restart: false,
        },
        // Validation
        SettingsFieldMetadata {
            key: "max_name_length".into(),
            display_name: "Max Name Length".into(),
            description: "Maximum length for entity names (characters, locations, etc.)".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(255),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(1000)),
            category: "Validation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "max_description_length".into(),
            display_name: "Max Description Length".into(),
            description: "Maximum length for description fields".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(10000),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(50000)),
            category: "Validation".into(),
            requires_restart: false,
        },
        // Challenge System
        SettingsFieldMetadata {
            key: "outcome_branch_count".into(),
            display_name: "Outcome Branches".into(),
            description: "Number of outcome branches to generate for each challenge result".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(2),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(4)),
            category: "Challenges".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "suggestion_tokens_per_branch".into(),
            display_name: "Suggestion Tokens per Branch".into(),
            description: "Maximum tokens per outcome branch when generating LLM suggestions".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(200),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "Challenges".into(),
            requires_restart: false,
        },
        // Animation
        SettingsFieldMetadata {
            key: "typewriter_sentence_delay_ms".into(),
            display_name: "Sentence Delay (ms)".into(),
            description: "Delay after completing a sentence in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(150),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(1000)),
            category: "Animation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "typewriter_pause_delay_ms".into(),
            display_name: "Pause Delay (ms)".into(),
            description: "Delay at punctuation marks in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(80),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(500)),
            category: "Animation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "typewriter_char_delay_ms".into(),
            display_name: "Character Delay (ms)".into(),
            description: "Delay between characters in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(200)),
            category: "Animation".into(),
            requires_restart: false,
        },
        // LLM Context Budgets
        SettingsFieldMetadata {
            key: "context_budget.total_budget_tokens".into(),
            display_name: "Total Token Budget".into(),
            description: "Maximum total tokens for LLM system prompt".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(4000),
            min_value: Some(serde_json::json!(1000)),
            max_value: Some(serde_json::json!(32000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.scene_tokens".into(),
            display_name: "Scene Context Tokens".into(),
            description: "Token budget for scene description, location, atmosphere".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(500),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(2000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.character_tokens".into(),
            display_name: "Character Context Tokens".into(),
            description: "Token budget for NPC personality, motivations, relationships".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(800),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(3000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.conversation_history_tokens".into(),
            display_name: "Conversation History Tokens".into(),
            description: "Token budget for recent conversation history".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(1000),
            min_value: Some(serde_json::json!(200)),
            max_value: Some(serde_json::json!(4000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.challenges_tokens".into(),
            display_name: "Challenges Context Tokens".into(),
            description: "Token budget for active challenges".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(400),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(1500)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.narrative_events_tokens".into(),
            display_name: "Narrative Events Tokens".into(),
            description: "Token budget for active story events".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(400),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(1500)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.directorial_notes_tokens".into(),
            display_name: "Directorial Notes Tokens".into(),
            description: "Token budget for DM guidance and notes".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(300),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.location_context_tokens".into(),
            display_name: "Location Context Tokens".into(),
            description: "Token budget for location-specific details".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(300),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.player_context_tokens".into(),
            display_name: "Player Context Tokens".into(),
            description: "Token budget for player character information".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(300),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.enable_summarization".into(),
            display_name: "Enable Auto-Summarization".into(),
            description: "Automatically summarize context when over budget".into(),
            field_type: "boolean".into(),
            default_value: serde_json::json!(true),
            min_value: None,
            max_value: None,
            category: "LLM Context".into(),
            requires_restart: false,
        },
        // System settings (require restart)
        SettingsFieldMetadata {
            key: "circuit_breaker_failure_threshold".into(),
            display_name: "Circuit Breaker Failures".into(),
            description: "Number of failures before circuit breaker opens".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(5),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(20)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "circuit_breaker_open_duration_secs".into(),
            display_name: "Circuit Breaker Open Duration (s)".into(),
            description: "How long circuit breaker stays open before retry".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(60),
            min_value: Some(serde_json::json!(10)),
            max_value: Some(serde_json::json!(600)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "health_check_cache_ttl_secs".into(),
            display_name: "Health Check Cache TTL (s)".into(),
            description: "How long to cache health check results".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(5)),
            max_value: Some(serde_json::json!(300)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "default_max_stat_value".into(),
            display_name: "Default Max Stat Value".into(),
            description: "Default maximum value for character stats".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(20),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(100)),
            category: "Game".into(),
            requires_restart: false,
        },
    ]
}
