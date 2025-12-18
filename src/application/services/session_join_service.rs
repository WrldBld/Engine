//! Session join service - encapsulates session creation/join and world snapshot export.
//!
//! This service handles joining or creating a session for a given world,
//! exporting the world snapshot for the Player, and gathering participant info.
//!
//! Uses `AsyncSessionPort` for session operations, maintaining hexagonal architecture.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::application::ports::outbound::{AsyncSessionPort, AsyncSessionError, PlayerWorldSnapshot, SessionParticipantInfo, SessionParticipantRole, SessionWorldData};
use crate::application::services::world_service::{WorldService, WorldServiceImpl};
use crate::domain::value_objects::{SessionId, WorldId};

/// Participant information DTO for session join responses
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticipantInfo {
    pub user_id: String,
    pub role: SessionParticipantRole,
    pub character_name: Option<String>,
}

/// Session snapshot message DTO
#[derive(Debug, Clone, serde::Serialize)]
struct SessionSnapshotMessage {
    r#type: &'static str,
    session_id: String,
    world_snapshot: serde_json::Value,
}

/// Information returned when a client successfully joins a session
pub struct SessionJoinedInfo {
    pub session_id: SessionId,
    pub participants: Vec<ParticipantInfo>,
    pub world_snapshot: serde_json::Value,
}

/// Service responsible for handling session join/create flows.
///
/// This is intentionally a small, stateful service that holds references to
/// `AsyncSessionPort` and `WorldServiceImpl` so that the WebSocket handler and
/// HTTP layer can depend on a single injected instance from `AppState`.
pub struct SessionJoinService {
    sessions: Arc<dyn AsyncSessionPort>,
    world_service: WorldServiceImpl,
}

impl SessionJoinService {
    pub fn new(sessions: Arc<dyn AsyncSessionPort>, world_service: WorldServiceImpl) -> Self {
        Self { sessions, world_service }
    }

