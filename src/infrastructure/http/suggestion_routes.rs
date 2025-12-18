//! Suggestion API routes - LLM-powered content suggestions for world-building
//!
//! All suggestions are processed asynchronously via the LLM queue.
//! Results are delivered via WebSocket events.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::dto::{LLMRequestItem, LLMRequestType, UnifiedSuggestionRequestDto};
use crate::application::services::SuggestionContext;
use crate::infrastructure::state::AppState;

/// Response for queued suggestion request
#[derive(Debug, serde::Serialize)]
pub struct SuggestionQueuedResponse {
    pub request_id: String,
    pub status: String,
}

/// Queue a suggestion request
///
/// The suggestion will be processed asynchronously by the LLM queue.
/// Results are delivered via WebSocket `SuggestionCompleted` or `SuggestionFailed` events.
///
/// # Request Body
/// - `suggestion_type`: One of `character_name`, `character_description`, `character_wants`,
///   `character_fears`, `character_backstory`, `location_name`, `location_description`,
///   `location_atmosphere`, `location_features`, `location_secrets`
/// - `world_id`: The world ID for routing the response
/// - Context fields (optional): `entity_type`, `entity_name`, `world_setting`, `hints`, `additional_context`
pub async fn suggest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UnifiedSuggestionRequestDto>,
) -> Result<Json<SuggestionQueuedResponse>, (StatusCode, String)> {
    let context: SuggestionContext = req.context.into();
    let field_type = req.suggestion_type.to_field_type();

    // Generate request ID
    let request_id = Uuid::new_v4().to_string();

    // Parse world_id for routing
    let world_id = uuid::Uuid::parse_str(&req.world_id)
        .map(crate::domain::value_objects::WorldId::from_uuid)
        .ok();

    // Create LLM request item
    let llm_request = LLMRequestItem {
        request_type: LLMRequestType::Suggestion {
            field_type: field_type.to_string(),
            entity_id: None,
        },
        session_id: None,
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: Some(context),
        callback_id: request_id.clone(),
    };

    // Enqueue to LLM queue
    state
        .queues
        .llm_queue_service
        .enqueue(llm_request)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to enqueue suggestion: {}", e),
            )
        })?;

    Ok(Json(SuggestionQueuedResponse {
        request_id,
        status: "queued".to_string(),
    }))
}

/// Cancel a pending suggestion request
///
/// If the suggestion is still queued or processing, it will be cancelled.
/// A `SuggestionFailed` event with "Cancelled by user" will be sent via WebSocket.
pub async fn cancel_suggestion(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state
        .queues
        .llm_queue_service
        .cancel_suggestion(&request_id)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            "Suggestion request not found or already processed".to_string(),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
