//! Session management port - Interface for game session management operations
//!
//! This port abstracts session management operations used by application services,
//! allowing the infrastructure to provide the concrete implementation while
//! maintaining hexagonal architecture boundaries.

use crate::domain::value_objects::{ProposedToolInfo, SessionId, WorldId};
use std::collections::HashMap;

use super::SessionParticipantRole;

/// Information about a pending approval request
#[derive(Debug, Clone)]
pub struct PendingApprovalInfo {
    /// Unique ID for this approval request
    pub request_id: String,
    /// Name of the NPC whose response is pending approval
    pub npc_name: String,
    /// The proposed dialogue from the LLM
    pub proposed_dialogue: String,
    /// Internal reasoning from the LLM
    pub internal_reasoning: String,
    /// Proposed tool calls
    pub proposed_tools: Vec<ProposedToolInfo>,
    /// Number of times this has been rejected and retried
    pub retry_count: u32,
}

/// Result of joining a session
#[derive(Debug, Clone)]
pub struct SessionJoinResult {
    /// The session that was joined
    pub session_id: SessionId,
    /// List of other participants in the session
    pub participants: Vec<ParticipantSummary>,
    /// World snapshot as JSON for the client
    pub world_snapshot_json: serde_json::Value,
}

/// Summary information about a participant
#[derive(Debug, Clone)]
pub struct ParticipantSummary {
    /// User ID of the participant
    pub user_id: String,
    /// Role in the session
    pub role: SessionParticipantRole,
    /// Selected character name if any
    pub character_name: Option<String>,
}

/// A message to be broadcast to session participants
#[derive(Debug, Clone)]
pub struct BroadcastMessage {
    /// JSON-serializable message content
    pub content: serde_json::Value,
}

/// Errors that can occur during session management
#[derive(Debug, thiserror::Error)]
pub enum SessionManagementError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("World not found: {0}")]
    WorldNotFound(String),

    #[error("Not authorized for this operation")]
    NotAuthorized,

    #[error("Client not in any session")]
    ClientNotInSession,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Session is full")]
    SessionFull,

    #[error("Approval not found: {0}")]
    ApprovalNotFound(String),
}

/// Port for session management operations
///
/// This trait defines the interface for session management operations needed by
/// the application layer. The infrastructure layer (SessionManager) provides
/// the concrete implementation.
///
/// # Design Notes
///
/// - Operations are sync (session manager uses in-memory storage)
/// - Client identification uses string IDs to decouple from infrastructure format
/// - Messages are serialized to JSON at the boundary
pub trait SessionManagementPort: Send + Sync {
    /// Get the session ID for a client, if they're in one
    fn get_client_session(&self, client_id: &str) -> Option<SessionId>;

    /// Check if a client is the DM for their session
    fn is_client_dm(&self, client_id: &str) -> bool;

    /// Get the user ID for a client
    fn get_client_user_id(&self, client_id: &str) -> Option<String>;

    /// Get a pending approval by request ID
    fn get_pending_approval(
        &self,
        session_id: SessionId,
        request_id: &str,
    ) -> Option<PendingApprovalInfo>;

    /// Add a pending approval to a session
    fn add_pending_approval(
        &mut self,
        session_id: SessionId,
        approval: PendingApprovalInfo,
    ) -> Result<(), SessionManagementError>;

    /// Remove a pending approval from a session
    fn remove_pending_approval(
        &mut self,
        session_id: SessionId,
        request_id: &str,
    ) -> Result<(), SessionManagementError>;

    /// Increment the retry count for a pending approval
    fn increment_retry_count(
        &mut self,
        session_id: SessionId,
        request_id: &str,
    ) -> Result<u32, SessionManagementError>;

    /// Broadcast a message to all players in a session (not DM)
    fn broadcast_to_players(
        &self,
        session_id: SessionId,
        message: &BroadcastMessage,
    ) -> Result<(), SessionManagementError>;

    /// Send a message to the DM of a session
    fn send_to_dm(
        &self,
        session_id: SessionId,
        message: &BroadcastMessage,
    ) -> Result<(), SessionManagementError>;

    /// Broadcast a message to all participants except one
    fn broadcast_except(
        &self,
        session_id: SessionId,
        message: &BroadcastMessage,
        exclude_client: &str,
    ) -> Result<(), SessionManagementError>;

    /// Broadcast a message to all participants in a session (players and DM)
    fn broadcast_to_session(
        &self,
        session_id: SessionId,
        message: &BroadcastMessage,
    ) -> Result<(), SessionManagementError>;

    /// Add an NPC response to the session's conversation history
    fn add_to_conversation_history(
        &mut self,
        session_id: SessionId,
        speaker: &str,
        text: &str,
    ) -> Result<(), SessionManagementError>;

    /// Check if a session has a DM
    fn session_has_dm(&self, session_id: SessionId) -> bool;

    /// Get world data from a session for building LLM context
    fn get_session_world_context(
        &self,
        session_id: SessionId,
    ) -> Option<SessionWorldContext>;
}

/// World context data for building LLM prompts
#[derive(Debug, Clone)]
pub struct SessionWorldContext {
    /// Current scene name
    pub scene_name: String,
    /// Location name
    pub location_name: String,
    /// Time context
    pub time_context: String,
    /// Names of characters present in the scene
    pub present_character_names: Vec<String>,
    /// Character contexts for building prompts
    pub characters: HashMap<String, CharacterContextInfo>,
    /// Directorial notes for the current scene
    pub directorial_notes: String,
}

/// Character information for LLM context
#[derive(Debug, Clone)]
pub struct CharacterContextInfo {
    /// Character name
    pub name: String,
    /// Character archetype
    pub archetype: String,
    /// Character wants/motivations
    pub wants: Vec<String>,
}

/// Port for session lifecycle management (join/create/leave)
///
/// This is separate from SessionManagementPort to allow different
/// implementations or to restrict access to lifecycle operations.
pub trait SessionLifecyclePort: Send + Sync {
    /// Join an existing session or create a new one for a world
    fn join_or_create_session(
        &mut self,
        client_id: u64,
        user_id: String,
        role: SessionParticipantRole,
        world_id: Option<WorldId>,
        world_snapshot_json: Option<serde_json::Value>,
    ) -> Result<SessionJoinResult, SessionManagementError>;

    /// Leave the current session
    fn leave_session(&mut self, client_id: u64) -> Option<(SessionId, String)>;

    /// Find a session for a given world
    fn find_session_for_world(&self, world_id: WorldId) -> Option<SessionId>;
}
