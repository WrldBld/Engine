//! Session Manager Adapter - Implements AsyncSessionPort for SessionManager
//!
//! This adapter wraps the concrete SessionManager implementation and provides
//! the async-aware interface required by application services.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::application::dto::WorldSnapshot;
use crate::application::ports::outbound::{
    AsyncSessionError, AsyncSessionPort, SessionJoinInfo, SessionParticipantInfo,
    SessionParticipantRole, SessionWorldData,
};
use crate::domain::value_objects::{ProposedToolInfo, SessionId, WorldId};
use crate::infrastructure::session::{ClientId, PendingApproval, SessionError, SessionManager};
use crate::infrastructure::websocket::messages::{ParticipantRole, ServerMessage};

/// Adapter that wraps SessionManager and implements AsyncSessionPort
pub struct SessionManagerAdapter {
    inner: Arc<RwLock<SessionManager>>,
}

impl SessionManagerAdapter {
    /// Create a new adapter wrapping a SessionManager
    pub fn new(manager: Arc<RwLock<SessionManager>>) -> Self {
        Self { inner: manager }
    }
}

/// Convert application role to infrastructure role
fn to_infra_role(role: SessionParticipantRole) -> ParticipantRole {
    match role {
        SessionParticipantRole::DungeonMaster => ParticipantRole::DungeonMaster,
        SessionParticipantRole::Player => ParticipantRole::Player,
        SessionParticipantRole::Spectator => ParticipantRole::Spectator,
    }
}

/// Convert infrastructure role to application role
fn from_infra_role(role: ParticipantRole) -> SessionParticipantRole {
    match role {
        ParticipantRole::DungeonMaster => SessionParticipantRole::DungeonMaster,
        ParticipantRole::Player => SessionParticipantRole::Player,
        ParticipantRole::Spectator => SessionParticipantRole::Spectator,
    }
}

/// Parse a client ID string to infrastructure ClientId
fn parse_client_id(client_id_str: &str) -> Option<ClientId> {
    uuid::Uuid::parse_str(client_id_str)
        .ok()
        .map(|uuid| ClientId::from_uuid(uuid))
}

/// Convert SessionError to AsyncSessionError
fn convert_error(err: SessionError) -> AsyncSessionError {
    match err {
        SessionError::NotFound(_) => AsyncSessionError::SessionNotFound(err.to_string()),
        SessionError::WorldNotFound(w) => AsyncSessionError::WorldNotFound(w),
        SessionError::ClientNotInSession(_) => AsyncSessionError::ClientNotInSession,
        SessionError::DmAlreadyPresent => AsyncSessionError::DmAlreadyPresent,
        SessionError::Database(e) => AsyncSessionError::Internal(e.to_string()),
    }
}

/// Convert JSON to WorldSnapshot for session creation
fn json_to_world_snapshot(_json: SessionWorldData) -> WorldSnapshot {
    // For now, create a minimal snapshot. In production, this would
    // deserialize the full world data from the JSON value.
    // WorldSnapshot contains domain types that don't implement Deserialize,
    // so full deserialization requires mapping JSON -> domain types first.
    WorldSnapshot::default()
}

#[async_trait]
impl AsyncSessionPort for SessionManagerAdapter {
    async fn get_client_session(&self, client_id: &str) -> Option<SessionId> {
        let client_id = parse_client_id(client_id)?;
        let sessions = self.inner.read().await;
        sessions.get_client_session(client_id)
    }

    async fn is_client_dm(&self, client_id: &str) -> bool {
        let Some(client_id) = parse_client_id(client_id) else {
            return false;
        };
        let sessions = self.inner.read().await;
        let Some(session_id) = sessions.get_client_session(client_id) else {
            return false;
        };
        let Some(session) = sessions.get_session(session_id) else {
            return false;
        };
        session
            .get_dm()
            .map(|dm| dm.client_id == client_id)
            .unwrap_or(false)
    }

    async fn get_client_user_id(&self, client_id: &str) -> Option<String> {
        let client_id = parse_client_id(client_id)?;
        let sessions = self.inner.read().await;
        let session_id = sessions.get_client_session(client_id)?;
        let session = sessions.get_session(session_id)?;
        session.participants.get(&client_id).map(|p| p.user_id.clone())
    }

