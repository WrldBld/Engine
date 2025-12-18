use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put, delete},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;
use crate::infrastructure::state::AppState;
use crate::domain::value_objects::{AppSettings, WorldId, settings_metadata, SettingsFieldMetadata};

pub fn settings_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Global settings
        .route("/api/settings", get(get_settings))
        .route("/api/settings", put(update_settings))
        .route("/api/settings/reset", post(reset_settings))
        // Settings metadata for UI
        .route("/api/settings/metadata", get(get_settings_metadata))
        // Per-world settings
        .route("/api/worlds/{world_id}/settings", get(get_world_settings))
        .route("/api/worlds/{world_id}/settings", put(update_world_settings))
        .route("/api/worlds/{world_id}/settings/reset", post(reset_world_settings))
        .route("/api/worlds/{world_id}/settings", delete(delete_world_settings))
}

// =============================================================================
// Global Settings
// =============================================================================

async fn get_settings(State(state): State<Arc<AppState>>) -> Json<AppSettings> {
    Json(state.settings_service.get().await)
}

async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(settings): Json<AppSettings>,
) -> Result<Json<AppSettings>, (StatusCode, String)> {
    state
        .settings_service
        .update(settings.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(settings))
}

async fn reset_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AppSettings>, (StatusCode, String)> {
    state
        .settings_service
        .reset()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// =============================================================================
// Settings Metadata
// =============================================================================

async fn get_settings_metadata() -> Json<Vec<SettingsFieldMetadata>> {
    Json(settings_metadata())
}

// =============================================================================
// Per-World Settings
// =============================================================================

async fn get_world_settings(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<AppSettings>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID format".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);
    
    Ok(Json(state.settings_service.get_for_world(world_id).await))
}

async fn update_world_settings(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(settings): Json<AppSettings>,
) -> Result<Json<AppSettings>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID format".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);
    
    state
        .settings_service
        .update_for_world(world_id, settings.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Return the updated settings with world_id set
    Ok(Json(state.settings_service.get_for_world(world_id).await))
}

async fn reset_world_settings(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<AppSettings>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID format".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);
    
    state
        .settings_service
        .reset_for_world(world_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn delete_world_settings(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID format".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);
    
    state
        .settings_service
        .delete_for_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(StatusCode::NO_CONTENT)
}
