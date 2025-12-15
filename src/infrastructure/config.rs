//! Application configuration

use std::env;

use anyhow::{Context, Result};

/// Application configuration loaded from environment
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Neo4j connection URI
    pub neo4j_uri: String,
    /// Neo4j username
    pub neo4j_user: String,
    /// Neo4j password
    pub neo4j_password: String,
    /// Neo4j database name
    pub neo4j_database: String,

    /// Ollama API base URL (OpenAI-compatible)
    pub ollama_base_url: String,
    /// Default model for LLM requests
    pub ollama_model: String,

    /// ComfyUI server URL
    pub comfyui_base_url: String,

    /// WebSocket server port
    pub server_port: u16,

    /// Queue configuration
    pub queue: QueueConfig,

    /// Session configuration
    pub session: SessionConfig,
}

/// Queue system configuration
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Queue storage backend: "memory" or "sqlite"
    pub backend: String,
    /// SQLite database path (if using sqlite backend)
    pub sqlite_path: String,
    /// Max concurrent LLM requests
    pub llm_batch_size: usize,
    /// Max concurrent ComfyUI requests (always 1 recommended)
    pub asset_batch_size: usize,
    /// How long to keep completed items before cleanup (hours)
    pub history_retention_hours: u64,
    /// How long before pending approvals expire (minutes)
    pub approval_timeout_minutes: u64,
    /// Cleanup worker interval (seconds)
    pub cleanup_interval_seconds: u64,
    /// Recovery poll interval for crash recovery (seconds)
    pub recovery_poll_interval_seconds: u64,
}

/// Session and conversation configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum conversation history turns to retain per session
    pub max_conversation_history: usize,
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            neo4j_uri: env::var("NEO4J_URI")
                .unwrap_or_else(|_| "bolt://localhost:7687".to_string()),
            neo4j_user: env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string()),
            neo4j_password: env::var("NEO4J_PASSWORD")
                .context("NEO4J_PASSWORD environment variable is required")?,
            neo4j_database: env::var("NEO4J_DATABASE").unwrap_or_else(|_| "neo4j".to_string()),

            ollama_base_url: env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://10.8.0.6:11434/v1".to_string()),
            ollama_model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3-vl:30b".to_string()),

            comfyui_base_url: env::var("COMFYUI_BASE_URL")
                .unwrap_or_else(|_| "http://10.8.0.6:8188".to_string()),

            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,

            queue: QueueConfig {
                backend: env::var("QUEUE_BACKEND")
                    .unwrap_or_else(|_| "sqlite".to_string()),
                sqlite_path: env::var("QUEUE_SQLITE_PATH")
                    .unwrap_or_else(|_| "./data/queues.db".to_string()),
                llm_batch_size: env::var("QUEUE_LLM_BATCH_SIZE")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .unwrap_or(1),
                asset_batch_size: env::var("QUEUE_ASSET_BATCH_SIZE")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .unwrap_or(1),
                history_retention_hours: env::var("QUEUE_HISTORY_RETENTION_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .unwrap_or(24),
                approval_timeout_minutes: env::var("QUEUE_APPROVAL_TIMEOUT_MINUTES")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                cleanup_interval_seconds: env::var("QUEUE_CLEANUP_INTERVAL_SECONDS")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .unwrap_or(3600),
                recovery_poll_interval_seconds: env::var("QUEUE_RECOVERY_POLL_INTERVAL_SECONDS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },

            session: SessionConfig {
                max_conversation_history: env::var("SESSION_MAX_CONVERSATION_HISTORY")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
        })
    }
}
