//! Queue health check and status routes

use axum::{extract::State, routing::get, Json, Router};
use serde_json::json;
use std::sync::Arc;

use crate::application::ports::outbound::{ProcessingQueuePort, QueuePort};
use crate::infrastructure::state::AppState;

/// Create queue-related routes
pub fn create_queue_routes() -> Router<Arc<AppState>> {
    Router::new().route("/health/queues", get(queue_health_check))
}

/// Health check endpoint for queue status
async fn queue_health_check(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let player_action_depth = state
        .player_action_queue_service
        .depth()
        .await
        .unwrap_or(0);

    let llm_pending = state
        .llm_queue_service
        .queue
        .depth()
        .await
        .unwrap_or(0);

    let llm_processing = state
        .llm_queue_service
        .queue
        .processing_count()
        .await
        .unwrap_or(0);

    let approvals_pending = state
        .dm_approval_queue_service
        .queue
        .depth()
        .await
        .unwrap_or(0);

    let asset_pending = state
        .asset_generation_queue_service
        .depth()
        .await
        .unwrap_or(0);

    let asset_processing = state
        .asset_generation_queue_service
        .processing_count()
        .await
        .unwrap_or(0);

    Json(json!({
        "status": "healthy",
        "queues": {
            "player_actions": {
                "pending": player_action_depth,
                "processing": 0,
            },
            "llm_requests": {
                "pending": llm_pending,
                "processing": llm_processing,
            },
            "approvals": {
                "pending": approvals_pending,
                "processing": 0,
            },
            "asset_generation": {
                "pending": asset_pending,
                "processing": asset_processing,
            },
        },
        "total_pending": player_action_depth + llm_pending + approvals_pending + asset_pending,
        "total_processing": llm_processing + asset_processing,
    }))
}
