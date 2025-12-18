//! WrldBldr Engine - Backend API for TTRPG world management
//!
//! The Engine is the backend server that:
//! - Manages world data in Neo4j
//! - Serves the Player frontend via WebSocket
//! - Integrates with Ollama for LLM-powered NPC responses
//! - Integrates with ComfyUI for asset generation

mod application;
mod domain;
mod infrastructure;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::get, Router};
use crate::application::ports::outbound::{ApprovalQueuePort, QueueNotificationPort, QueuePort};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::infrastructure::config::AppConfig;
use crate::infrastructure::http;
use crate::infrastructure::queue_workers::{approval_notification_worker, dm_action_worker};
use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket_helpers::build_prompt_from_action;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wrldbldr_engine=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting WrldBldr Engine");

    // Load configuration
    let config = AppConfig::from_env()?;
    tracing::info!("Configuration loaded");
    tracing::info!("  Neo4j: {}", config.neo4j_uri);
    tracing::info!("  Ollama: {}", config.ollama_base_url);
    tracing::info!("  ComfyUI: {}", config.comfyui_base_url);

    // Initialize application state
    let (state, generation_event_rx) = AppState::new(config).await?;
    let state = Arc::new(state);
    tracing::info!("Application state initialized");

    // Clone queue config for workers
    let queue_config = state.config.queue.clone();
    let recovery_interval = std::time::Duration::from_secs(queue_config.recovery_poll_interval_seconds);

    // Start background queue workers
    let llm_worker = {
        let service = state.queues.llm_queue_service.clone();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            tracing::info!("Starting LLM queue worker");
            service.run_worker(recovery_interval_clone).await;
        })
    };

    let asset_worker = {
        let service = state.queues.asset_generation_queue_service.clone();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            tracing::info!("Starting asset generation queue worker");
            service.run_worker(recovery_interval_clone).await;
        })
    };

    // Player action queue worker (processes actions and routes to LLM queue)
    let player_action_worker = {
        let service = state.queues.player_action_queue_service.clone();
        let sessions = state.sessions.clone();
        let challenge_service = Arc::new(state.game.challenge_service.clone());
        let skill_service = Arc::new(state.core.skill_service.clone());
        let narrative_event_service = Arc::new(state.game.narrative_event_service.clone());
        let character_repo: Arc<dyn crate::application::ports::outbound::CharacterRepositoryPort> =
            Arc::new(state.repository.characters());
        let settings_service = state.settings_service.clone();
        let notifier = service.queue.notifier();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            tracing::info!("Starting player action queue worker");
            loop {
                let sessions_clone = sessions.clone();
                let challenge_service_clone = challenge_service.clone();
                let skill_service_clone = skill_service.clone();
                let narrative_event_service_clone = narrative_event_service.clone();
                let character_repo_clone = character_repo.clone();
                let settings_service_clone = settings_service.clone();
                match service
                    .process_next(|action| {
                        let sessions = sessions_clone.clone();
                        let challenge_service = challenge_service_clone.clone();
                        let skill_service = skill_service_clone.clone();
                        let narrative_event_service = narrative_event_service_clone.clone();
                        let character_repo = character_repo_clone.clone();
                        let settings_service = settings_service_clone.clone();
                        async move {
                        build_prompt_from_action(
                            &sessions,
                            &challenge_service,
                            &skill_service,
                            &narrative_event_service,
                            &character_repo,
                            &settings_service,
                                &action,
                        )
                        .await
                        }
                    })
                    .await
                {
                    Ok(Some(action_id)) => {
                        tracing::debug!("Processed player action: {}", action_id);
                    }
                    Ok(None) => {
                        // Queue empty - wait for notification or recovery timeout
                        let _ = notifier.wait_for_work(recovery_interval_clone).await;
                    }
                    Err(e) => {
                        tracing::error!("Error processing player action: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        })
    };

    // Approval notification worker (sends ApprovalRequired messages to DM)
    let approval_notification_worker_task = {
        let service = state.queues.dm_approval_queue_service.clone();
        let async_session_port = state.async_session_port.clone();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            approval_notification_worker(service, async_session_port, recovery_interval_clone).await;
        })
    };

    // DM action queue worker (processes approval decisions and other DM actions)
    let dm_action_worker_task = {
        let service = state.queues.dm_action_queue_service.clone();
        let approval_service = state.queues.dm_approval_queue_service.clone();
        let narrative_event_service = Arc::new(state.game.narrative_event_service.clone());
        let scene_service = Arc::new(state.core.scene_service.clone());
        let interaction_service = Arc::new(state.core.interaction_service.clone());
        let async_session_port = state.async_session_port.clone();
        let sessions = state.sessions.clone();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            dm_action_worker(
                service,
                approval_service,
                narrative_event_service,
                scene_service,
                interaction_service,
                async_session_port,
                sessions,
                recovery_interval_clone,
            )
            .await;
        })
    };

    // Cleanup worker (removes old completed/failed queue items)
    let cleanup_worker = {
        let player_action_service = state.queues.player_action_queue_service.clone();
        let llm_service = state.queues.llm_queue_service.clone();
        let approval_service = state.queues.dm_approval_queue_service.clone();
        let asset_service = state.queues.asset_generation_queue_service.clone();
        let queue_config_clone = queue_config.clone();
        tokio::spawn(async move {
            tracing::info!("Starting queue cleanup worker");
            loop {
                let retention = std::time::Duration::from_secs(queue_config_clone.history_retention_hours * 3600);
                
                // Cleanup all queues
                let _ = player_action_service.queue.cleanup(retention).await;
                let _ = llm_service.queue.cleanup(retention).await;
                let _ = approval_service.queue.cleanup(retention).await;
                let _ = asset_service.queue.cleanup(retention).await;
                
                // Expire old approvals
                let approval_timeout = std::time::Duration::from_secs(queue_config_clone.approval_timeout_minutes * 60);
                let _ = approval_service.queue.expire_old(approval_timeout).await;
                
                // Run cleanup using configured interval
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    queue_config_clone.cleanup_interval_seconds
                )).await;
            }
        })
    };

    // Generation event publisher (converts GenerationEvents to AppEvents and publishes to event bus)
    let generation_event_worker = {
        use crate::application::services::GenerationEventPublisher;
        let event_bus = state.events.event_bus.clone();
        let publisher = GenerationEventPublisher::new(event_bus);
        tokio::spawn(async move {
            tracing::info!("Starting generation event publisher");
            publisher.run(generation_event_rx).await;
        })
    };

    // ComfyUI state monitor (broadcasts connection state changes)
    let comfyui_state_monitor = {
        use crate::infrastructure::comfyui::ComfyUIConnectionState;
        use crate::infrastructure::websocket::messages::ServerMessage;
        let comfyui_client = state.comfyui_client.clone();
        let sessions = state.sessions.clone();
        tokio::spawn(async move {
            tracing::info!("Starting ComfyUI state monitor");
            let mut last_state: Option<ComfyUIConnectionState> = None;
            loop {
                let current_state = comfyui_client.connection_state();
                
                // Only broadcast if state changed
                if Some(current_state) != last_state {
                    let (state_str, message, retry_in_seconds) = match current_state {
                        ComfyUIConnectionState::Connected => (
                            "connected".to_string(),
                            Some("ComfyUI is connected".to_string()),
                            None,
                        ),
                        ComfyUIConnectionState::Degraded { consecutive_failures } => (
                            "degraded".to_string(),
                            Some(format!("ComfyUI experiencing issues ({} failures)", consecutive_failures)),
                            Some(30),
                        ),
                        ComfyUIConnectionState::Disconnected => (
                            "disconnected".to_string(),
                            Some("ComfyUI is disconnected".to_string()),
                            Some(30),
                        ),
                        ComfyUIConnectionState::CircuitOpen { until } => {
                            let seconds_until = (until - chrono::Utc::now()).num_seconds().max(0) as u32;
                            (
                                "circuit_open".to_string(),
                                Some("ComfyUI circuit breaker is open - too many failures".to_string()),
                                Some(seconds_until),
                            )
                        }
                    };
                    
                    let msg = ServerMessage::ComfyUIStateChanged {
                        state: state_str,
                        message,
                        retry_in_seconds,
                    };
                    
                    // Broadcast to all sessions
                    let sessions_read = sessions.read().await;
                    for session_id in sessions_read.list_sessions() {
                        sessions_read.broadcast_to_session(session_id, &msg);
                    }
                    
                    last_state = Some(current_state);
                }
                
                // Check every 5 seconds
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        })
    };

    // WebSocket event subscriber (converts AppEvents to ServerMessages and broadcasts to clients)
    let websocket_event_subscriber = {
        use crate::infrastructure::websocket_event_subscriber::WebSocketEventSubscriber;

        // Reuse the event repository from AppState (no duplicate DB connection)
        let app_event_repository = state.events.app_event_repository.clone();
        let notifier = state.events.event_notifier.clone();
        let async_session_port = state.async_session_port.clone();
        let sessions = state.sessions.clone();
        let subscriber = WebSocketEventSubscriber::new(app_event_repository, notifier, async_session_port, sessions, 30);
        tokio::spawn(async move {
            subscriber.run().await;
        })
    };

    tracing::info!("Background queue workers started");

    // Get server port before moving state
    let server_port = state.config.server_port;

    // Build the router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(infrastructure::websocket::ws_handler))
        // Merge REST API routes
        .merge(http::create_routes())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], server_port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    // Run server with graceful shutdown
    let server = axum::serve(listener, app);
    
    // Wait for shutdown signal (Ctrl+C)
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received, stopping workers...");
            // Workers will stop when their tasks complete or are dropped
            llm_worker.abort();
            asset_worker.abort();
            player_action_worker.abort();
            approval_notification_worker_task.abort();
            dm_action_worker_task.abort();
            cleanup_worker.abort();
            generation_event_worker.abort();
            websocket_event_subscriber.abort();
            tracing::info!("Workers stopped");
        }
    }

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
