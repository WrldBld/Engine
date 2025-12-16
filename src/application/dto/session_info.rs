use serde::Serialize;

/// Lightweight metadata about an active play session, for discovery and UX.
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub world_id: String,
    pub dm_user_id: String,
    pub active_player_count: usize,
    pub created_at: i64,
}


