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

/// Internal world snapshot structure (application layer)
struct WorldSnapshot {
    world: crate::domain::entities::World,
    locations: Vec<crate::domain::entities::Location>,
    characters: Vec<crate::domain::entities::Character>,
    scenes: Vec<crate::domain::entities::Scene>,
    current_scene_id: Option<String>,
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
            character_name: None, // TODO: Load from character selection
        })
        .collect()
}

/// Convert PlayerWorldSnapshot to internal representation for legacy callers
/// (kept for reference and potential future use).
#[allow(dead_code)]
pub fn convert_to_internal_snapshot(player_snapshot: &PlayerWorldSnapshot) -> crate::infrastructure::session::WorldSnapshot {
    use crate::domain::entities::{
        Character, Location, LocationType, Scene, StatBlock, TimeContext, World,
    };
    use crate::domain::value_objects::{
        ActId, CampbellArchetype, CharacterId, LocationId, SceneId,
    };

    // Convert world data
    let world_id = uuid::Uuid::parse_str(&player_snapshot.world.id)
        .map(WorldId::from_uuid)
        .unwrap_or_else(|_| WorldId::new());

    use chrono::Utc;
    let now = Utc::now();

    let world = World {
        id: world_id,
        name: player_snapshot.world.name.clone(),
        description: player_snapshot.world.description.clone(),
        rule_system: player_snapshot.world.rule_system.clone().into(),
        created_at: now,
        updated_at: now,
    };

    // Convert locations
    let locations: Vec<Location> = player_snapshot
        .locations
        .iter()
        .map(|l| {
            let location_id = uuid::Uuid::parse_str(&l.id)
                .map(LocationId::from_uuid)
                .unwrap_or_else(|_| LocationId::new());
            let parent_id = l
                .parent_id
                .as_ref()
                .and_then(|pid| uuid::Uuid::parse_str(pid).map(LocationId::from_uuid).ok());

            Location {
                id: location_id,
                world_id,
                parent_id,
                name: l.name.clone(),
                description: l.description.clone(),
                location_type: LocationType::Interior, // Default to Interior
                backdrop_asset: l.backdrop_asset.clone(),
                grid_map_id: None,
                backdrop_regions: Vec::new(),
            }
        })
        .collect();

    // Convert characters
    let characters: Vec<Character> = player_snapshot
        .characters
        .iter()
        .map(|c| {
            let character_id = uuid::Uuid::parse_str(&c.id)
                .map(CharacterId::from_uuid)
                .unwrap_or_else(|_| CharacterId::new());

            Character {
                id: character_id,
                world_id,
                name: c.name.clone(),
                description: c.description.clone(),
                sprite_asset: c.sprite_asset.clone(),
                portrait_asset: c.portrait_asset.clone(),
                base_archetype: CampbellArchetype::Ally, // Default archetype
                current_archetype: CampbellArchetype::Ally,
                archetype_history: Vec::new(),
                wants: Vec::new(),
                stats: StatBlock::default(),
                inventory: Vec::new(),
                is_alive: c.is_alive,
                is_active: c.is_active,
            }
        })
        .collect();

    // Convert scenes
    let scenes: Vec<Scene> = player_snapshot
        .scenes
        .iter()
        .map(|s| {
            let scene_id = uuid::Uuid::parse_str(&s.id)
                .map(SceneId::from_uuid)
                .unwrap_or_else(|_| SceneId::new());
            let location_id = uuid::Uuid::parse_str(&s.location_id)
                .map(LocationId::from_uuid)
                .unwrap_or_else(|_| LocationId::new());
            let featured_characters: Vec<CharacterId> = s
                .featured_characters
                .iter()
                .filter_map(|cid| uuid::Uuid::parse_str(cid).map(CharacterId::from_uuid).ok())
                .collect();

            Scene {
                id: scene_id,
                act_id: ActId::new(), // Placeholder
                name: s.name.clone(),
                location_id,
                time_context: TimeContext::Unspecified,
                backdrop_override: s.backdrop_override.clone(),
                entry_conditions: Vec::new(),
                featured_characters,
                directorial_notes: s.directorial_notes.clone(),
                order: 0,
            }
        })
        .collect();

    crate::infrastructure::session::WorldSnapshot {
        world,
        locations,
        characters,
        scenes,
        current_scene_id: player_snapshot
            .current_scene
            .as_ref()
            .map(|s| s.id.clone()),
    }
}

/// Create a demo world snapshot for testing
fn create_demo_world() -> WorldSnapshot {
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

    WorldSnapshot {
        world,
        locations: vec![],
        characters: vec![],
        scenes: vec![],
        current_scene_id: None,
    }
}


