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
    let state = AppState::new(config).await?;
    let state = Arc::new(state);
    tracing::info!("Application state initialized");

    // Start background queue workers
    let llm_worker = {
        let service = state.llm_queue_service.clone();
        tokio::spawn(async move {
            tracing::info!("Starting LLM queue worker");
            service.run_worker().await;
        })
    };

    let asset_worker = {
        let service = state.asset_generation_queue_service.clone();
        tokio::spawn(async move {
            tracing::info!("Starting asset generation queue worker");
            service.run_worker().await;
        })
    };

    // Player action queue worker (processes actions and routes to LLM queue)
    let player_action_worker = {
        let service = state.player_action_queue_service.clone();
        let sessions = state.sessions.clone();
        let challenge_service = state.challenge_service.clone();
        let narrative_event_service = state.narrative_event_service.clone();
        tokio::spawn(async move {
            tracing::info!("Starting player action queue worker");
            loop {
                match service
                    .process_next(|action| async move {
                        build_prompt_from_action(
                            &sessions,
                            &challenge_service,
                            &narrative_event_service,
                            action,
                        )
                        .await
                    })
                    .await
                {
                    Ok(Some(action_id)) => {
                        tracing::debug!("Processed player action: {}", action_id);
                    }
                    Ok(None) => {
                        // Queue empty, wait a bit before checking again
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
        let service = state.dm_approval_queue_service.clone();
        let sessions = state.sessions.clone();
        tokio::spawn(async move {
            approval_notification_worker(service, sessions).await;
        })
    };

    // DM action queue worker (processes approval decisions and other DM actions)
    let dm_action_worker_task = {
        let service = state.dm_action_queue_service.clone();
        let approval_service = state.dm_approval_queue_service.clone();
        let sessions = state.sessions.clone();
        tokio::spawn(async move {
            dm_action_worker(service, approval_service, sessions).await;
        })
    };

    // Cleanup worker (removes old completed/failed queue items)
    let cleanup_worker = {
        let player_action_service = state.player_action_queue_service.clone();
        let llm_service = state.llm_queue_service.clone();
        let approval_service = state.dm_approval_queue_service.clone();
        let asset_service = state.asset_generation_queue_service.clone();
        let config = state.config.queue.clone();
        tokio::spawn(async move {
            tracing::info!("Starting queue cleanup worker");
            loop {
                let retention = std::time::Duration::from_secs(config.history_retention_hours * 3600);
                
                // Cleanup all queues
                let _ = player_action_service.queue.cleanup(retention).await;
                let _ = llm_service.queue.cleanup(retention).await;
                let _ = approval_service.queue.cleanup(retention).await;
                let _ = asset_service.queue.cleanup(retention).await;
                
                // Expire old approvals
                let approval_timeout = std::time::Duration::from_secs(config.approval_timeout_minutes * 60);
                let _ = approval_service.queue.expire_old(approval_timeout).await;
                
                // Run cleanup every hour
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        })
    };

    tracing::info!("Background queue workers started");

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
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
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
            tracing::info!("Workers stopped");
        }
    }

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
