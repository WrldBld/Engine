//! Async Session Port - Async interface for session management operations
//!
//! This port provides async-friendly session management operations for use by
//! application services. It abstracts over the locking/synchronization details
//! of the underlying session manager implementation.


use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::domain::value_objects::{SessionId, WorldId};

/// Participant role in a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionParticipantRole {
    DungeonMaster,
    Player,
    Spectator,
}

/// Information about a session participant
#[derive(Debug, Clone)]
pub struct SessionParticipantInfo {
    pub client_id: String,
    pub user_id: String,
    pub role: SessionParticipantRole,
    /// Character name if the participant has selected a character
    pub character_name: Option<String>,
}

/// World snapshot for session initialization
///
/// This is an opaque type from the application's perspective - the concrete
/// snapshot format is defined by infrastructure.
pub type SessionWorldData = serde_json::Value;

/// Error types for async session operations
#[derive(Debug, thiserror::Error)]
pub enum AsyncSessionError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("World not found: {0}")]
    WorldNotFound(String),

    #[error("Client not in any session")]
    ClientNotInSession,

    #[error("Not authorized for this operation")]
    NotAuthorized,

    #[error("Session already has a DM")]
    DmAlreadyPresent,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Message sender type for sending messages to clients
///
/// The actual message type is opaque to the application layer - messages
/// are serialized/deserialized at the infrastructure boundary.
pub type MessageSender = mpsc::UnboundedSender<serde_json::Value>;

/// Result of joining a session
#[derive(Debug, Clone)]
pub struct SessionJoinInfo {
    pub session_id: SessionId,
    pub world_snapshot_json: serde_json::Value,
}

/// Async port for session management operations
///
/// This trait provides async-aware session management that application services
/// can depend on without coupling to infrastructure implementation details.
#[async_trait]
pub trait AsyncSessionPort: Send + Sync {
    /// Get the session ID for a client
    async fn get_client_session(&self, client_id: &str) -> Option<SessionId>;

    /// Check if a client is the DM for their session
    async fn is_client_dm(&self, client_id: &str) -> bool;

    /// Get the user ID for a client
    async fn get_client_user_id(&self, client_id: &str) -> Option<String>;

    /// Get participant info for a client
    async fn get_participant_info(&self, client_id: &str) -> Option<SessionParticipantInfo>;

    /// Get a session's world ID
    async fn get_session_world_id(&self, session_id: SessionId) -> Option<WorldId>;

    /// Find an existing session for a world
    async fn find_session_for_world(&self, world_id: WorldId) -> Option<SessionId>;

    /// Create a new session for a world
    async fn create_session(
        &self,
        world_id: WorldId,
        world_snapshot: SessionWorldData,
    ) -> SessionId;

    /// Create a new session with a specific ID
    async fn create_session_with_id(
        &self,
        session_id: SessionId,
        world_id: WorldId,
        world_snapshot: SessionWorldData,
    ) -> SessionId;

    /// Join an existing session
    async fn join_session(
        &self,
        session_id: SessionId,
        client_id: &str,
        user_id: String,
        role: SessionParticipantRole,
    ) -> Result<SessionJoinInfo, AsyncSessionError>;

    /// Broadcast a JSON message to all participants in a session
    async fn broadcast_to_session(
        &self,
        session_id: SessionId,
        message: serde_json::Value,
    ) -> Result<(), AsyncSessionError>;

    /// Broadcast a JSON message to all players (not DM) in a session
    async fn broadcast_to_players(
        &self,
        session_id: SessionId,
        message: serde_json::Value,
    ) -> Result<(), AsyncSessionError>;

    /// Send a message to the DM of a session
    async fn send_to_dm(
        &self,
        session_id: SessionId,
        message: serde_json::Value,
    ) -> Result<(), AsyncSessionError>;

    /// Broadcast to all except a specific client
    async fn broadcast_except(
        &self,
        session_id: SessionId,
        message: serde_json::Value,
        exclude_client: &str,
    ) -> Result<(), AsyncSessionError>;

    /// Get all participants in a session
    async fn get_session_participants(
        &self,
        session_id: SessionId,
    ) -> Vec<SessionParticipantInfo>;

    /// Add to conversation history
    async fn add_to_conversation_history(
        &self,
        session_id: SessionId,
        speaker: &str,
        text: &str,
    ) -> Result<(), AsyncSessionError>;

    /// Check if session has a DM
    async fn session_has_dm(&self, session_id: SessionId) -> bool;

    /// Get the world snapshot JSON for a session
    async fn get_session_snapshot(&self, session_id: SessionId) -> Option<serde_json::Value>;

    /// Get all active session IDs
    async fn list_session_ids(&self) -> Vec<SessionId>;

    // === New methods for WebSocket handler refactoring ===

    /// Remove a client from their session (cleanup on disconnect)
    /// Returns the session ID and participant info if the client was in a session
    async fn client_leave_session(&self, client_id: &str) -> Option<(SessionId, SessionParticipantInfo)>;

    /// Update the current scene ID for a session
    async fn update_session_scene(&self, session_id: SessionId, scene_id: String) -> Result<(), AsyncSessionError>;

    /// Send a message to a specific participant by user_id
    async fn send_to_participant(
        &self,
        session_id: SessionId,
        user_id: &str,
        message: serde_json::Value,
    ) -> Result<(), AsyncSessionError>;

    /// Get DM info for a session
    async fn get_session_dm(&self, session_id: SessionId) -> Option<SessionParticipantInfo>;

    // === New methods for queue worker refactoring ===

    /// Register a pending approval if not already registered
    /// Returns true if the approval was newly registered, false if it already existed
    async fn register_pending_approval(
        &self,
        session_id: SessionId,
        approval_id: String,
        npc_name: String,
        proposed_dialogue: String,
        internal_reasoning: Option<String>,
        proposed_tools: Vec<crate::domain::value_objects::ProposedToolInfo>,
    ) -> Result<bool, AsyncSessionError>;
}
