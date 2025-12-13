//! Shared application state

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::application::services::{
    AssetServiceImpl, ChallengeServiceImpl, CharacterServiceImpl, EventChainServiceImpl,
    InteractionServiceImpl, LocationServiceImpl, NarrativeEventServiceImpl, SceneServiceImpl,
    SheetTemplateService, SkillServiceImpl, StoryEventService, RelationshipServiceImpl,
    WorkflowConfigService, WorldServiceImpl,
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
    /// Neo4j repository - now private, all access goes through services
    #[allow(dead_code)]
    repository: Neo4jRepository,
    pub llm_client: OllamaClient,
    pub comfyui_client: ComfyUIClient,
    /// Active WebSocket sessions
    pub sessions: RwLock<SessionManager>,
    // Application services
    pub world_service: WorldServiceImpl,
    pub character_service: CharacterServiceImpl,
    pub location_service: LocationServiceImpl,
    pub scene_service: SceneServiceImpl,
    pub skill_service: SkillServiceImpl,
    pub interaction_service: InteractionServiceImpl,
    pub relationship_service: RelationshipServiceImpl,
    pub story_event_service: StoryEventService,
    pub challenge_service: ChallengeServiceImpl,
    pub narrative_event_service: NarrativeEventServiceImpl,
    pub event_chain_service: EventChainServiceImpl,
    pub asset_service: AssetServiceImpl,
    pub workflow_config_service: WorkflowConfigService,
    pub sheet_template_service: SheetTemplateService,
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
        let skill_repo: Arc<dyn crate::application::ports::outbound::SkillRepositoryPort> =
            Arc::new(repository.skills());
        let interaction_repo: Arc<dyn crate::application::ports::outbound::InteractionRepositoryPort> =
            Arc::new(repository.interactions());
        let story_event_repo: Arc<dyn crate::application::ports::outbound::StoryEventRepositoryPort> =
            Arc::new(repository.story_events());
        let challenge_repo: Arc<dyn crate::application::ports::outbound::ChallengeRepositoryPort> =
            Arc::new(repository.challenges());
        let asset_repo: Arc<dyn crate::application::ports::outbound::AssetRepositoryPort> =
            Arc::new(repository.assets());
        let workflow_repo: Arc<dyn crate::application::ports::outbound::WorkflowRepositoryPort> =
            Arc::new(repository.workflows());
        let sheet_template_repo: Arc<dyn crate::application::ports::outbound::SheetTemplateRepositoryPort> =
            Arc::new(repository.sheet_templates());
        let narrative_event_repo: Arc<dyn crate::application::ports::outbound::NarrativeEventRepositoryPort> =
            Arc::new(repository.narrative_events());
        let event_chain_repo: Arc<dyn crate::application::ports::outbound::EventChainRepositoryPort> =
            Arc::new(repository.event_chains());

        // Create world exporter
        let world_exporter: Arc<dyn crate::application::ports::outbound::WorldExporterPort> =
            Arc::new(Neo4jWorldExporter::new(repository.clone()));

        // Initialize application services
        let world_service = WorldServiceImpl::new(world_repo.clone(), world_exporter);
        let character_service = CharacterServiceImpl::new(
            world_repo.clone(),
            character_repo.clone(),
            relationship_repo.clone(),
        );
        let location_service = LocationServiceImpl::new(world_repo.clone(), location_repo.clone());
        let relationship_service = RelationshipServiceImpl::new(relationship_repo);
        let scene_service = SceneServiceImpl::new(scene_repo, location_repo, character_repo);
        let skill_service = SkillServiceImpl::new(skill_repo, world_repo);
        let interaction_service = InteractionServiceImpl::new(interaction_repo);
        let story_event_service = StoryEventService::new(story_event_repo);
        let challenge_service = ChallengeServiceImpl::new(challenge_repo);
        let narrative_event_service = NarrativeEventServiceImpl::new(narrative_event_repo);
        let event_chain_service = EventChainServiceImpl::new(event_chain_repo);
        let asset_service = AssetServiceImpl::new(asset_repo);
        let workflow_config_service = WorkflowConfigService::new(workflow_repo);
        let sheet_template_service = SheetTemplateService::new(sheet_template_repo);

        Ok(Self {
            config,
            repository,
            llm_client,
            comfyui_client,
            sessions: RwLock::new(SessionManager::new()),
            relationship_service,
            world_service,
            character_service,
            location_service,
            scene_service,
            skill_service,
            interaction_service,
            story_event_service,
            challenge_service,
            narrative_event_service,
            event_chain_service,
            asset_service,
            workflow_config_service,
            sheet_template_service,
        })
    }
}
