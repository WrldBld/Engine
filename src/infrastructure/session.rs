//! Session management for active game sessions
//!
//! This module provides session tracking for WebSocket connections,
//! allowing multiple clients to join a shared game session and
//! receive synchronized updates.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::domain::entities::{Character, Location, Scene, World};
use crate::domain::value_objects::{SessionId, WorldId};
use crate::infrastructure::websocket::{ParticipantRole, ServerMessage};

/// Unique identifier for a connected client
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(uuid::Uuid);

impl ClientId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for ClientId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A participant in a game session
#[derive(Debug, Clone)]
pub struct SessionParticipant {
    pub client_id: ClientId,
    pub user_id: String,
    pub role: ParticipantRole,
    pub joined_at: DateTime<Utc>,
    /// Channel to send messages to this client
    pub sender: mpsc::UnboundedSender<ServerMessage>,
}

/// A snapshot of the current world state for session joining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub world: World,
    pub locations: Vec<Location>,
    pub characters: Vec<Character>,
    pub scenes: Vec<Scene>,
    pub current_scene_id: Option<String>,
}

impl WorldSnapshot {
    /// Convert to a JSON value for transmission
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "world": {
                "id": self.world.id.to_string(),
                "name": &self.world.name,
                "description": &self.world.description
            },
            "locations": self.locations.iter().map(|l| serde_json::json!({
                "id": l.id.to_string(),
                "name": &l.name,
                "description": &l.description,
                "backdrop_asset": &l.backdrop_asset,
                "location_type": format!("{:?}", l.location_type)
            })).collect::<Vec<_>>(),
            "characters": self.characters.iter().map(|c| serde_json::json!({
                "id": c.id.to_string(),
                "name": &c.name,
                "description": &c.description,
                "sprite_asset": &c.sprite_asset,
                "portrait_asset": &c.portrait_asset,
                "archetype": format!("{:?}", c.current_archetype)
            })).collect::<Vec<_>>(),
            "scenes": self.scenes.iter().map(|s| serde_json::json!({
                "id": s.id.to_string(),
                "name": &s.name,
                "location_id": s.location_id.to_string(),
                "directorial_notes": &s.directorial_notes
            })).collect::<Vec<_>>(),
            "current_scene_id": &self.current_scene_id
        })
    }
}

/// An active game session
#[derive(Debug)]
pub struct GameSession {
    pub id: SessionId,
    pub world_id: WorldId,
    pub world_snapshot: Arc<WorldSnapshot>,
    pub participants: HashMap<ClientId, SessionParticipant>,
    pub created_at: DateTime<Utc>,
    pub current_scene_id: Option<String>,
}

impl GameSession {
    /// Create a new game session for a world
    pub fn new(world_id: WorldId, world_snapshot: WorldSnapshot) -> Self {
        Self {
            id: SessionId::new(),
            world_id,
            world_snapshot: Arc::new(world_snapshot),
            participants: HashMap::new(),
            created_at: Utc::now(),
            current_scene_id: None,
        }
    }

    /// Add a participant to the session
    pub fn add_participant(
        &mut self,
        client_id: ClientId,
        user_id: String,
        role: ParticipantRole,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) {
        let participant = SessionParticipant {
            client_id,
            user_id,
            role,
            joined_at: Utc::now(),
            sender,
        };
        self.participants.insert(client_id, participant);
    }

    /// Remove a participant from the session
    pub fn remove_participant(&mut self, client_id: ClientId) -> Option<SessionParticipant> {
        self.participants.remove(&client_id)
    }

    /// Check if a DM is present in the session
    pub fn has_dm(&self) -> bool {
        self.participants
            .values()
            .any(|p| p.role == ParticipantRole::DungeonMaster)
    }

    /// Get the DM participant if present
    pub fn get_dm(&self) -> Option<&SessionParticipant> {
        self.participants
            .values()
            .find(|p| p.role == ParticipantRole::DungeonMaster)
    }

    /// Broadcast a message to all participants
    pub fn broadcast(&self, message: &ServerMessage) {
        for participant in self.participants.values() {
            if let Err(e) = participant.sender.send(message.clone()) {
                tracing::warn!(
                    "Failed to send message to client {}: {}",
                    participant.client_id,
                    e
                );
            }
        }
    }

