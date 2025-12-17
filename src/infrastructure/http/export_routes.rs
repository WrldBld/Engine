//! Export API routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::dto::ExportQueryDto;
use crate::application::ports::outbound::PlayerWorldSnapshot;
use crate::application::services::WorldService;
use crate::domain::value_objects::WorldId;
use crate::infrastructure::state::AppState;

/// Export a world as JSON snapshot
pub async fn export_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(_query): Query<ExportQueryDto>,
) -> Result<Json<PlayerWorldSnapshot>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let snapshot = state
        .core.world_service
        .export_world_snapshot(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(snapshot))
}

/// Export a world as raw JSON string (for download)
pub async fn export_world_raw(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<ExportQueryDto>,
) -> Result<String, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let snapshot = state
        .core.world_service
        .export_world_snapshot(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let json = match query.format.as_deref() {
        Some("compressed") => serde_json::to_string(&snapshot),
        _ => serde_json::to_string_pretty(&snapshot),
    }
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(json)
}
