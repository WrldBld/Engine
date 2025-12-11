//! Suggestion API routes - LLM-powered content suggestions for world-building

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::services::{
    SuggestionContext, SuggestionService, SuggestionType,
};
use crate::infrastructure::state::AppState;

/// Request body for suggestion endpoints
#[derive(Debug, Deserialize)]
pub struct SuggestionRequest {
    /// Type of entity (e.g., "character", "location", "tavern", "forest")
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Name of the entity (if already set)
    #[serde(default)]
    pub entity_name: Option<String>,
    /// World/setting name or type
    #[serde(default)]
    pub world_setting: Option<String>,
    /// Hints or keywords to guide generation
    #[serde(default)]
    pub hints: Option<String>,
    /// Additional context from other fields
    #[serde(default)]
    pub additional_context: Option<String>,
}

impl From<SuggestionRequest> for SuggestionContext {
    fn from(req: SuggestionRequest) -> Self {
        SuggestionContext {
            entity_type: req.entity_type,
            entity_name: req.entity_name,
            world_setting: req.world_setting,
            hints: req.hints,
            additional_context: req.additional_context,
        }
    }
}

/// Response containing generated suggestions
#[derive(Debug, Serialize)]
pub struct SuggestionResponse {
    /// The type of suggestion that was generated
    pub suggestion_type: String,
    /// The generated suggestions
    pub suggestions: Vec<String>,
}

/// Generate character name suggestions
pub async fn suggest_character_names(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_character_names(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "character_name".to_string(),
        suggestions,
    }))
}

/// Generate character description suggestions
pub async fn suggest_character_description(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_character_description(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "character_description".to_string(),
        suggestions,
    }))
}

/// Generate character wants/desires suggestions
pub async fn suggest_character_wants(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_character_wants(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "character_wants".to_string(),
        suggestions,
    }))
}

/// Generate character fears suggestions
pub async fn suggest_character_fears(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_character_fears(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "character_fears".to_string(),
        suggestions,
    }))
}

/// Generate character backstory suggestions
pub async fn suggest_character_backstory(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_character_backstory(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "character_backstory".to_string(),
        suggestions,
    }))
}

/// Generate location name suggestions
pub async fn suggest_location_names(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_location_names(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "location_name".to_string(),
        suggestions,
    }))
}

/// Generate location description suggestions
pub async fn suggest_location_description(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_location_description(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "location_description".to_string(),
        suggestions,
    }))
}

/// Generate location atmosphere suggestions
pub async fn suggest_location_atmosphere(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_location_atmosphere(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "location_atmosphere".to_string(),
        suggestions,
    }))
}

/// Generate location notable features suggestions
pub async fn suggest_location_features(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_location_features(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "location_features".to_string(),
        suggestions,
    }))
}

/// Generate location hidden secrets suggestions
pub async fn suggest_location_secrets(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req);

    let suggestions = service
        .suggest_location_secrets(&context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: "location_secrets".to_string(),
        suggestions,
    }))
}

/// Unified suggestion endpoint - uses suggestion_type in body
#[derive(Debug, Deserialize)]
pub struct UnifiedSuggestionRequest {
    /// Type of suggestion to generate
    pub suggestion_type: SuggestionType,
    /// Context for the suggestion
    #[serde(flatten)]
    pub context: SuggestionRequest,
}

pub async fn suggest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UnifiedSuggestionRequest>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    let service = SuggestionService::new(state.llm_client.clone());
    let context = SuggestionContext::from(req.context);

    let (suggestion_type_str, suggestions) = match req.suggestion_type {
        SuggestionType::CharacterName => (
            "character_name",
            service.suggest_character_names(&context).await,
        ),
        SuggestionType::CharacterDescription => (
            "character_description",
            service.suggest_character_description(&context).await,
        ),
        SuggestionType::CharacterWants => (
            "character_wants",
            service.suggest_character_wants(&context).await,
        ),
        SuggestionType::CharacterFears => (
            "character_fears",
            service.suggest_character_fears(&context).await,
        ),
        SuggestionType::CharacterBackstory => (
            "character_backstory",
            service.suggest_character_backstory(&context).await,
        ),
        SuggestionType::LocationName => (
            "location_name",
            service.suggest_location_names(&context).await,
        ),
        SuggestionType::LocationDescription => (
            "location_description",
            service.suggest_location_description(&context).await,
        ),
        SuggestionType::LocationAtmosphere => (
            "location_atmosphere",
            service.suggest_location_atmosphere(&context).await,
        ),
        SuggestionType::LocationFeatures => (
            "location_features",
            service.suggest_location_features(&context).await,
        ),
        SuggestionType::LocationSecrets => (
            "location_secrets",
            service.suggest_location_secrets(&context).await,
        ),
    };

    let suggestions = suggestions.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestionResponse {
        suggestion_type: suggestion_type_str.to_string(),
        suggestions,
    }))
}