    /// Broadcast a message to all participants except one
    pub fn broadcast_except(&self, message: &ServerMessage, exclude: ClientId) {
        for participant in self.participants.values() {
            if participant.client_id != exclude {
                if let Err(e) = participant.sender.send(message.clone()) {
                    tracing::warn!(
                        "Failed to send message to client {}: {}",
                        participant.client_id,
                        e
                    );
                }
            }
        }
    }

    /// Send a message only to the DM
    pub fn send_to_dm(&self, message: &ServerMessage) {
        if let Some(dm) = self.get_dm() {
            if let Err(e) = dm.sender.send(message.clone()) {
                tracing::warn!("Failed to send message to DM: {}", e);
            }
        }
    }

    /// Send a message to players only (excludes DM and spectators)
    pub fn broadcast_to_players(&self, message: &ServerMessage) {
        for participant in self.participants.values() {
            if participant.role == ParticipantRole::Player {
                if let Err(e) = participant.sender.send(message.clone()) {
                    tracing::warn!(
                        "Failed to send message to player {}: {}",
                        participant.client_id,
                        e
                    );
                }
            }
        }
    }

    /// Get the number of active participants
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Check if the session is empty
    pub fn is_empty(&self) -> bool {
        self.participants.is_empty()
    }
}

/// Error types for session operations
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(SessionId),

    #[error("World not found: {0}")]
    WorldNotFound(String),

    #[error("Client not in any session: {0}")]
    ClientNotInSession(ClientId),

    #[error("Session already has a DM")]
    DmAlreadyPresent,

    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),
}

