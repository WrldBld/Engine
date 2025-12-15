//! Shared application state

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::application::services::{
    AssetGenerationQueueService, AssetServiceImpl, ChallengeServiceImpl, CharacterServiceImpl,
    DMActionQueueService, DMApprovalQueueService, EventChainServiceImpl,
    InteractionServiceImpl, LLMQueueService, LocationServiceImpl, NarrativeEventServiceImpl,
    PlayerActionQueueService, SceneServiceImpl, SheetTemplateService, SkillServiceImpl,
    StoryEventService, RelationshipServiceImpl, WorkflowConfigService, WorldServiceImpl,
};
use crate::application::services::generation_service::{GenerationService, GenerationEvent};
use crate::application::dto::{
    AppEvent, ApprovalItem, AssetGenerationItem, DMActionItem, LLMRequestItem, PlayerActionItem,
};
use crate::application::ports::outbound::{AppEventRepositoryPort, EventBusPort};
use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::event_bus::{InProcessEventNotifier, SqliteEventBus};
use crate::infrastructure::export::Neo4jWorldExporter;
use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::persistence::Neo4jRepository;
use crate::infrastructure::queues::QueueFactory;
use crate::infrastructure::repositories::SqliteAppEventRepository;
use crate::infrastructure::session::SessionManager;

