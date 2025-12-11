//! Shared application state

use anyhow::Result;
use tokio::sync::RwLock;

use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::persistence::Neo4jRepository;
use crate::infrastructure::session::SessionManager;

/// Shared application state
pub struct AppState {
    pub config: AppConfig,
    pub repository: Neo4jRepository,
    pub llm_client: OllamaClient,
    pub comfyui_client: ComfyUIClient,
    /// Active WebSocket sessions
    pub sessions: RwLock<SessionManager>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self> {
        // Initialize Neo4j repository
        let repository = Neo4jRepository::new(
            &config.neo4j_uri,
            &config.neo4j_user,
            &config.neo4j_password,
            &config.neo4j_database,
        )
        .await?;

        // Initialize Ollama client
        let llm_client = OllamaClient::new(&config.ollama_base_url, &config.ollama_model);

        // Initialize ComfyUI client
        let comfyui_client = ComfyUIClient::new(&config.comfyui_base_url);

        Ok(Self {
            config,
            repository,
            llm_client,
            comfyui_client,
            sessions: RwLock::new(SessionManager::new()),
        })
    }
}
