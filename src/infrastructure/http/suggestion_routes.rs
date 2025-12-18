//! Suggestion API routes - LLM-powered content suggestions for world-building

use axum::{
    extract::{State, Path},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::application::services::{
    SuggestionService, SuggestionType, SuggestionContext,
};
use crate::application::dto::{
    SuggestionRequestDto, SuggestionResponseDto, UnifiedSuggestionRequestDto,
    LLMRequestItem, LLMRequestType,
};
use crate::infrastructure::state::AppState;
use uuid::Uuid;

/// Generate character name suggestions
pub async fn suggest_character_names(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_character_names(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "character_name".to_string(),
        suggestions,
    }))
}

/// Generate character description suggestions
pub async fn suggest_character_description(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_character_description(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "character_description".to_string(),
        suggestions,
    }))
}

/// Generate character wants/desires suggestions
pub async fn suggest_character_wants(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_character_wants(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "character_wants".to_string(),
        suggestions,
    }))
}

/// Generate character fears suggestions
pub async fn suggest_character_fears(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_character_fears(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "character_fears".to_string(),
        suggestions,
    }))
}

/// Generate character backstory suggestions
pub async fn suggest_character_backstory(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_character_backstory(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "character_backstory".to_string(),
        suggestions,
    }))
}

/// Generate location name suggestions
pub async fn suggest_location_names(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_location_names(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "location_name".to_string(),
        suggestions,
    }))
}

/// Generate location description suggestions
pub async fn suggest_location_description(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_location_description(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "location_description".to_string(),
        suggestions,
    }))
}

/// Generate location atmosphere suggestions
pub async fn suggest_location_atmosphere(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_location_atmosphere(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "location_atmosphere".to_string(),
        suggestions,
    }))
}

/// Generate location notable features suggestions
pub async fn suggest_location_features(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_location_features(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "location_features".to_string(),
        suggestions,
    }))
}

/// Generate location hidden secrets suggestions
pub async fn suggest_location_secrets(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequestDto>,
) -> Result<Json<SuggestionResponseDto>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = req.into();

    let suggestions = service
        .suggest_location_secrets(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponseDto {
        suggestion_type: "location_secrets".to_string(),
        suggestions,
    }))
}

/// Unified suggestion endpoint - uses suggestion_type in body
/// Now queues the request instead of processing synchronously
pub async fn suggest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UnifiedSuggestionRequestDto>,
) -> Result<Json<SuggestionQueuedResponse>, (StatusCode, String)> {
    let context: SuggestionContext = req.context.into();
    let field_type = match req.suggestion_type {
        SuggestionType::CharacterName => "character_name",
        SuggestionType::CharacterDescription => "character_description",
        SuggestionType::CharacterWants => "character_wants",
        SuggestionType::CharacterFears => "character_fears",
        SuggestionType::CharacterBackstory => "character_backstory",
        SuggestionType::LocationName => "location_name",
        SuggestionType::LocationDescription => "location_description",
        SuggestionType::LocationAtmosphere => "location_atmosphere",
        SuggestionType::LocationFeatures => "location_features",
        SuggestionType::LocationSecrets => "location_secrets",
    };
    
    // Generate request ID
    let request_id = Uuid::new_v4().to_string();
    
    // Create LLM request item
    let llm_request = LLMRequestItem {
        request_type: LLMRequestType::Suggestion {
            field_type: field_type.to_string(),
            entity_id: None, // Could extract from context if needed
        },
        session_id: None, // Creator mode, no session
        pc_id: None, // Creator mode, no player character
        prompt: None, // Suggestions don't use GamePromptRequest
        suggestion_context: Some(context),
        callback_id: request_id.clone(),
    };

    // Enqueue to LLM queue
    state.queues.llm_queue_service.enqueue(llm_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to enqueue suggestion: {}", e)))?;

    Ok(Json(SuggestionQueuedResponse {
        request_id,
        status: "queued".to_string(),
    }))
}

/// Response for queued suggestion request
#[derive(Debug, serde::Serialize)]
pub struct SuggestionQueuedResponse {
    pub request_id: String,
    pub status: String,
}

/// Cancel a pending suggestion request
pub async fn cancel_suggestion(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state.queues.llm_queue_service.cancel_suggestion(&request_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((StatusCode::NOT_FOUND, "Suggestion request not found or already processed".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Retry a failed suggestion request
/// Note: This requires the client to provide the original request parameters
#[derive(Debug, serde::Deserialize)]
pub struct RetrySuggestionRequest {
    pub field_type: String,
    pub context: SuggestionRequestDto,
}

pub async fn retry_suggestion(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
    Json(req): Json<RetrySuggestionRequest>,
) -> Result<Json<SuggestionQueuedResponse>, (StatusCode, String)> {
    // Generate new request ID for the retry
    let new_request_id = Uuid::new_v4().to_string();
    
    let context: SuggestionContext = req.context.into();
    
    // Create LLM request item
    let llm_request = LLMRequestItem {
        request_type: LLMRequestType::Suggestion {
            field_type: req.field_type.clone(),
            entity_id: None,
        },
        session_id: None,
        pc_id: None, // Suggestion retry, no player character context
        prompt: None,
        suggestion_context: Some(context),
        callback_id: new_request_id.clone(),
    };

    // Enqueue to LLM queue
    state.queues.llm_queue_service.enqueue(llm_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to enqueue retry: {}", e)))?;

    Ok(Json(SuggestionQueuedResponse {
        request_id: new_request_id,
        status: "queued".to_string(),
    }))
}
