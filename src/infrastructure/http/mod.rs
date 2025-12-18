//! HTTP REST API routes

mod asset_routes;
mod challenge_routes;
mod character_routes;
mod config_routes;
mod event_chain_routes;
mod export_routes;
mod interaction_routes;
mod location_routes;
mod narrative_event_routes;
mod observation_routes;
mod player_character_routes;
mod region_routes;
mod session_routes;
mod queue_routes;
mod rule_system_routes;
mod scene_routes;
mod settings_routes;
mod sheet_template_routes;
mod skill_routes;
mod story_event_routes;
mod suggestion_routes;
mod workflow_routes;
mod world_routes;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;

use crate::infrastructure::state::AppState;


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
        // Character inventory route (Phase 23B)
        .route(
            "/api/characters/{id}/inventory",
            get(character_routes::get_inventory),
        )
        // Character-Region relationship routes (Phase 23C)
        .route(
            "/api/characters/{id}/region-relationships",
            get(character_routes::list_region_relationships),
        )
        .route(
            "/api/characters/{id}/region-relationships",
            post(character_routes::add_region_relationship),
        )
        .route(
            "/api/characters/{character_id}/region-relationships/{region_id}/{rel_type}",
            delete(character_routes::remove_region_relationship),
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
        .route(
            "/api/worlds/{world_id}/locations/available-for-starting",
            get(location_routes::list_available_starting_locations),
        )
        // Region routes
        .route(
            "/api/locations/{location_id}/regions",
            get(region_routes::list_regions),
        )
        .route(
            "/api/locations/{location_id}/regions",
            post(region_routes::create_region),
        )
        .route(
            "/api/regions/{region_id}",
            get(region_routes::get_region),
        )
        .route(
            "/api/regions/{region_id}",
            patch(region_routes::update_region),
        )
        .route(
            "/api/regions/{region_id}",
            delete(region_routes::delete_region),
        )
        .route(
            "/api/worlds/{world_id}/spawn-points",
            get(region_routes::list_spawn_points),
        )
        .route(
            "/api/regions/{region_id}/connections",
            get(region_routes::list_region_connections),
        )
        .route(
            "/api/regions/{region_id}/connections",
            post(region_routes::create_region_connection),
        )
        .route(
            "/api/regions/{from_region_id}/connections/{to_region_id}",
            delete(region_routes::delete_region_connection),
        )
        .route(
            "/api/regions/{from_region_id}/connections/{to_region_id}/unlock",
            post(region_routes::unlock_region_connection),
        )
        .route(
            "/api/regions/{region_id}/exits",
            get(region_routes::list_region_exits),
        )
        .route(
            "/api/regions/{region_id}/exits",
            post(region_routes::create_region_exit),
        )
        .route(
            "/api/regions/{region_id}/exits/{location_id}",
            delete(region_routes::delete_region_exit),
        )
        // NPC-Region relationship routes (Phase 23C)
        .route(
            "/api/regions/{region_id}/npcs",
            get(region_routes::list_region_npcs),
        )
        // Derived scene route (Phase 23C)
        .route(
            "/api/regions/{region_id}/scene",
            post(region_routes::get_derived_scene),
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
        // Session routes
        .route("/api/sessions", get(session_routes::list_sessions))
        .route(
            "/api/worlds/{world_id}/sessions",
            get(session_routes::list_world_sessions),
        )
        .route(
            "/api/worlds/{world_id}/sessions",
            post(session_routes::create_or_get_dm_session),
        )
        // Game Time routes (Phase 23F)
        .route(
            "/api/sessions/{session_id}/game-time",
            get(session_routes::get_game_time),
        )
        .route(
            "/api/sessions/{session_id}/game-time/advance",
            post(session_routes::advance_game_time),
        )
        // Player Character routes
        .route(
            "/api/sessions/{session_id}/player-characters",
            post(player_character_routes::create_player_character),
        )
        .route(
            "/api/sessions/{session_id}/player-characters",
            get(player_character_routes::list_player_characters),
        )
        .route(
            "/api/sessions/{session_id}/player-characters/me",
            get(player_character_routes::get_my_player_character),
        )
        // PC Selection routes (Phase 23B.6)
        .route(
            "/api/sessions/{session_id}/available-pcs",
            get(player_character_routes::list_available_pcs),
        )
        .route(
            "/api/sessions/{session_id}/select-pc",
            post(player_character_routes::select_pc),
        )
        .route(
            "/api/users/{user_id}/pcs",
            get(player_character_routes::list_user_pcs),
        )
        .route(
            "/api/worlds/{world_id}/import-pc",
            post(player_character_routes::import_pc),
        )
        .route(
            "/api/player-characters/{pc_id}",
            get(player_character_routes::get_player_character),
        )
        .route(
            "/api/player-characters/{pc_id}",
            put(player_character_routes::update_player_character),
        )
        .route(
            "/api/player-characters/{pc_id}",
            delete(player_character_routes::delete_player_character),
        )
        .route(
            "/api/player-characters/{pc_id}/location",
            put(player_character_routes::update_player_character_location),
        )
        // Observation routes (Phase 23D)
        .route(
            "/api/player-characters/{pc_id}/observations",
            get(observation_routes::list_observations),
        )
        .route(
            "/api/player-characters/{pc_id}/observations",
            post(observation_routes::create_observation),
        )
        .route(
            "/api/player-characters/{pc_id}/observations/{npc_id}",
            get(observation_routes::get_observation),
        )
        .route(
            "/api/player-characters/{pc_id}/observations/{npc_id}",
            delete(observation_routes::delete_observation),
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
        // Unified generation queue snapshot
        .route(
            "/api/generation/queue",
            get(queue_routes::get_generation_queue),
        )
        // Generation Queue routes
        .route("/api/assets/generate", post(asset_routes::queue_generation))
        .route("/api/worlds/{world_id}/assets/queue", get(asset_routes::list_queue))
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
        .route(
            "/api/assets/batch/{batch_id}/retry",
            post(asset_routes::retry_batch),
        )
        // Suggestion routes (async queue-based)
        .route("/api/suggest", post(suggestion_routes::suggest))
        .route(
            "/api/suggest/{request_id}/cancel",
            delete(suggestion_routes::cancel_suggestion),
        )
        // Workflow Configuration routes
        .route("/api/workflows", get(workflow_routes::list_workflow_slots))
        .route(
            "/api/workflows/{slot}",
            get(workflow_routes::get_workflow_config),
        )
        .route(
            "/api/workflows/{slot}",
            post(workflow_routes::save_workflow_config),
        )
        .route(
            "/api/workflows/{slot}",
            delete(workflow_routes::delete_workflow_config),
        )
        .route(
            "/api/workflows/{slot}/defaults",
            patch(workflow_routes::update_workflow_defaults),
        )
        .route(
            "/api/workflows/analyze",
            post(workflow_routes::analyze_workflow),
        )
        .route(
            "/api/workflows/export",
            get(workflow_routes::export_workflows),
        )
        .route(
            "/api/workflows/import",
            post(workflow_routes::import_workflows),
        )
        .route(
            "/api/workflows/{slot}/test",
            post(workflow_routes::test_workflow),
        )
        // Rule System routes
        .route(
            "/api/rule-systems",
            get(rule_system_routes::list_rule_systems),
        )
        .route(
            "/api/rule-systems/{system_type}",
            get(rule_system_routes::get_rule_system),
        )
        .route(
            "/api/rule-systems/{system_type}/presets",
            get(rule_system_routes::list_presets),
        )
        .route(
            "/api/rule-systems/{system_type}/presets/{variant}",
            get(rule_system_routes::get_preset),
        )
        // Skill routes
        .route(
            "/api/worlds/{world_id}/skills",
            get(skill_routes::list_skills),
        )
        .route(
            "/api/worlds/{world_id}/skills",
            post(skill_routes::create_skill),
        )
        .route(
            "/api/worlds/{world_id}/skills/{skill_id}",
            put(skill_routes::update_skill),
        )
        .route(
            "/api/worlds/{world_id}/skills/{skill_id}",
            delete(skill_routes::delete_skill),
        )
        .route(
            "/api/worlds/{world_id}/skills/initialize",
            post(skill_routes::initialize_skills),
        )
        // Sheet Template routes
        .route(
            "/api/worlds/{world_id}/sheet-template",
            get(sheet_template_routes::get_template),
        )
        .route(
            "/api/worlds/{world_id}/sheet-templates",
            get(sheet_template_routes::list_templates),
        )
        .route(
            "/api/worlds/{world_id}/sheet-templates/{template_id}",
            get(sheet_template_routes::get_template_by_id),
        )
        .route(
            "/api/worlds/{world_id}/sheet-templates/{template_id}",
            delete(sheet_template_routes::delete_template),
        )
        .route(
            "/api/worlds/{world_id}/sheet-template/initialize",
            post(sheet_template_routes::initialize_template),
        )
        .route(
            "/api/worlds/{world_id}/sheet-templates/{template_id}/sections",
            post(sheet_template_routes::add_section),
        )
        .route(
            "/api/worlds/{world_id}/sheet-templates/{template_id}/sections/{section_id}/fields",
            post(sheet_template_routes::add_field),
        )
        // Challenge routes
        .route(
            "/api/worlds/{world_id}/challenges",
            get(challenge_routes::list_challenges),
        )
        .route(
            "/api/worlds/{world_id}/challenges",
            post(challenge_routes::create_challenge),
        )
        .route(
            "/api/worlds/{world_id}/challenges/active",
            get(challenge_routes::list_active_challenges),
        )
        .route(
            "/api/worlds/{world_id}/challenges/favorites",
            get(challenge_routes::list_favorite_challenges),
        )
        .route(
            "/api/scenes/{scene_id}/challenges",
            get(challenge_routes::list_scene_challenges),
        )
        .route(
            "/api/challenges/{challenge_id}",
            get(challenge_routes::get_challenge),
        )
        .route(
            "/api/challenges/{challenge_id}",
            put(challenge_routes::update_challenge),
        )
        .route(
            "/api/challenges/{challenge_id}",
            delete(challenge_routes::delete_challenge),
        )
        .route(
            "/api/challenges/{challenge_id}/favorite",
            put(challenge_routes::toggle_favorite),
        )
        .route(
            "/api/challenges/{challenge_id}/active",
            put(challenge_routes::set_active),
        )
        // Story Event routes (Timeline)
        .route(
            "/api/worlds/{world_id}/story-events",
            get(story_event_routes::list_story_events),
        )
        .route(
            "/api/worlds/{world_id}/story-events/count",
            get(story_event_routes::count_story_events),
        )
        .route(
            "/api/worlds/{world_id}/story-events/dm-marker",
            post(story_event_routes::create_dm_marker),
        )
        .route(
            "/api/story-events/{event_id}",
            get(story_event_routes::get_story_event),
        )
        .route(
            "/api/story-events/{event_id}",
            put(story_event_routes::update_story_event),
        )
        .route(
            "/api/story-events/{event_id}",
            delete(story_event_routes::delete_story_event),
        )
        .route(
            "/api/story-events/{event_id}/visibility",
            put(story_event_routes::toggle_visibility),
        )
        // Narrative Event routes (DM-designed events)
        .route(
            "/api/worlds/{world_id}/narrative-events",
            get(narrative_event_routes::list_narrative_events),
        )
        .route(
            "/api/worlds/{world_id}/narrative-events",
            post(narrative_event_routes::create_narrative_event),
        )
        .route(
            "/api/worlds/{world_id}/narrative-events/active",
            get(narrative_event_routes::list_active_events),
        )
        .route(
            "/api/worlds/{world_id}/narrative-events/favorites",
            get(narrative_event_routes::list_favorite_events),
        )
        .route(
            "/api/worlds/{world_id}/narrative-events/pending",
            get(narrative_event_routes::list_pending_events),
        )
        .route(
            "/api/narrative-events/{event_id}",
            get(narrative_event_routes::get_narrative_event),
        )
        .route(
            "/api/narrative-events/{event_id}",
            put(narrative_event_routes::update_narrative_event),
        )
        .route(
            "/api/narrative-events/{event_id}",
            delete(narrative_event_routes::delete_narrative_event),
        )
        .route(
            "/api/narrative-events/{event_id}/favorite",
            put(narrative_event_routes::toggle_favorite),
        )
        .route(
            "/api/narrative-events/{event_id}/active",
            put(narrative_event_routes::set_active),
        )
        .route(
            "/api/narrative-events/{event_id}/trigger",
            post(narrative_event_routes::mark_triggered),
        )
        .route(
            "/api/narrative-events/{event_id}/reset",
            post(narrative_event_routes::reset_triggered),
        )
        // Event Chain routes (Story arcs)
        .route(
            "/api/worlds/{world_id}/event-chains",
            get(event_chain_routes::list_event_chains),
        )
        .route(
            "/api/worlds/{world_id}/event-chains",
            post(event_chain_routes::create_event_chain),
        )
        .route(
            "/api/worlds/{world_id}/event-chains/active",
            get(event_chain_routes::list_active_chains),
        )
        .route(
            "/api/worlds/{world_id}/event-chains/favorites",
            get(event_chain_routes::list_favorite_chains),
        )
        .route(
            "/api/worlds/{world_id}/event-chains/statuses",
            get(event_chain_routes::list_chain_statuses),
        )
        .route(
            "/api/event-chains/{chain_id}",
            get(event_chain_routes::get_event_chain),
        )
        .route(
            "/api/event-chains/{chain_id}",
            put(event_chain_routes::update_event_chain),
        )
        .route(
            "/api/event-chains/{chain_id}",
            delete(event_chain_routes::delete_event_chain),
        )
        .route(
            "/api/event-chains/{chain_id}/favorite",
            put(event_chain_routes::toggle_favorite),
        )
        .route(
            "/api/event-chains/{chain_id}/active",
            put(event_chain_routes::set_active),
        )
        .route(
            "/api/event-chains/{chain_id}/reset",
            post(event_chain_routes::reset_chain),
        )
        .route(
            "/api/event-chains/{chain_id}/events",
            post(event_chain_routes::add_event_to_chain),
        )
        .route(
            "/api/event-chains/{chain_id}/events/{event_id}",
            delete(event_chain_routes::remove_event_from_chain),
        )
        .route(
            "/api/event-chains/{chain_id}/events/{event_id}/complete",
            post(event_chain_routes::complete_event_in_chain),
        )
        // Queue health check
        .merge(queue_routes::create_queue_routes())
        // Settings routes
        .merge(settings_routes::settings_routes())
}
