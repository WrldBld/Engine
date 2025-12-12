//! Shared application state

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::application::services::{
    CharacterServiceImpl, LocationServiceImpl, SceneServiceImpl, WorldServiceImpl,
};
use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::export::Neo4jWorldExporter;
use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::persistence::Neo4jRepository;
use crate::infrastructure::session::SessionManager;

/// Shared application state
pub struct AppState {
    pub config: AppConfig,
    /// Neo4j repository - still exposed for routes that haven't been migrated to services
    /// TODO: Phase 2 - Remove direct repository access once all routes use services
    pub repository: Neo4jRepository,
    pub llm_client: OllamaClient,
    pub comfyui_client: ComfyUIClient,
    /// Active WebSocket sessions
    pub sessions: RwLock<SessionManager>,
    // Application services
    pub world_service: WorldServiceImpl,
    pub character_service: CharacterServiceImpl,
    pub location_service: LocationServiceImpl,
    pub scene_service: SceneServiceImpl,
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

        // Create individual repository ports as Arc'd trait objects
        let world_repo: Arc<dyn crate::application::ports::outbound::WorldRepositoryPort> =
            Arc::new(repository.worlds());
        let character_repo: Arc<dyn crate::application::ports::outbound::CharacterRepositoryPort> =
            Arc::new(repository.characters());
        let location_repo: Arc<dyn crate::application::ports::outbound::LocationRepositoryPort> =
            Arc::new(repository.locations());
        let scene_repo: Arc<dyn crate::application::ports::outbound::SceneRepositoryPort> =
            Arc::new(repository.scenes());
        let relationship_repo: Arc<dyn crate::application::ports::outbound::RelationshipRepositoryPort> =
            Arc::new(repository.relationships());

        // Create world exporter
        let world_exporter: Arc<dyn crate::application::ports::outbound::WorldExporterPort> =
            Arc::new(Neo4jWorldExporter::new(repository.clone()));

        // Initialize application services
        let world_service = WorldServiceImpl::new(world_repo.clone(), world_exporter);
        let character_service = CharacterServiceImpl::new(
            world_repo.clone(),
            character_repo.clone(),
            relationship_repo,
        );
        let location_service = LocationServiceImpl::new(world_repo, location_repo.clone());
        let scene_service = SceneServiceImpl::new(scene_repo, location_repo, character_repo);

        Ok(Self {
            config,
            repository,
            llm_client,
            comfyui_client,
            sessions: RwLock::new(SessionManager::new()),
            world_service,
            character_service,
            location_service,
            scene_service,
        })
    }
}
