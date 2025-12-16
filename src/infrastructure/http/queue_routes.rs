//! Queue health check and status routes

use axum::{
    extract::{State, Query},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use crate::application::ports::outbound::{ProcessingQueuePort, QueuePort, QueueItemStatus};
use crate::application::services::{GenerationQueueProjectionService, GenerationQueueSnapshot};
use crate::infrastructure::state::AppState;
use crate::infrastructure::session::SessionManager;
use tokio::sync::RwLockReadGuard;

/// Create queue-related routes
pub fn create_queue_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health/queues", get(queue_health_check))
        .route("/generation/queue", get(get_generation_queue))
        .route("/generation/read-state", axum::routing::post(update_generation_read_state))
}

/// Health check endpoint for queue status
async fn queue_health_check(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    use std::collections::HashMap;

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

    // Compute per-session depths for better observability and future
    // fairness tuning. These are best-effort and should not affect
    // critical-path queue processing.
    let mut player_actions_by_session: HashMap<String, usize> = HashMap::new();
    if let Ok(items) = state
        .player_action_queue_service
        .queue
        .list_by_status(QueueItemStatus::Pending)
        .await
    {
        for item in items {
            let key = item.payload.session_id.to_string();
            *player_actions_by_session.entry(key).or_insert(0) += 1;
        }
    }

    let mut llm_requests_by_session: HashMap<String, usize> = HashMap::new();
    if let Ok(items) = state
        .llm_queue_service
        .queue
        .list_by_status(QueueItemStatus::Pending)
        .await
    {
        for item in items {
            let key = item
                .payload
                .session_id
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "GLOBAL".to_string());
            *llm_requests_by_session.entry(key).or_insert(0) += 1;
        }
    }

    let mut asset_generation_by_session: HashMap<String, usize> = HashMap::new();
    if let Ok(items) = state
        .asset_generation_queue_service
        .queue
        .list_by_status(QueueItemStatus::Pending)
        .await
    {
        for item in items {
            let key = item
                .payload
                .session_id
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "GLOBAL".to_string());
            *asset_generation_by_session.entry(key).or_insert(0) += 1;
        }
    }

    // Approvals are already session-aware at the service layer; reuse
    // that to build a per-session view.
    let mut approvals_by_session: HashMap<String, usize> = HashMap::new();
    let sessions_read: RwLockReadGuard<SessionManager> = state.sessions.read().await;
    let session_ids = sessions_read.get_session_ids();
    drop(sessions_read);

    for session_id in session_ids {
        if let Ok(pending) = state.dm_approval_queue_service.get_pending(session_id).await {
            if !pending.is_empty() {
                approvals_by_session.insert(session_id.to_string(), pending.len());
            }
        }
    }

    Json(json!({
        "status": "healthy",
        "queues": {
            "player_actions": {
                "pending": player_action_depth,
                "by_session": player_actions_by_session,
                "processing": 0,
            },
            "llm_requests": {
                "pending": llm_pending,
                "processing": llm_processing,
                "by_session": llm_requests_by_session,
            },
            "approvals": {
                "pending": approvals_pending,
                "by_session": approvals_by_session,
                "processing": 0,
            },
            "asset_generation": {
                "pending": asset_pending,
                "processing": asset_processing,
                "by_session": asset_generation_by_session,
            },
        },
        "total_pending": player_action_depth + llm_pending + approvals_pending + asset_pending,
        "total_processing": llm_processing + asset_processing,
    }))
}

/// Read-only endpoint exposing current generation queue state
///
/// This is used by the Player Creator UI to reconstruct the unified generation
/// queue (image batches + text suggestions) after a reload.
pub async fn get_generation_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<GenerationQueueSnapshot> {
    // Prefer header-based user ID for future auth/middleware friendliness
    let user_id = headers
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        // Fallback to query param for backward compatibility
        .or_else(|| params.get("user_id").cloned());

    // World context for scoping read-state. Until the Player passes an explicit
    // world_id, we fall back to a global placeholder so existing data continues
    // to function.
    let world_key = params
        .get("world_id")
        .cloned()
        .unwrap_or_else(|| "GLOBAL".to_string());

    // Delegate to the application-layer projection service for reconstruction.
    let snapshot = state
        .generation_queue_projection_service
        .project_queue(user_id.as_deref(), &world_key)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Failed to project generation queue: {}", e);
            GenerationQueueSnapshot {
                batches: Vec::new(),
                suggestions: Vec::new(),
            }
        });

    Json(snapshot)
}

/// Request body for marking generation queue items as read
#[derive(Debug, serde::Deserialize)]
pub struct GenerationReadStateUpdate {
    #[serde(default)]
    pub user_id: String,
    /// Optional world identifier for scoping read-state.
    ///
    /// When omitted, the Engine will store markers under a global placeholder
    /// key so existing clients continue to function.
    #[serde(default)]
    pub world_id: String,
    #[serde(default)]
    pub read_batches: Vec<String>,
    #[serde(default)]
    pub read_suggestions: Vec<String>,
}

/// Persist read/unread state for generation queue items
pub async fn update_generation_read_state(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<GenerationReadStateUpdate>,
) -> Result<StatusCode, (StatusCode, String)> {
    let header_user = headers
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let effective_user_id = header_user.or_else(|| {
        if body.user_id.trim().is_empty() {
            None
        } else {
            Some(body.user_id.clone())
        }
    });

    let Some(user_id) = effective_user_id else {
        return Err((StatusCode::BAD_REQUEST, "user_id is required".to_string()));
    };

    // Derive a world key for scoping the markers. For now this falls back to a
    // global placeholder when the client does not send a world_id yet.
    let world_key = if body.world_id.trim().is_empty() {
        "GLOBAL".to_string()
    } else {
        body.world_id.clone()
    };

    use crate::application::ports::outbound::GenerationReadKind;

    for batch_id in &body.read_batches {
        if let Err(e) = state
            .generation_read_state_repository
            .mark_read(&user_id, &world_key, batch_id, GenerationReadKind::Batch)
            .await
        {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to mark batch read: {}", e),
            ));
        }
    }

    for req_id in &body.read_suggestions {
        if let Err(e) = state
            .generation_read_state_repository
            .mark_read(&user_id, &world_key, req_id, GenerationReadKind::Suggestion)
            .await
        {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to mark suggestion read: {}", e),
            ));
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
