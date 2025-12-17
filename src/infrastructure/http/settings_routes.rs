use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use std::sync::Arc;
use crate::infrastructure::state::AppState;
use crate::domain::value_objects::AppSettings;

pub fn settings_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/settings", get(get_settings))
        .route("/api/settings", put(update_settings))
        .route("/api/settings/reset", post(reset_settings))
}

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