/// Shared application state
pub struct AppState {
    pub config: AppConfig,
    /// Neo4j repository - kept for potential direct access, normally use services
    ///
    /// This field is private and marked `#[allow(dead_code)]` because all data access
    /// should go through the individual repository ports (world_repo, character_repo, etc.)
    /// which provide proper trait-based abstraction. This field is retained in case
    /// direct repository access is needed for advanced operations.
    #[allow(dead_code)]
    repository: Neo4jRepository,
    pub llm_client: OllamaClient,
    pub comfyui_client: ComfyUIClient,
    /// Active WebSocket sessions
    pub sessions: Arc<RwLock<SessionManager>>,
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
    pub generation_service: Arc<GenerationService>,
    // Queue services - using QueueBackendEnum for runtime backend selection
    // Note: Services take Arc<Q> where Q implements the port, so Q = QueueBackendEnum<T>
    // (not Arc<QueueBackendEnum<T>>) since Arc<QueueBackendEnum<T>> implements the port
    pub player_action_queue_service: Arc<PlayerActionQueueService<
        crate::infrastructure::queues::QueueBackendEnum<PlayerActionItem>,
        crate::infrastructure::queues::QueueBackendEnum<LLMRequestItem>,
    >>,
    pub dm_action_queue_service: Arc<DMActionQueueService<crate::infrastructure::queues::QueueBackendEnum<DMActionItem>>>,
    pub llm_queue_service: Arc<LLMQueueService<crate::infrastructure::queues::QueueBackendEnum<LLMRequestItem>, OllamaClient, crate::infrastructure::queues::InProcessNotifier>>,
    pub asset_generation_queue_service: Arc<
        AssetGenerationQueueService<
            crate::infrastructure::queues::QueueBackendEnum<AssetGenerationItem>,
            ComfyUIClient,
            crate::infrastructure::queues::InProcessNotifier,
        >,
    >,
    pub dm_approval_queue_service: Arc<DMApprovalQueueService<crate::infrastructure::queues::QueueBackendEnum<ApprovalItem>>>,
    // Event bus
    pub event_bus: Arc<dyn EventBusPort<AppEvent>>,
    pub event_notifier: InProcessEventNotifier,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<(Self, tokio::sync::mpsc::UnboundedReceiver<GenerationEvent>)> {
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
        let skill_service = SkillServiceImpl::new(skill_repo.clone(), world_repo);
        let interaction_service = InteractionServiceImpl::new(interaction_repo);
        // Temporarily create a simple story event service without event_bus, will update after event_bus is created
        let story_event_repo_for_service = story_event_repo.clone();
        let challenge_service = ChallengeServiceImpl::new(challenge_repo.clone());
        // Narrative event service will be created after event_bus
        let narrative_event_repo_for_service = narrative_event_repo.clone();
        let event_chain_service = EventChainServiceImpl::new(event_chain_repo);
        let asset_repo_for_service = asset_repo.clone();
        let asset_service = AssetServiceImpl::new(asset_repo_for_service);
        let workflow_config_service = WorkflowConfigService::new(workflow_repo);
        let sheet_template_service = SheetTemplateService::new(sheet_template_repo);

        // Initialize queue infrastructure using factory
        let queue_factory = QueueFactory::new(config.queue.clone()).await?;
        tracing::info!("Queue backend: {}", queue_factory.config().backend);

        let player_action_queue = queue_factory.create_player_action_queue().await?;
        let llm_queue = queue_factory.create_llm_queue().await?;
        let dm_action_queue = queue_factory.create_dm_action_queue().await?;
        let asset_generation_queue = queue_factory.create_asset_generation_queue().await?;
        let approval_queue = queue_factory.create_approval_queue().await?;

        // Initialize event bus infrastructure
        // For now, use a separate SQLite database for events
        // In production, this could share the queue pool or use Redis
        let event_db_path = config.queue.sqlite_path.replace(".db", "_events.db");
        if let Some(parent) = std::path::Path::new(&event_db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create event database directory: {}", e))?;
        }
        let event_pool = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", event_db_path))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to event database: {}", e))?;
        tracing::info!("Connected to event database: {}", event_db_path);

        let app_event_repository = SqliteAppEventRepository::new(event_pool).await
            .map_err(|e| anyhow::anyhow!("Failed to initialize event repository: {}", e))?;
        let app_event_repository: Arc<dyn AppEventRepositoryPort> = Arc::new(app_event_repository);

        let event_notifier = InProcessEventNotifier::new();
        let event_bus: Arc<dyn EventBusPort<AppEvent>> = Arc::new(SqliteEventBus::new(
            app_event_repository,
            event_notifier.clone(),
        ));

        // Create story event service with event bus
        let story_event_service = StoryEventService::new(story_event_repo_for_service, event_bus.clone());
        // Create narrative event service with event bus
        let narrative_event_service = NarrativeEventServiceImpl::new(narrative_event_repo_for_service, event_bus.clone());

        // Initialize queue services
        // Services take Arc<Q>, so we pass Arc<QueueBackendEnum<T>> directly
        let player_action_queue_service = Arc::new(PlayerActionQueueService::new(
            player_action_queue.clone(),
            llm_queue.clone(),
        ));

        let dm_action_queue_service = Arc::new(DMActionQueueService::new(dm_action_queue.clone()));

        // Create event channel for generation service (needed for LLMQueueService suggestions)
        let (generation_event_tx, generation_event_rx) = tokio::sync::mpsc::unbounded_channel();
        let generation_event_tx_for_llm = generation_event_tx.clone();

        let llm_client_arc = Arc::new(llm_client.clone());
        let llm_queue_service = Arc::new(LLMQueueService::new(
            llm_queue.clone(),
            llm_client_arc,
            approval_queue.clone(),
            challenge_repo.clone(),
            skill_repo.clone(),
            narrative_event_repo.clone(),
            queue_factory.config().llm_batch_size,
            queue_factory.llm_notifier(),
            generation_event_tx_for_llm,
        ));

        let asset_repo_for_queue = asset_repo.clone();
        let asset_generation_queue_service = Arc::new(AssetGenerationQueueService::new(
            asset_generation_queue.clone(),
            Arc::new(comfyui_client.clone()),
            asset_repo_for_queue,
            queue_factory.config().asset_batch_size,
            queue_factory.asset_generation_notifier(),
        ));

        let dm_approval_queue_service = Arc::new(DMApprovalQueueService::new(approval_queue.clone()));

        // Create generation service (generation_event_tx already created above)
        let generation_service = Arc::new(GenerationService::new(
            Arc::new(comfyui_client.clone()) as Arc<dyn crate::application::ports::outbound::ComfyUIPort>,
            asset_repo.clone(),
            std::path::PathBuf::from("./data/assets"),
            std::path::PathBuf::from("./workflows"),
            generation_event_tx,
        ));

        Ok((Self {
            config: config.clone(),
            repository,
            llm_client,
            comfyui_client,
            sessions: Arc::new(RwLock::new(SessionManager::new(
                config.session.max_conversation_history
            ))),
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
            generation_service,
            player_action_queue_service,
            dm_action_queue_service,
            llm_queue_service,
            asset_generation_queue_service,
            dm_approval_queue_service,
            event_bus,
            event_notifier,
        }, generation_event_rx))
    }
}
