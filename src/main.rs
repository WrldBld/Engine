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
use crate::infrastructure::state::AppState;

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
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
