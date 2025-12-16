//! Infrastructure layer - External adapters and implementations
//!
//! This layer contains:
//! - Persistence: Neo4j adapter for data storage
//! - HTTP: REST API routes
//! - WebSocket: Real-time communication with Player clients
//! - Ollama: LLM integration for AI-powered responses
//! - ComfyUI: Asset generation integration
//! - Config: Application configuration
//! - State: Shared application state
//! - Session: Game session management
//! - Event Bus: Event publishing and subscription infrastructure
//! - Repositories: Additional persistence implementations

pub mod comfyui;
pub mod config;
pub mod event_bus;
pub mod export;
pub mod http;
pub mod ollama;
pub mod persistence;
pub mod queue_workers;
pub mod queues;
pub mod repositories;
pub mod session;
pub mod session_adapter;
pub mod state;
pub mod websocket;
pub mod websocket_event_subscriber;
pub mod websocket_helpers;
