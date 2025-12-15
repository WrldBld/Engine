//! Queue health check and status routes

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use crate::application::ports::outbound::{ProcessingQueuePort, QueuePort};
use crate::application::dto::{AppEvent, GenerationBatchResponseDto};
use crate::application::services::asset_service::AssetService;
use crate::infrastructure::state::AppState;

/// Create queue-related routes
pub fn create_queue_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health/queues", get(queue_health_check))
        .route("/generation/queue", get(get_generation_queue))
        .route("/generation/read-state", axum::routing::post(update_generation_read_state))
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

/// Unified generation queue snapshot (batches + suggestions)
#[derive(Debug, Serialize)]
pub struct SuggestionTaskSnapshot {
    pub request_id: String,
    pub field_type: String,
    pub entity_id: Option<String>,
    pub status: String,
    pub suggestions: Option<Vec<String>>,
    pub error: Option<String>,
    pub is_read: bool,
}

#[derive(Debug, Serialize)]
pub struct GenerationQueueSnapshot {
    pub batches: Vec<GenerationBatchResponseDtoWithRead>,
    pub suggestions: Vec<SuggestionTaskSnapshot>,
}

#[derive(Debug, Serialize)]
pub struct GenerationBatchResponseDtoWithRead {
    #[serde(flatten)]
    pub batch: GenerationBatchResponseDto,
    pub is_read: bool,
}

/// Read-only endpoint exposing current generation queue state
///
/// This is used by the Player Creator UI to reconstruct the unified generation
/// queue (image batches + text suggestions) after a reload.
pub async fn get_generation_queue(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<GenerationQueueSnapshot> {
    let user_id = params.get("user_id").cloned();

    // Compute read markers for this user, if provided
    let mut read_batches = std::collections::HashSet::new();
    let mut read_suggestions = std::collections::HashSet::new();

    if let Some(ref uid) = user_id {
        if let Ok(markers) = state
            .generation_read_state_repository
            .list_read_for_user(uid)
            .await
        {
            use crate::application::ports::outbound::GenerationReadKind;
            for (item_id, kind) in markers {
                match kind {
                    GenerationReadKind::Batch => {
                        read_batches.insert(item_id);
                    }
                    GenerationReadKind::Suggestion => {
                        read_suggestions.insert(item_id);
                    }
                }
            }
        }
    }

    // Image batches: reuse existing asset service listing
    let batches = state
        .asset_service
        .list_active_batches()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|b| {
            let dto = GenerationBatchResponseDto::from(b);
            let is_read = read_batches.contains(&dto.id);
            GenerationBatchResponseDtoWithRead { batch: dto, is_read }
        })
        .collect();

    // Suggestion tasks: reconstruct from recent AppEvents
    // For now, fetch a fixed window of recent events.
    let mut suggestions_map: std::collections::HashMap<String, SuggestionTaskSnapshot> =
        std::collections::HashMap::new();

    if let Ok(events) = state.app_event_repository.fetch_since(0, 500).await {
        for (_id, event, _ts) in events {
            match event {
                AppEvent::SuggestionQueued {
                    request_id,
                    field_type,
                    entity_id,
                } => {
                    let entry = suggestions_map
                        .entry(request_id.clone())
                        .or_insert(SuggestionTaskSnapshot {
                            request_id,
                            field_type,
                            entity_id,
                            status: "queued".to_string(),
                            suggestions: None,
                            error: None,
                            is_read: false,
                        });
                    entry.status = "queued".to_string();
                }
                AppEvent::SuggestionProgress { request_id, .. } => {
                    let entry = suggestions_map
                        .entry(request_id.clone())
                        .or_insert(SuggestionTaskSnapshot {
                            request_id,
                            field_type: String::new(),
                            entity_id: None,
                            status: "processing".to_string(),
                            suggestions: None,
                            error: None,
                            is_read: false,
                        });
                    entry.status = "processing".to_string();
                }
                AppEvent::SuggestionCompleted {
                    request_id,
                    field_type,
                    suggestions,
                } => {
                    let entry = suggestions_map
                        .entry(request_id.clone())
                        .or_insert(SuggestionTaskSnapshot {
                            request_id,
                            field_type: field_type.clone(),
                            entity_id: None,
                            status: "ready".to_string(),
                            suggestions: Some(suggestions.clone()),
                            error: None,
                            is_read: false,
                        });
                    entry.field_type = field_type;
                    entry.status = "ready".to_string();
                    entry.suggestions = Some(suggestions);
                    entry.error = None;
                }
                AppEvent::SuggestionFailed {
                    request_id,
                    field_type,
                    error,
                } => {
                    let entry = suggestions_map
                        .entry(request_id.clone())
                        .or_insert(SuggestionTaskSnapshot {
                            request_id,
                            field_type: field_type.clone(),
                            entity_id: None,
                            status: "failed".to_string(),
                            suggestions: None,
                            error: Some(error.clone()),
                            is_read: false,
                        });
                    entry.field_type = field_type;
                    entry.status = "failed".to_string();
                    entry.error = Some(error);
                }
                _ => {}
            }
        }
    }

    // Apply read-state to suggestions
    let mut suggestions: Vec<SuggestionTaskSnapshot> = suggestions_map.into_values().collect();
    for s in &mut suggestions {
        if read_suggestions.contains(&s.request_id) {
            s.is_read = true;
        }
    }

    Json(GenerationQueueSnapshot { batches, suggestions })
}

/// Request body for marking generation queue items as read
#[derive(Debug, serde::Deserialize)]
pub struct GenerationReadStateUpdate {
    pub user_id: String,
    #[serde(default)]
    pub read_batches: Vec<String>,
    #[serde(default)]
    pub read_suggestions: Vec<String>,
}

/// Persist read/unread state for generation queue items
pub async fn update_generation_read_state(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GenerationReadStateUpdate>,
) -> Result<StatusCode, (StatusCode, String)> {
    if body.user_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "user_id is required".to_string()));
    }

    use crate::application::ports::outbound::GenerationReadKind;

    for batch_id in &body.read_batches {
        if let Err(e) = state
            .generation_read_state_repository
            .mark_read(&body.user_id, batch_id, GenerationReadKind::Batch)
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
            .mark_read(&body.user_id, req_id, GenerationReadKind::Suggestion)
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
