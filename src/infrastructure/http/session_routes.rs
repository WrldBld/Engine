use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{
    application::dto::SessionInfo,
    application::services::session_join_service::convert_to_internal_snapshot,
    application::services::world_service::WorldService,
    domain::value_objects::{SessionId, WorldId},
    infrastructure::state::AppState,
};

/// List all active sessions.
pub async fn list_sessions(State(state): State<Arc<AppState>>) -> Json<Vec<SessionInfo>> {
    let sessions = state.sessions.read().await;

    let infos = sessions
        .get_session_ids()
        .into_iter()
        .filter_map(|session_id| sessions.get_session(session_id).map(|s| (session_id, s)))
        .map(|(session_id, session)| {
            let dm_user_id = session
                .dm_user_id
                .clone()
                .or_else(|| session.get_dm().map(|p| p.user_id.clone()))
                .unwrap_or_default();

            let active_player_count = session
                .participants
                .values()
                .filter(|p| p.role == crate::infrastructure::websocket::messages::ParticipantRole::Player)
                .count();

            SessionInfo {
                session_id: session_id.to_string(),
                world_id: session.world_id.to_string(),
                dm_user_id,
                active_player_count,
                created_at: session.created_at.timestamp(),
            }
        })
        .collect();

    Json(infos)
}

/// List active sessions for a specific world.
pub async fn list_world_sessions(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Json<Vec<SessionInfo>> {
    let sessions = state.sessions.read().await;
    let world_uuid = match uuid::Uuid::parse_str(&world_id) {
        Ok(id) => id,
        Err(_) => return Json(Vec::new()),
    };
    let world_id = WorldId::from_uuid(world_uuid);

    let infos = sessions
        .get_session_ids()
        .into_iter()
        .filter_map(|session_id| sessions.get_session(session_id).map(move |s| (session_id, s)))
        .filter(|(_, session)| session.world_id == world_id)
        .map(|(session_id, session)| {
            let dm_user_id = session
                .dm_user_id
                .clone()
                .or_else(|| session.get_dm().map(|p| p.user_id.clone()))
                .unwrap_or_default();

            let active_player_count = session
                .participants
                .values()
                .filter(|p| p.role == crate::infrastructure::websocket::messages::ParticipantRole::Player)
                .count();

            SessionInfo {
                session_id: session_id.to_string(),
                world_id: session.world_id.to_string(),
                dm_user_id,
                active_player_count,
                created_at: session.created_at.timestamp(),
            }
        })
        .collect();

    Json(infos)
}

#[derive(serde::Deserialize)]
pub struct CreateSessionRequest {
    pub dm_user_id: String,
}

/// Idempotently create or return the DM's session for a world.
pub async fn create_or_get_dm_session(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, StatusCode> {
    let world_uuid = uuid::Uuid::parse_str(&world_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let world_id = WorldId::from_uuid(world_uuid);

    // First, see if a session for this world already exists
    {
        let sessions = state.sessions.read().await;
        if let Some(session_id) = sessions.find_session_for_world(world_id) {
            if let Some(session) = sessions.get_session(session_id) {
                let dm_user_id = session
                    .dm_user_id
                    .clone()
                    .or_else(|| session.get_dm().map(|p| p.user_id.clone()))
                    .unwrap_or(body.dm_user_id.clone());

                let active_player_count = session
                    .participants
                    .values()
                    .filter(|p| p.role
                        == crate::infrastructure::websocket::messages::ParticipantRole::Player)
                    .count();

                let info = SessionInfo {
                    session_id: session_id.to_string(),
                    world_id: session.world_id.to_string(),
                    dm_user_id,
                    active_player_count,
                    created_at: session.created_at.timestamp(),
                };

                return Ok(Json(info));
            }
        }
    }

    // Otherwise create a new session for this world
    let player_snapshot = state
        .core.world_service
        .export_world_snapshot(world_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let internal_snapshot = convert_to_internal_snapshot(&player_snapshot);

    let mut sessions = state.sessions.write().await;

    let session_id = sessions.create_session_with_id(SessionId::new(), world_id, internal_snapshot);

    // Set DM owner metadata for the new session
    if let Some(s) = sessions.get_session_mut(session_id) {
        if s.dm_user_id.is_none() {
            s.dm_user_id = Some(body.dm_user_id.clone());
        }
    }

    let info = {
        let session = sessions
            .get_session(session_id)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        SessionInfo {
            session_id: session_id.to_string(),
            world_id: session.world_id.to_string(),
            dm_user_id: body.dm_user_id,
            active_player_count: 0,
            created_at: session.created_at.timestamp(),
        }
    };

    Ok(Json(info))
}


