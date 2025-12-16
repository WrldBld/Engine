//! Session join service - encapsulates session creation/join and world snapshot export.
//!
//! This service is responsible for joining or creating a session for a given world,
//! exporting the world snapshot for the Player, and gathering participant info.
//!
//! # Architecture Note: Hexagonal Violation
//!
//! This service currently imports `SessionManager` directly from the infrastructure layer:
//! ```ignore
//! use crate::infrastructure::session::{ClientId, SessionError, SessionManager, WorldSnapshot};
//! ```
//!
//! This violates hexagonal architecture rules where application services should depend only
//! on domain types and port traits, not infrastructure implementations.
//!
//! ## Planned Refactoring
//!
//! To fix this violation, the service should be refactored to:
//! 1. Accept `AsyncSessionPort` trait bound instead of concrete `SessionManager`
//! 2. Replace all `SessionManager` method calls with equivalent `AsyncSessionPort` methods
//! 3. Move `ClientId`, `SessionError`, `WorldSnapshot` to domain or port definitions
//!
//! The port trait already exists at: `application/ports/outbound/async_session_port.rs`
//! The adapter already exists at: `infrastructure/session_adapter.rs`
//!
//! Example of the fix:
//! ```ignore
//! use crate::application::ports::outbound::AsyncSessionPort;
//!
//! pub struct SessionJoinService<S: AsyncSessionPort> {
//!     sessions: S,
//!     world_service: WorldServiceImpl,
//! }
//! ```
//!
//! This refactoring should be done as a complete pass (with tests) rather than partially
//! applied, as the service has multiple complex flows that depend on SessionManager's API.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::application::ports::outbound::{AsyncSessionPort, PlayerWorldSnapshot, SessionParticipantInfo, SessionParticipantRole, SessionWorldData};
use crate::application::services::world_service::{WorldService, WorldServiceImpl};
use crate::domain::value_objects::{SessionId, WorldId};
use crate::infrastructure::session::{ClientId, SessionError};
use crate::infrastructure::websocket::messages::{ParticipantInfo, ParticipantRole, ServerMessage};

/// Information returned when a client successfully joins a session
pub struct SessionJoinedInfo {
    pub session_id: SessionId,
    pub participants: Vec<ParticipantInfo>,
    pub world_snapshot: serde_json::Value,
}

/// Service responsible for handling session join/create flows.
///
/// This is intentionally a small, stateful service that holds references to
/// `SessionManager` and `WorldServiceImpl` so that the WebSocket handler and
/// HTTP layer can depend on a single injected instance from `AppState`.
///
/// # TODO: Architecture Violation
///
/// This service depends on `SessionManager` (a concrete infrastructure type) rather than
/// `AsyncSessionPort` (the port trait). This violates hexagonal architecture rules.
/// See module documentation for planned refactoring approach.
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
        client_id: ClientId,
        user_id: String,
        role: ParticipantRole,
        world_id: Option<String>,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) -> Result<SessionJoinedInfo, SessionError> {
        // Parse the world ID if provided
        let world_id = if let Some(id_str) = world_id {
            let uuid = uuid::Uuid::parse_str(&id_str)
                .map_err(|_| SessionError::WorldNotFound(id_str.clone()))?;
            Some(WorldId::from_uuid(uuid))
        } else {
            None
        };

        // Try to find an existing session for this world
        if let Some(wid) = world_id {
            if let Some(session_id) = self.sessions.find_session_for_world(wid).await {
                // Join existing session
                let client_id_str = client_id.to_string();
                let join_info = self
                    .sessions
                    .join_session(
                        session_id,
                        &client_id_str,
                        user_id,
                        map_role_to_session_role(role),
                    )
                    .await
                    .map_err(|e| SessionError::Internal(format!("Failed to join session: {}", e)))?;

                // Gather participant info
                let participants = gather_participants(&*self.sessions, session_id).await;

                // Forward the initial snapshot to the client via the provided sender
                let snapshot_msg = ServerMessage::SessionSnapshot {
                    session_id: session_id.to_string(),
                    world_snapshot: join_info.world_snapshot_json.clone(),
                };
                if let Err(e) = sender.send(snapshot_msg) {
                    tracing::warn!("Failed to send initial session snapshot to client {}: {}", client_id, e);
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
                .map_err(|e| SessionError::Database(e.into()))?;

            // Convert PlayerWorldSnapshot to session world data (opaque JSON)
            let world_data: SessionWorldData = serde_json::to_value(&player_snapshot)
                .map_err(|e| SessionError::Database(e.to_string().into()))?;

            // Create session for this world using the async port
            let session_id = self
                .sessions
                .create_session(wid, world_data)
                .await;

            // Join the newly created session
            let client_id_str = client_id.to_string();
            let join_info = self
                .sessions
                .join_session(
                    session_id,
                    &client_id_str,
                    user_id,
                    map_role_to_session_role(role),
                )
                .await
                .map_err(|e| SessionError::Internal(format!("Failed to join new session: {}", e)))?;

            // Gather participant info (just the joining user at this point)
            let participants = gather_participants(&*self.sessions, session_id).await;

            // Forward the initial snapshot to the client via the provided sender
            let snapshot_msg = ServerMessage::SessionSnapshot {
                session_id: session_id.to_string(),
                world_snapshot: join_info.world_snapshot_json.clone(),
            };
            if let Err(e) = sender.send(snapshot_msg) {
                tracing::warn!("Failed to send initial session snapshot to client {}: {}", client_id, e);
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

            let world_data: SessionWorldData = serde_json::to_value(&demo_world.world)
                .unwrap_or(serde_json::json!({ "name": "Demo World" }));

            let session_id = self.sessions.create_session(world_id, world_data).await;

            let client_id_str = client_id.to_string();
            let join_info = self
                .sessions
                .join_session(
                    session_id,
                    &client_id_str,
                    user_id,
                    map_role_to_session_role(role),
                )
                .await
                .map_err(|e| SessionError::Internal(format!("Failed to join demo session: {}", e)))?;

            // Gather participant info
            let participants = gather_participants(&*self.sessions, session_id).await;

            let snapshot_msg = ServerMessage::SessionSnapshot {
                session_id: session_id.to_string(),
                world_snapshot: join_info.world_snapshot_json.clone(),
            };
            if let Err(e) = sender.send(snapshot_msg) {
                tracing::warn!("Failed to send initial demo session snapshot to client {}: {}", client_id, e);
            }

            Ok(SessionJoinedInfo {
                session_id,
                participants,
                world_snapshot: join_info.world_snapshot_json,
            })
        }
    }
}

/// Map application-level session participant role to websocket role DTO
fn map_role_to_websocket_role(role: SessionParticipantRole) -> ParticipantRole {
    match role {
        SessionParticipantRole::DungeonMaster => ParticipantRole::DungeonMaster,
        SessionParticipantRole::Player => ParticipantRole::Player,
        SessionParticipantRole::Spectator => ParticipantRole::Spectator,
    }
}

/// Map websocket participant role to session participant role
fn map_role_to_session_role(role: ParticipantRole) -> SessionParticipantRole {
    match role {
        ParticipantRole::DungeonMaster => SessionParticipantRole::DungeonMaster,
        ParticipantRole::Player => SessionParticipantRole::Player,
        ParticipantRole::Spectator => SessionParticipantRole::Spectator,
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
            role: map_role_to_websocket_role(p.role),
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

    WorldSnapshot {
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