    /// Join an existing session for the given world (if any) or create a new one.
    ///
    /// This mirrors the previous inline `join_or_create_session` logic that lived in
    /// `infrastructure/websocket.rs`, but is now reusable and testable in isolation.
    pub async fn join_or_create_session_for_world(
        &self,
        client_id: String,
        user_id: String,
        role: SessionParticipantRole,
        world_id: Option<String>,
        sender: mpsc::UnboundedSender<serde_json::Value>,
    ) -> Result<SessionJoinedInfo, AsyncSessionError> {
        // Parse the world ID if provided
        let world_id = if let Some(id_str) = world_id {
            let uuid = uuid::Uuid::parse_str(&id_str)
                .map_err(|_| AsyncSessionError::WorldNotFound(id_str.clone()))?;
            Some(WorldId::from_uuid(uuid))
        } else {
            None
        };

        // Try to find an existing session for this world
        if let Some(wid) = world_id {
            if let Some(session_id) = self.sessions.find_session_for_world(wid).await {
                // Join existing session
                let join_info = self
                    .sessions
                    .join_session(
                        session_id,
                        &client_id,
                        user_id,
                        role,
                    )
                    .await?;

                // Gather participant info
                let participants = gather_participants(&*self.sessions, session_id).await;

                // Forward the initial snapshot to the client via the provided sender
                let snapshot_msg = SessionSnapshotMessage {
                    r#type: "SessionSnapshot",
                    session_id: session_id.to_string(),
                    world_snapshot: join_info.world_snapshot_json.clone(),
                };
                if let Ok(msg_json) = serde_json::to_value(&snapshot_msg) {
                    if let Err(e) = sender.send(msg_json) {
                        tracing::warn!("Failed to send initial session snapshot to client {}: {}", client_id, e);
                    }
                } else {
                    tracing::warn!("Failed to serialize session snapshot for client {}", client_id);
                }

                return Ok(SessionJoinedInfo {
                    session_id,
                    participants,
                    world_snapshot: join_info.world_snapshot_json,
                });
            }

            // Load world data from database using the world service
            let player_snapshot = self.world_service
                .export_world_snapshot(wid)
                .await
                .map_err(|e| AsyncSessionError::Internal(format!("Database error: {}", e)))?;

            // Convert PlayerWorldSnapshot to session world data (opaque JSON)
            let world_data: SessionWorldData = serde_json::to_value(&player_snapshot)
                .map_err(|e| AsyncSessionError::Internal(format!("Serialization error: {}", e)))?;

            // Create session for this world using the async port
            let session_id = self
                .sessions
                .create_session(wid, world_data)
                .await;

            // Join the newly created session
            let join_info = self
                .sessions
                .join_session(
                    session_id,
                    &client_id,
                    user_id,
                    role,
                )
                .await?;

            // Gather participant info (just the joining user at this point)
            let participants = gather_participants(&*self.sessions, session_id).await;

            // Forward the initial snapshot to the client via the provided sender
            let snapshot_msg = SessionSnapshotMessage {
                r#type: "SessionSnapshot",
                session_id: session_id.to_string(),
                world_snapshot: join_info.world_snapshot_json.clone(),
            };
            if let Ok(msg_json) = serde_json::to_value(&snapshot_msg) {
                if let Err(e) = sender.send(msg_json) {
                    tracing::warn!("Failed to send initial session snapshot to client {}: {}", client_id, e);
                }
            } else {
                tracing::warn!("Failed to serialize session snapshot for client {}", client_id);
            }

            Ok(SessionJoinedInfo {
                session_id,
                participants,
                world_snapshot: join_info.world_snapshot_json,
            })
        } else {
            // No world specified - create a demo session via world service
            let demo_world = create_demo_world();
            let world_id = demo_world.world.id;

            // Create world_data as a simple JSON object since domain World doesn't implement Serialize
            let world_data: SessionWorldData = serde_json::json!({
                "id": world_id.to_string(),
                "name": demo_world.world.name.clone(),
                "description": demo_world.world.description.clone()
            });

            let session_id = self.sessions.create_session(world_id, world_data).await;

            let join_info = self
                .sessions
                .join_session(
                    session_id,
                    &client_id,
                    user_id,
                    role,
                )
                .await?;

            // Gather participant info
            let participants = gather_participants(&*self.sessions, session_id).await;

            let snapshot_msg = SessionSnapshotMessage {
                r#type: "SessionSnapshot",
                session_id: session_id.to_string(),
                world_snapshot: join_info.world_snapshot_json.clone(),
            };
            if let Ok(msg_json) = serde_json::to_value(&snapshot_msg) {
                if let Err(e) = sender.send(msg_json) {
                    tracing::warn!("Failed to send initial demo session snapshot to client {}: {}", client_id, e);
                }
            } else {
                tracing::warn!("Failed to serialize demo session snapshot for client {}", client_id);
            }

            Ok(SessionJoinedInfo {
                session_id,
                participants,
                world_snapshot: join_info.world_snapshot_json,
            })
        }
    }
}

/// Gather participant info from a session using the async session port
async fn gather_participants(
    sessions: &dyn AsyncSessionPort,
    session_id: SessionId,
) -> Vec<ParticipantInfo> {
    let infos: Vec<SessionParticipantInfo> = sessions.get_session_participants(session_id).await;
    infos
        .into_iter()
        .map(|p| ParticipantInfo {
            user_id: p.user_id,
            role: p.role,
            character_name: p.character_name,
        })
        .collect()
}

/// Create a demo world snapshot for testing
fn create_demo_world() -> crate::application::dto::WorldSnapshot {
    #[cfg(debug_assertions)]
    tracing::warn!("Creating demo world - this should only happen in development");

    use crate::domain::entities::World;
    use crate::domain::value_objects::RuleSystemConfig;
    use chrono::Utc;

    let world = World {
        id: WorldId::new(),
        name: "Demo World".to_string(),
        description: "A demonstration world for testing".to_string(),
        rule_system: RuleSystemConfig::default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    crate::application::dto::WorldSnapshot {
        world,
        locations: vec![],
        characters: vec![],
        scenes: vec![],
        current_scene_id: None,
    }
}


