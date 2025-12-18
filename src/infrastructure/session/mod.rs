//! Session management for active game sessions
//!
//! This module provides session tracking for WebSocket connections,
//! allowing multiple clients to join a shared game session and
//! receive synchronized updates. It also maintains conversation history
//! for LLM context.

mod conversation;
mod errors;
mod game_session;

// Re-export all public types
pub use errors::SessionError;
pub use game_session::{GameSession, PendingApproval, SessionParticipant};

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::mpsc;

use crate::application::dto::WorldSnapshot;
use crate::application::ports::outbound::{
    BroadcastMessage, CharacterContextInfo, PendingApprovalInfo,
    SessionManagementError, SessionManagementPort, SessionWorldContext,
};
use crate::domain::value_objects::{SessionId, WorldId};
use crate::infrastructure::websocket::{ParticipantRole, ServerMessage};

/// Unique identifier for a connected client
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(uuid::Uuid);

impl ClientId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create a ClientId from an existing UUID
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
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

/// Manages active game sessions
pub struct SessionManager {
    /// Active sessions by session ID
    sessions: HashMap<SessionId, GameSession>,
    /// Maps client IDs to their current session
    client_sessions: HashMap<ClientId, SessionId>,
    /// Maps world IDs to active sessions (for finding existing sessions)
    world_sessions: HashMap<WorldId, SessionId>,
    /// Maximum conversation history turns to retain per session
    max_conversation_history: usize,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(max_conversation_history: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            client_sessions: HashMap::new(),
            world_sessions: HashMap::new(),
            max_conversation_history,
        }
    }

    /// Get all active session IDs.
    ///
    /// NOTE: Prefer `list_sessions` for application-facing callers. This
    /// helper is kept only for legacy/debug code paths and may be removed in
    /// a future cleanup.
    pub fn get_session_ids(&self) -> Vec<SessionId> {
        self.sessions.keys().copied().collect()
    }

    /// Create a new session for a world with a generated session ID
    pub fn create_session(
        &mut self,
        world_id: WorldId,
        world_snapshot: WorldSnapshot,
    ) -> SessionId {
        let session = GameSession::new(world_id, world_snapshot, self.max_conversation_history);
        let session_id = session.id;

        self.world_sessions.insert(world_id, session_id);
        self.sessions.insert(session_id, session);

        tracing::info!("Created new session {} for world {}", session_id, world_id);
        session_id
    }

    /// Create a new session for a world with an explicit session ID
    pub fn create_session_with_id(
        &mut self,
        session_id: SessionId,
        world_id: WorldId,
        world_snapshot: WorldSnapshot,
    ) -> SessionId {
        let session =
            GameSession::new_with_id(session_id, world_id, world_snapshot, self.max_conversation_history);

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

        // Check if trying to join as DM when one already exists with a different user_id
        // Allow multiple DM connections from the same user_id (for multiple tabs/windows)
        if role == ParticipantRole::DungeonMaster && session.has_dm() {
            if let Some(existing_dm) = session.get_dm() {
                // Only reject if the existing DM has a different user_id
                if existing_dm.user_id != user_id {
                    return Err(SessionError::DmAlreadyPresent);
                }
                // Same user_id is allowed - they can have multiple tabs/windows
            }
        }

        // Record the DM user ID for session metadata when a DM joins
        if role == ParticipantRole::DungeonMaster && session.dm_user_id.is_none() {
            session.dm_user_id = Some(user_id.clone());
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
    #[allow(dead_code)] // Kept for future monitoring/metrics features
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the number of connected clients
    #[allow(dead_code)] // Kept for future monitoring/metrics features
    pub fn client_count(&self) -> usize {
        self.client_sessions.len()
    }

    /// Get all active session IDs (canonical helper; prefer this over
    /// `get_session_ids` in new code).
    pub fn list_sessions(&self) -> Vec<SessionId> {
        self.sessions.keys().copied().collect()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(30) // Default to 30 conversation turns
    }
}

/// Helper to parse a client ID string to ClientId
fn parse_client_id(client_id_str: &str) -> Option<ClientId> {
    uuid::Uuid::parse_str(client_id_str)
        .ok()
        .map(ClientId)
}

/// Convert BroadcastMessage to ServerMessage by deserializing the JSON
fn broadcast_to_server_message(msg: &BroadcastMessage) -> Option<ServerMessage> {
    serde_json::from_value(msg.content.clone()).ok()
}

/// Implement SessionManagementPort for SessionManager
///
/// This implementation bridges the application layer's abstract port interface
/// to the concrete infrastructure implementation.
impl SessionManagementPort for SessionManager {
    fn get_client_session(&self, client_id: &str) -> Option<SessionId> {
        let client_id = parse_client_id(client_id)?;
        self.client_sessions.get(&client_id).copied()
    }

    fn is_client_dm(&self, client_id: &str) -> bool {
        let Some(client_id) = parse_client_id(client_id) else {
            return false;
        };
        let Some(session_id) = self.client_sessions.get(&client_id) else {
            return false;
        };
        let Some(session) = self.sessions.get(session_id) else {
            return false;
        };
        session
            .get_dm()
            .map(|dm| dm.client_id == client_id)
            .unwrap_or(false)
    }

    fn get_client_user_id(&self, client_id: &str) -> Option<String> {
        let client_id = parse_client_id(client_id)?;
        let session_id = self.client_sessions.get(&client_id)?;
        let session = self.sessions.get(session_id)?;
        session
            .participants
            .get(&client_id)
            .map(|p| p.user_id.clone())
    }

    fn get_pending_approval(
        &self,
        session_id: SessionId,
        request_id: &str,
    ) -> Option<PendingApprovalInfo> {
        let session = self.sessions.get(&session_id)?;
        let pending = session.get_pending_approval(request_id)?;
        Some(PendingApprovalInfo {
            request_id: pending.request_id.clone(),
            npc_name: pending.npc_name.clone(),
            proposed_dialogue: pending.proposed_dialogue.clone(),
            internal_reasoning: pending.internal_reasoning.clone(),
            proposed_tools: pending.proposed_tools.clone(),
            retry_count: pending.retry_count,
        })
    }

    fn add_pending_approval(
        &mut self,
        session_id: SessionId,
        approval: PendingApprovalInfo,
    ) -> Result<(), SessionManagementError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| SessionManagementError::SessionNotFound(session_id.to_string()))?;

        let pending = PendingApproval {
            request_id: approval.request_id,
            npc_name: approval.npc_name,
            proposed_dialogue: approval.proposed_dialogue,
            internal_reasoning: approval.internal_reasoning,
            proposed_tools: approval.proposed_tools,
            retry_count: approval.retry_count,
            requested_at: Utc::now(),
        };

        session.add_pending_approval(pending);
        Ok(())
    }

    fn remove_pending_approval(
        &mut self,
        session_id: SessionId,
        request_id: &str,
    ) -> Result<(), SessionManagementError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| SessionManagementError::SessionNotFound(session_id.to_string()))?;

        session
            .remove_pending_approval(request_id)
            .ok_or_else(|| SessionManagementError::ApprovalNotFound(request_id.to_string()))?;

        Ok(())
    }

    fn increment_retry_count(
        &mut self,
        session_id: SessionId,
        request_id: &str,
    ) -> Result<u32, SessionManagementError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| SessionManagementError::SessionNotFound(session_id.to_string()))?;

        let pending = session
            .get_pending_approval_mut(request_id)
            .ok_or_else(|| SessionManagementError::ApprovalNotFound(request_id.to_string()))?;

        pending.retry_count += 1;
        Ok(pending.retry_count)
    }

    fn broadcast_to_players(
        &self,
        session_id: SessionId,
        message: &BroadcastMessage,
    ) -> Result<(), SessionManagementError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| SessionManagementError::SessionNotFound(session_id.to_string()))?;

        if let Some(server_msg) = broadcast_to_server_message(message) {
            session.broadcast_to_players(&server_msg);
        }
        Ok(())
    }

    fn send_to_dm(
        &self,
        session_id: SessionId,
        message: &BroadcastMessage,
    ) -> Result<(), SessionManagementError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| SessionManagementError::SessionNotFound(session_id.to_string()))?;

        if let Some(server_msg) = broadcast_to_server_message(message) {
            session.send_to_dm(&server_msg);
        }
        Ok(())
    }

    fn broadcast_except(
        &self,
        session_id: SessionId,
        message: &BroadcastMessage,
        exclude_client: &str,
    ) -> Result<(), SessionManagementError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| SessionManagementError::SessionNotFound(session_id.to_string()))?;

        let exclude_id = parse_client_id(exclude_client)
            .ok_or(SessionManagementError::ClientNotInSession)?;

        if let Some(server_msg) = broadcast_to_server_message(message) {
            session.broadcast_except(&server_msg, exclude_id);
        }
        Ok(())
    }

    fn broadcast_to_session(
        &self,
        session_id: SessionId,
        message: &BroadcastMessage,
    ) -> Result<(), SessionManagementError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| SessionManagementError::SessionNotFound(session_id.to_string()))?;

        if let Some(server_msg) = broadcast_to_server_message(message) {
            session.broadcast(&server_msg);
        }
        Ok(())
    }

    fn add_to_conversation_history(
        &mut self,
        session_id: SessionId,
        speaker: &str,
        text: &str,
    ) -> Result<(), SessionManagementError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| SessionManagementError::SessionNotFound(session_id.to_string()))?;

        session.add_npc_response(speaker, text);
        Ok(())
    }

    fn session_has_dm(&self, session_id: SessionId) -> bool {
        self.sessions
            .get(&session_id)
            .map(|s| s.has_dm())
            .unwrap_or(false)
    }

    fn get_session_world_context(
        &self,
        session_id: SessionId,
    ) -> Option<SessionWorldContext> {
        let session = self.sessions.get(&session_id)?;
        let snapshot = &session.world_snapshot;

        // Get current scene
        let current_scene = session
            .current_scene_id
            .as_ref()
            .and_then(|scene_id| {
                snapshot.scenes.iter().find(|s| s.id.to_string() == *scene_id)
            })
            .or_else(|| snapshot.scenes.first())?;

        // Get location for the scene
        let location = snapshot
            .locations
            .iter()
            .find(|l| l.id == current_scene.location_id);

        // Get present character names
        let present_character_names: Vec<String> = current_scene
            .featured_characters
            .iter()
            .filter_map(|char_id| {
                snapshot
                    .characters
                    .iter()
                    .find(|c| c.id == *char_id)
                    .map(|c| c.name.clone())
            })
            .collect();

        // Build character context map
        // NOTE: Wants are stored as graph edges and require async DB queries to fetch.
        // The primary LLM prompt builder (build_prompt_from_action) fetches wants directly.
        // This context is used for scene presence info where wants are less critical.
        // To add wants here, either:
        // 1. Pre-populate wants in WorldSnapshot during session creation, or
        // 2. Add CharacterRepositoryPort dependency to SessionManager
        let mut characters = std::collections::HashMap::new();
        for character in &snapshot.characters {
            characters.insert(
                character.name.clone(),
                CharacterContextInfo {
                    name: character.name.clone(),
                    archetype: format!("{:?}", character.current_archetype),
                    wants: Vec::new(), // See note above
                },
            );
        }

        Some(SessionWorldContext {
            scene_name: current_scene.name.clone(),
            location_name: location.map(|l| l.name.clone()).unwrap_or_else(|| "Unknown".to_string()),
            time_context: match &current_scene.time_context {
                crate::domain::entities::TimeContext::Unspecified => "Unspecified".to_string(),
                crate::domain::entities::TimeContext::TimeOfDay(tod) => format!("{:?}", tod),
                crate::domain::entities::TimeContext::During(s) => s.clone(),
                crate::domain::entities::TimeContext::Custom(s) => s.clone(),
            },
            present_character_names,
            characters,
            directorial_notes: current_scene.directorial_notes.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::World;
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
        let mut manager = SessionManager::new(30);
        let world = create_test_world();
        let world_id = world.id;
        let snapshot = create_test_snapshot(world);

        let session_id = manager.create_session(world_id, snapshot);

        assert!(manager.get_session(session_id).is_some());
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_join_session() {
        let mut manager = SessionManager::new(30);
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
        let mut manager = SessionManager::new(30);
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
        let mut manager = SessionManager::new(30);
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

        // Second DM with different user_id tries to join - should be rejected
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

        // Same user_id (dm1) tries to join again (multiple tabs) - should be allowed
        let dm1_tab2_id = ClientId::new();
        let (tx1_tab2, _rx1_tab2) = mpsc::unbounded_channel();
        let result3 = manager.join_session(
            session_id,
            dm1_tab2_id,
            "dm1".to_string(), // Same user_id as first DM
            ParticipantRole::DungeonMaster,
            tx1_tab2,
        );
        assert!(result3.is_ok(), "Same user_id should be allowed to join multiple times");
    }
}