    async fn get_participant_info(&self, client_id: &str) -> Option<SessionParticipantInfo> {
        let client_id_parsed = parse_client_id(client_id)?;
        let sessions = self.inner.read().await;
        let session_id = sessions.get_client_session(client_id_parsed)?;
        let session = sessions.get_session(session_id)?;
        let participant = session.participants.get(&client_id_parsed)?;
        let character_name = session.get_character_name_for_user(&participant.user_id);
        Some(SessionParticipantInfo {
            client_id: client_id.to_string(),
            user_id: participant.user_id.clone(),
            role: from_infra_role(participant.role),
            character_name,
        })
    }

    async fn get_session_world_id(&self, session_id: SessionId) -> Option<WorldId> {
        let sessions = self.inner.read().await;
        sessions.get_session(session_id).map(|s| s.world_id)
    }

    async fn find_session_for_world(&self, world_id: WorldId) -> Option<SessionId> {
        let sessions = self.inner.read().await;
        sessions.find_session_for_world(world_id)
    }

    async fn create_session(
        &self,
        world_id: WorldId,
        world_snapshot: SessionWorldData,
    ) -> SessionId {
        let snapshot = json_to_world_snapshot(world_snapshot);
        let mut sessions = self.inner.write().await;
        sessions.create_session(world_id, snapshot)
    }

    async fn create_session_with_id(
        &self,
        session_id: SessionId,
        world_id: WorldId,
        world_snapshot: SessionWorldData,
    ) -> SessionId {
        let snapshot = json_to_world_snapshot(world_snapshot);
        let mut sessions = self.inner.write().await;
        sessions.create_session_with_id(session_id, world_id, snapshot)
    }

    async fn join_session(
        &self,
        session_id: SessionId,
        client_id: &str,
        user_id: String,
        role: SessionParticipantRole,
    ) -> Result<SessionJoinInfo, AsyncSessionError> {
        let client_id_parsed = parse_client_id(client_id)
            .ok_or_else(|| AsyncSessionError::Internal("Invalid client ID format".to_string()))?;

        // Create a dummy sender for now - in practice, the caller would provide this
        // through a different mechanism or we'd need to extend the interface
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();

        let mut sessions = self.inner.write().await;
        let world_snapshot = sessions
            .join_session(session_id, client_id_parsed, user_id, to_infra_role(role), tx)
            .map_err(convert_error)?;

        Ok(SessionJoinInfo {
            session_id,
            world_snapshot_json: world_snapshot.to_json(),
        })
    }

    async fn broadcast_to_session(
        &self,
        session_id: SessionId,
        message: serde_json::Value,
    ) -> Result<(), AsyncSessionError> {
        let server_msg: ServerMessage = serde_json::from_value(message)
            .map_err(|e| AsyncSessionError::Internal(format!("Invalid message format: {}", e)))?;

        let sessions = self.inner.read().await;
        sessions.broadcast_to_session(session_id, &server_msg);
        Ok(())
    }

    async fn broadcast_to_players(
        &self,
        session_id: SessionId,
        message: serde_json::Value,
    ) -> Result<(), AsyncSessionError> {
        let server_msg: ServerMessage = serde_json::from_value(message)
            .map_err(|e| AsyncSessionError::Internal(format!("Invalid message format: {}", e)))?;

        let sessions = self.inner.read().await;
        if let Some(session) = sessions.get_session(session_id) {
            session.broadcast_to_players(&server_msg);
        }
        Ok(())
    }

    async fn send_to_dm(
        &self,
        session_id: SessionId,
        message: serde_json::Value,
    ) -> Result<(), AsyncSessionError> {
        let server_msg: ServerMessage = serde_json::from_value(message)
            .map_err(|e| AsyncSessionError::Internal(format!("Invalid message format: {}", e)))?;

        let sessions = self.inner.read().await;
        if let Some(session) = sessions.get_session(session_id) {
            session.send_to_dm(&server_msg);
        }
        Ok(())
    }

    async fn broadcast_except(
        &self,
        session_id: SessionId,
        message: serde_json::Value,
        exclude_client: &str,
    ) -> Result<(), AsyncSessionError> {
        let exclude_id = parse_client_id(exclude_client)
            .ok_or_else(|| AsyncSessionError::Internal("Invalid exclude client ID".to_string()))?;

        let server_msg: ServerMessage = serde_json::from_value(message)
            .map_err(|e| AsyncSessionError::Internal(format!("Invalid message format: {}", e)))?;

        let sessions = self.inner.read().await;
        sessions.broadcast_to_session_except(session_id, &server_msg, exclude_id);
        Ok(())
    }