/// Manages active game sessions
pub struct SessionManager {
    /// Active sessions by session ID
    sessions: HashMap<SessionId, GameSession>,
    /// Maps client IDs to their current session
    client_sessions: HashMap<ClientId, SessionId>,
    /// Maps world IDs to active sessions (for finding existing sessions)
    world_sessions: HashMap<WorldId, SessionId>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            client_sessions: HashMap::new(),
            world_sessions: HashMap::new(),
        }
    }

    /// Create a new session for a world
    pub fn create_session(
        &mut self,
        world_id: WorldId,
        world_snapshot: WorldSnapshot,
    ) -> SessionId {
        let session = GameSession::new(world_id, world_snapshot);
        let session_id = session.id;

        self.world_sessions.insert(world_id, session_id);
        self.sessions.insert(session_id, session);

        tracing::info!("Created new session {} for world {}", session_id, world_id);
        session_id
    }

    /// Find an existing session for a world, or return None
    pub fn find_session_for_world(&self, world_id: WorldId) -> Option<SessionId> {
        self.world_sessions.get(&world_id).copied()
    }

    /// Join an existing session or create a new one
    pub fn join_session(
        &mut self,
        session_id: SessionId,
        client_id: ClientId,
        user_id: String,
        role: ParticipantRole,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) -> Result<Arc<WorldSnapshot>, SessionError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(SessionError::NotFound(session_id))?;

        // Check if trying to join as DM when one already exists
        if role == ParticipantRole::DungeonMaster && session.has_dm() {
            return Err(SessionError::DmAlreadyPresent);
        }

        session.add_participant(client_id, user_id.clone(), role, sender);
        self.client_sessions.insert(client_id, session_id);

        tracing::info!(
            "Client {} (user: {}) joined session {} as {:?}",
            client_id,
            user_id,
            session_id,
            role
        );

        Ok(Arc::clone(&session.world_snapshot))
    }

    /// Leave a session
    pub fn leave_session(
        &mut self,
        client_id: ClientId,
    ) -> Option<(SessionId, SessionParticipant)> {
        if let Some(session_id) = self.client_sessions.remove(&client_id) {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                if let Some(participant) = session.remove_participant(client_id) {
                    tracing::info!(
                        "Client {} left session {} (user: {})",
                        client_id,
                        session_id,
                        participant.user_id
                    );

                    // If session is empty, clean it up
                    if session.is_empty() {
                        let world_id = session.world_id;
                        self.sessions.remove(&session_id);
                        self.world_sessions.remove(&world_id);
                        tracing::info!("Removed empty session {}", session_id);
                    }

                    return Some((session_id, participant));
                }
            }
        }
        None
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: SessionId) -> Option<&GameSession> {
        self.sessions.get(&session_id)
    }

    /// Get a mutable session by ID
    pub fn get_session_mut(&mut self, session_id: SessionId) -> Option<&mut GameSession> {
        self.sessions.get_mut(&session_id)
    }

    /// Get the session ID for a client
    pub fn get_client_session(&self, client_id: ClientId) -> Option<SessionId> {
        self.client_sessions.get(&client_id).copied()
    }

    /// Broadcast a message to all participants in a session
    pub fn broadcast_to_session(&self, session_id: SessionId, message: &ServerMessage) {
        if let Some(session) = self.sessions.get(&session_id) {
            session.broadcast(message);
        }
    }

    /// Broadcast a message to all participants except one
    pub fn broadcast_to_session_except(
        &self,
        session_id: SessionId,
        message: &ServerMessage,
        exclude: ClientId,
    ) {
        if let Some(session) = self.sessions.get(&session_id) {
            session.broadcast_except(message, exclude);
        }
    }

    /// Get the number of active sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.client_sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::RuleSystemConfig;

    fn create_test_world() -> World {
        World {
            id: WorldId::new(),
            name: "Test World".to_string(),
            description: "A test world".to_string(),
            rule_system: RuleSystemConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_snapshot(world: World) -> WorldSnapshot {
        WorldSnapshot {
            world,
            locations: vec![],
            characters: vec![],
            scenes: vec![],
            current_scene_id: None,
        }
    }

    #[test]
    fn test_create_session() {
        let mut manager = SessionManager::new();
        let world = create_test_world();
        let world_id = world.id;
        let snapshot = create_test_snapshot(world);

        let session_id = manager.create_session(world_id, snapshot);

        assert!(manager.get_session(session_id).is_some());
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_join_session() {
        let mut manager = SessionManager::new();
        let world = create_test_world();
        let world_id = world.id;
        let snapshot = create_test_snapshot(world);

        let session_id = manager.create_session(world_id, snapshot);
        let client_id = ClientId::new();
        let (tx, _rx) = mpsc::unbounded_channel();

        let result = manager.join_session(
            session_id,
            client_id,
            "test_user".to_string(),
            ParticipantRole::Player,
            tx,
        );

        assert!(result.is_ok());
        assert_eq!(manager.get_client_session(client_id), Some(session_id));
    }

    #[test]
    fn test_leave_session() {
        let mut manager = SessionManager::new();
        let world = create_test_world();
        let world_id = world.id;
        let snapshot = create_test_snapshot(world);

        let session_id = manager.create_session(world_id, snapshot);
        let client_id = ClientId::new();
        let (tx, _rx) = mpsc::unbounded_channel();

        let _ = manager.join_session(
            session_id,
            client_id,
            "test_user".to_string(),
            ParticipantRole::Player,
            tx,
        );

        let result = manager.leave_session(client_id);

        assert!(result.is_some());
        assert!(manager.get_client_session(client_id).is_none());
        // Session should be removed when empty
        assert!(manager.get_session(session_id).is_none());
    }

    #[test]
    fn test_dm_restriction() {
        let mut manager = SessionManager::new();
        let world = create_test_world();
        let world_id = world.id;
        let snapshot = create_test_snapshot(world);

        let session_id = manager.create_session(world_id, snapshot);

        // First DM joins
        let dm1_id = ClientId::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let result1 = manager.join_session(
            session_id,
            dm1_id,
            "dm1".to_string(),
            ParticipantRole::DungeonMaster,
            tx1,
        );
        assert!(result1.is_ok());

        // Second DM tries to join
        let dm2_id = ClientId::new();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let result2 = manager.join_session(
            session_id,
            dm2_id,
            "dm2".to_string(),
            ParticipantRole::DungeonMaster,
            tx2,
        );
        assert!(matches!(result2, Err(SessionError::DmAlreadyPresent)));
    }
}
