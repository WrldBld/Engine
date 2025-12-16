//! Configuration API routes
//!
//! Endpoints for managing ComfyUI configuration and status.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::domain::value_objects::ComfyUIConfig;
use crate::infrastructure::comfyui::ComfyUIConnectionState;
use crate::infrastructure::state::AppState;

/// Get current ComfyUI configuration
pub async fn get_comfyui_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ComfyUIConfig>, (StatusCode, String)> {
    Ok(Json(state.comfyui_client.config()))
}

/// Update ComfyUI configuration
pub async fn update_comfyui_config(
    State(state): State<Arc<AppState>>,
    Json(config): Json<ComfyUIConfig>,
) -> Result<Json<ComfyUIConfig>, (StatusCode, String)> {
    // Validate and update the client's config
    state.comfyui_client.update_config(config.clone())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    
    Ok(Json(config))
}

/// Get current ComfyUI connection status
pub async fn get_comfyui_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ComfyUIConnectionState>, (StatusCode, String)> {
    let status = state.comfyui_client.connection_state();
    Ok(Json(status))
}