    async fn get_session_participants(
        &self,
        session_id: SessionId,
    ) -> Vec<SessionParticipantInfo> {
        let sessions = self.inner.read().await;
        sessions
            .get_session(session_id)
            .map(|session| {
                session
                    .participants
                    .values()
                    .map(|p| SessionParticipantInfo {
                        client_id: p.client_id.to_string(),
                        user_id: p.user_id.clone(),
                        role: from_infra_role(p.role),
                        character_name: session.get_character_name_for_user(&p.user_id),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn add_to_conversation_history(
        &self,
        session_id: SessionId,
        speaker: &str,
        text: &str,
    ) -> Result<(), AsyncSessionError> {
        let mut sessions = self.inner.write().await;
        if let Some(session) = sessions.get_session_mut(session_id) {
            session.add_npc_response(speaker, text);
            Ok(())
        } else {
            Err(AsyncSessionError::SessionNotFound(session_id.to_string()))
        }
    }

    async fn session_has_dm(&self, session_id: SessionId) -> bool {
        let sessions = self.inner.read().await;
        sessions
            .get_session(session_id)
            .map(|s| s.has_dm())
            .unwrap_or(false)
    }

    async fn get_session_snapshot(&self, session_id: SessionId) -> Option<serde_json::Value> {
        let sessions = self.inner.read().await;
        sessions
            .get_session(session_id)
            .map(|s| s.world_snapshot.to_json())
    }

    async fn list_session_ids(&self) -> Vec<SessionId> {
        let sessions = self.inner.read().await;
        sessions.get_session_ids()
    }

    // === New methods for WebSocket handler refactoring ===

    async fn client_leave_session(&self, client_id: &str) -> Option<(SessionId, SessionParticipantInfo)> {
        let client_id_parsed = parse_client_id(client_id)?;
        let mut sessions = self.inner.write().await;
        let (session_id, participant) = sessions.leave_session(client_id_parsed)?;
        // Note: character_name is None here because leave_session removes the participant
        // and we no longer have access to the session's character data for this user
        Some((
            session_id,
            SessionParticipantInfo {
                client_id: client_id.to_string(),
                user_id: participant.user_id,
                role: from_infra_role(participant.role),
                character_name: None,
            },
        ))
    }

    async fn update_session_scene(&self, session_id: SessionId, scene_id: String) -> Result<(), AsyncSessionError> {
        let mut sessions = self.inner.write().await;
        if let Some(session) = sessions.get_session_mut(session_id) {
            session.current_scene_id = Some(scene_id);
            Ok(())
        } else {
            Err(AsyncSessionError::SessionNotFound(session_id.to_string()))
        }
    }

    async fn send_to_participant(
        &self,
        session_id: SessionId,
        user_id: &str,
        message: serde_json::Value,
    ) -> Result<(), AsyncSessionError> {
        let server_msg: ServerMessage = serde_json::from_value(message)
            .map_err(|e| AsyncSessionError::Internal(format!("Invalid message format: {}", e)))?;

        let sessions = self.inner.read().await;
        if let Some(session) = sessions.get_session(session_id) {
            session.send_to_participant(user_id, &server_msg);
            Ok(())
        } else {
            Err(AsyncSessionError::SessionNotFound(session_id.to_string()))
        }
    }

    async fn get_session_dm(&self, session_id: SessionId) -> Option<SessionParticipantInfo> {
        let sessions = self.inner.read().await;
        let session = sessions.get_session(session_id)?;
        let dm = session.get_dm()?;
        // DM typically doesn't have a character (they're the game master)
        Some(SessionParticipantInfo {
            client_id: dm.client_id.to_string(),
            user_id: dm.user_id.clone(),
            role: SessionParticipantRole::DungeonMaster,
            character_name: None,
        })
    }

    // === New methods for queue worker refactoring ===

    async fn register_pending_approval(
        &self,
        session_id: SessionId,
        approval_id: String,
        npc_name: String,
        proposed_dialogue: String,
        internal_reasoning: Option<String>,
        proposed_tools: Vec<ProposedToolInfo>,
    ) -> Result<bool, AsyncSessionError> {
        let mut sessions = self.inner.write().await;
        if let Some(session) = sessions.get_session_mut(session_id) {
            // Check if already registered
            if session.get_pending_approval(&approval_id).is_some() {
                return Ok(false);
            }
            // Create and add the pending approval
            let pending = PendingApproval::new(
                approval_id,
                npc_name,
                proposed_dialogue,
                internal_reasoning.unwrap_or_default(),
                proposed_tools,
            );
            session.add_pending_approval(pending);
            Ok(true)
        } else {
            Err(AsyncSessionError::SessionNotFound(session_id.to_string()))
        }
    }
}
