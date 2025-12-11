//! HTTP REST API routes

mod asset_routes;
mod character_routes;
mod export_routes;
mod interaction_routes;
mod location_routes;
mod scene_routes;
mod suggestion_routes;
mod world_routes;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

use crate::infrastructure::state::AppState;

pub use asset_routes::*;
pub use character_routes::*;
pub use export_routes::*;
pub use interaction_routes::*;
pub use location_routes::*;
pub use scene_routes::*;
pub use suggestion_routes::*;
pub use world_routes::*;

/// Create all API routes
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        // World routes
        .route("/api/worlds", get(world_routes::list_worlds))
        .route("/api/worlds", post(world_routes::create_world))
        .route("/api/worlds/{id}", get(world_routes::get_world))
        .route("/api/worlds/{id}", put(world_routes::update_world))
        .route("/api/worlds/{id}", delete(world_routes::delete_world))
        .route("/api/worlds/{id}/acts", get(world_routes::list_acts))
        .route("/api/worlds/{id}/acts", post(world_routes::create_act))
        // Character routes
        .route(
            "/api/worlds/{world_id}/characters",
            get(character_routes::list_characters),
        )
        .route(
            "/api/worlds/{world_id}/characters",
            post(character_routes::create_character),
        )
        .route("/api/characters/{id}", get(character_routes::get_character))
        .route(
            "/api/characters/{id}",
            put(character_routes::update_character),
        )
        .route(
            "/api/characters/{id}",
            delete(character_routes::delete_character),
        )
        .route(
            "/api/characters/{id}/archetype",
            put(character_routes::change_archetype),
        )
        // Location routes
        .route(
            "/api/worlds/{world_id}/locations",
            get(location_routes::list_locations),
        )
        .route(
            "/api/worlds/{world_id}/locations",
            post(location_routes::create_location),
        )
        .route("/api/locations/{id}", get(location_routes::get_location))
        .route("/api/locations/{id}", put(location_routes::update_location))
        .route(
            "/api/locations/{id}",
            delete(location_routes::delete_location),
        )
        .route(
            "/api/locations/{id}/connections",
            get(location_routes::get_connections),
        )
        .route(
            "/api/locations/connections",
            post(location_routes::create_connection),
        )
        // Scene routes
        .route(
            "/api/acts/{act_id}/scenes",
            get(scene_routes::list_scenes_by_act),
        )
        .route(
            "/api/acts/{act_id}/scenes",
            post(scene_routes::create_scene),
        )
        .route("/api/scenes/{id}", get(scene_routes::get_scene))
        .route("/api/scenes/{id}", put(scene_routes::update_scene))
        .route("/api/scenes/{id}", delete(scene_routes::delete_scene))
        .route(
            "/api/scenes/{id}/notes",
            put(scene_routes::update_directorial_notes),
        )
        // Social network
        .route(
            "/api/worlds/{world_id}/social-network",
            get(character_routes::get_social_network),
        )
        .route(
            "/api/relationships",
            post(character_routes::create_relationship),
        )
        .route(
            "/api/relationships/{id}",
            delete(character_routes::delete_relationship),
        )
        // Export
        .route("/api/worlds/{id}/export", get(export_routes::export_world))
        .route(
            "/api/worlds/{id}/export/raw",
            get(export_routes::export_world_raw),
        )
        // Interaction routes
        .route(
            "/api/scenes/{scene_id}/interactions",
            get(interaction_routes::list_interactions),
        )
        .route(
            "/api/scenes/{scene_id}/interactions",
            post(interaction_routes::create_interaction),
        )
        .route(
            "/api/interactions/{id}",
            get(interaction_routes::get_interaction),
        )
        .route(
            "/api/interactions/{id}",
            put(interaction_routes::update_interaction),
        )
        .route(
            "/api/interactions/{id}",
            delete(interaction_routes::delete_interaction),
        )
        .route(
            "/api/interactions/{id}/availability",
            put(interaction_routes::set_interaction_availability),
        )
        // Asset Gallery routes - Characters
        .route(
            "/api/characters/{character_id}/gallery",
            get(asset_routes::list_character_assets),
        )
        .route(
            "/api/characters/{character_id}/gallery",
            post(asset_routes::upload_character_asset),
        )
        .route(
            "/api/characters/{character_id}/gallery/{asset_id}/activate",
            put(asset_routes::activate_character_asset),
        )
        .route(
            "/api/characters/{character_id}/gallery/{asset_id}/label",
            put(asset_routes::update_character_asset_label),
        )
        .route(
            "/api/characters/{character_id}/gallery/{asset_id}",
            delete(asset_routes::delete_character_asset),
        )
        // Asset Gallery routes - Locations
        .route(
            "/api/locations/{location_id}/gallery",
            get(asset_routes::list_location_assets),
        )
        .route(
            "/api/locations/{location_id}/gallery",
            post(asset_routes::upload_location_asset),
        )
        .route(
            "/api/locations/{location_id}/gallery/{asset_id}/activate",
            put(asset_routes::activate_location_asset),
        )
        .route(
            "/api/locations/{location_id}/gallery/{asset_id}",
            delete(asset_routes::delete_location_asset),
        )
        // Asset Gallery routes - Items
        .route(
            "/api/items/{item_id}/gallery",
            get(asset_routes::list_item_assets),
        )
        .route(
            "/api/items/{item_id}/gallery",
            post(asset_routes::upload_item_asset),
        )
        .route(
            "/api/items/{item_id}/gallery/{asset_id}/activate",
            put(asset_routes::activate_item_asset),
        )
        .route(
            "/api/items/{item_id}/gallery/{asset_id}",
            delete(asset_routes::delete_item_asset),
        )
        // Generation Queue routes
        .route("/api/assets/generate", post(asset_routes::queue_generation))
        .route("/api/assets/queue", get(asset_routes::list_queue))
        .route("/api/assets/ready", get(asset_routes::list_ready_batches))
        .route("/api/assets/batch/{batch_id}", get(asset_routes::get_batch))
        .route(
            "/api/assets/batch/{batch_id}/assets",
            get(asset_routes::get_batch_assets),
        )
        .route(
            "/api/assets/batch/{batch_id}/select",
            post(asset_routes::select_from_batch),
        )
        .route(
            "/api/assets/batch/{batch_id}",
            delete(asset_routes::cancel_batch),
        )
        // Suggestion routes
        .route("/api/suggest", post(suggestion_routes::suggest))
        .route(
            "/api/suggest/character/name",
            post(suggestion_routes::suggest_character_names),
        )
        .route(
            "/api/suggest/character/description",
            post(suggestion_routes::suggest_character_description),
        )
        .route(
            "/api/suggest/character/wants",
            post(suggestion_routes::suggest_character_wants),
        )
        .route(
            "/api/suggest/character/fears",
            post(suggestion_routes::suggest_character_fears),
        )
        .route(
            "/api/suggest/character/backstory",
            post(suggestion_routes::suggest_character_backstory),
        )
        .route(
            "/api/suggest/location/name",
            post(suggestion_routes::suggest_location_names),
        )
        .route(
            "/api/suggest/location/description",
            post(suggestion_routes::suggest_location_description),
        )
        .route(
            "/api/suggest/location/atmosphere",
            post(suggestion_routes::suggest_location_atmosphere),
        )
        .route(
            "/api/suggest/location/features",
            post(suggestion_routes::suggest_location_features),
        )
        .route(
            "/api/suggest/location/secrets",
            post(suggestion_routes::suggest_location_secrets),
        )
}
