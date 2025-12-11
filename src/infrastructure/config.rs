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
        })
    }
}
