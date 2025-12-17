//! Session management for active game sessions
//!
//! This module provides session tracking for WebSocket connections,
//! allowing multiple clients to join a shared game session and
//! receive synchronized updates. It also maintains conversation history
//! for LLM context.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::application::ports::outbound::{
    BroadcastMessage, CharacterContextInfo, PendingApprovalInfo,
    SessionManagementError, SessionManagementPort, SessionWorldContext,
};
use crate::domain::entities::{Character, Location, Scene, World};
use crate::domain::value_objects::{ProposedToolInfo, SessionId, WorldId};
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

/// Represents a single turn in the conversation history
///
/// Each turn tracks a message exchange between a player/NPC and captures
/// the speaker identity, content, and timestamp for LLM context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Name of the speaker (character name or "Player")
    pub speaker: String,
    /// Content of the dialogue or action
    pub content: String,
    /// Timestamp when this turn occurred
    pub timestamp: DateTime<Utc>,
    /// Whether this was a player action (true) or NPC response (false)
    pub is_player: bool,
}

impl ConversationTurn {
    /// Create a new conversation turn
    pub fn new(speaker: String, content: String, is_player: bool) -> Self {
        Self {
            speaker,
            content,
            timestamp: Utc::now(),
            is_player,
        }
    }
}

/// A participant in a game session
#[derive(Debug, Clone)]
pub struct SessionParticipant {
    pub client_id: ClientId,
    pub user_id: String,
    pub role: ParticipantRole,
    #[allow(dead_code)] // Kept for future session analytics and participant tracking
    pub joined_at: DateTime<Utc>,
    /// Channel to send messages to this client
    pub sender: mpsc::UnboundedSender<ServerMessage>,
}

/// Tracks a pending approval request from the LLM
///
/// This structure maintains all information needed to process the DM's approval decision.
#[derive(Debug, Clone)]
pub struct PendingApproval {
    /// Request ID matching the ApprovalRequired message
    pub request_id: String,
    /// Name of the NPC responding
    pub npc_name: String,
    /// Original proposed dialogue from LLM
    pub proposed_dialogue: String,
    /// Internal reasoning from LLM
    pub internal_reasoning: String,
    /// Proposed tool calls
    pub proposed_tools: Vec<ProposedToolInfo>,
    /// Number of rejection retries already used
    pub retry_count: u32,
    /// Timestamp when approval was requested
    #[allow(dead_code)] // Kept for future approval timeout/expiry features
    pub requested_at: DateTime<Utc>,
}

impl PendingApproval {
    pub fn new(
        request_id: String,
        npc_name: String,
        proposed_dialogue: String,
        internal_reasoning: String,
        proposed_tools: Vec<ProposedToolInfo>,
    ) -> Self {
        Self {
            request_id,
            npc_name,
            proposed_dialogue,
            internal_reasoning,
            proposed_tools,
            retry_count: 0,
            requested_at: Utc::now(),
        }
    }
}

/// A snapshot of the current world state for session joining
///
/// Note: This struct does not derive Serialize/Deserialize because it contains
/// domain types. Use `to_json()` method for serialization instead.
#[derive(Debug, Clone)]
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

impl Default for WorldSnapshot {
    /// Create a minimal empty world snapshot with placeholder values.
    ///
    /// This is used as a fallback when JSON deserialization fails during
    /// session creation. In normal operation, proper world data should be
    /// provided, but this ensures the system remains functional with a
    /// basic empty world containing no locations, characters, or scenes.
    fn default() -> Self {
        Self {
            world: World::new("Empty World", "A placeholder world"),
            locations: Vec::new(),
            characters: Vec::new(),
            scenes: Vec::new(),
            current_scene_id: None,
        }
    }
}

/// An active game session
#[derive(Debug)]
pub struct GameSession {
    pub id: SessionId,
    pub world_id: WorldId,
    pub world_snapshot: Arc<WorldSnapshot>,
    pub participants: HashMap<ClientId, SessionParticipant>,
    /// User ID of the DM who owns this session (if known)
    pub dm_user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub current_scene_id: Option<String>,
    /// Conversation history for LLM context
    conversation_history: Vec<ConversationTurn>,
    /// Maximum number of conversation turns to keep in history
    max_history_length: usize,
    /// Pending approval requests awaiting DM decision
    pending_approvals: HashMap<String, PendingApproval>,
    /// Map of user_id -> PlayerCharacter for this session
    pub player_characters: HashMap<String, crate::domain::entities::PlayerCharacter>,
}

impl GameSession {
    /// Create a new game session for a world with a generated session ID
    pub fn new(world_id: WorldId, world_snapshot: WorldSnapshot, max_history_length: usize) -> Self {
        Self::new_with_id(SessionId::new(), world_id, world_snapshot, max_history_length)
    }

    /// Create a new game session for a world with an explicit session ID.
    pub fn new_with_id(
        session_id: SessionId,
        world_id: WorldId,
        world_snapshot: WorldSnapshot,
        max_history_length: usize,
    ) -> Self {
        Self {
            id: session_id,
            world_id,
            world_snapshot: Arc::new(world_snapshot),
            participants: HashMap::new(),
            dm_user_id: None,
            created_at: Utc::now(),
            current_scene_id: None,
            conversation_history: Vec::new(),
            max_history_length,
            pending_approvals: HashMap::new(),
            player_characters: HashMap::new(),
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

    /// Add a player action to the conversation history
    ///
    /// # Arguments
    /// * `character_name` - Name of the character performing the action
    /// * `action` - Description of the action or dialogue
    pub fn add_player_action(&mut self, character_name: &str, action: &str) {
        let turn = ConversationTurn::new(
            character_name.to_string(),
            action.to_string(),
            true,
        );
        self.add_turn(turn);
    }

    /// Add an NPC response to the conversation history
    ///
    /// # Arguments
    /// * `npc_name` - Name of the NPC speaking
    /// * `dialogue` - The NPC's dialogue or response
    pub fn add_npc_response(&mut self, npc_name: &str, dialogue: &str) {
        let turn = ConversationTurn::new(
            npc_name.to_string(),
            dialogue.to_string(),
            false,
        );
        self.add_turn(turn);
    }

    /// Internal method to add a turn and maintain history length limit
    fn add_turn(&mut self, turn: ConversationTurn) {
        self.conversation_history.push(turn);
        // Remove oldest turns if we exceed the maximum
        if self.conversation_history.len() > self.max_history_length {
            let excess = self.conversation_history.len() - self.max_history_length;
            self.conversation_history.drain(0..excess);
        }
    }

    /// Get the recent conversation history
    ///
    /// Returns a slice of the most recent conversation turns.
    /// If `max_turns` is 0, returns the entire history.
    ///
    /// # Arguments
    /// * `max_turns` - Maximum number of recent turns to return (0 = all)
    ///
    /// # Returns
    /// Slice of conversation turns
    pub fn get_recent_history(&self, max_turns: usize) -> &[ConversationTurn] {
        if max_turns == 0 || self.conversation_history.len() <= max_turns {
            &self.conversation_history
        } else {
            let start = self.conversation_history.len() - max_turns;
            &self.conversation_history[start..]
        }
    }

    /// Add a player character to the session
    pub fn add_player_character(
        &mut self,
        pc: crate::domain::entities::PlayerCharacter,
    ) -> Result<(), String> {
        // Validate that the PC belongs to this session
        if pc.session_id != self.id {
            return Err("Player character session_id does not match session".to_string());
        }
        self.player_characters.insert(pc.user_id.clone(), pc);
        Ok(())
    }

    /// Get a player character by user ID
    pub fn get_player_character(
        &self,
        user_id: &str,
    ) -> Option<&crate::domain::entities::PlayerCharacter> {
        self.player_characters.get(user_id)
    }

    /// Get all player characters in the session
    pub fn get_all_pcs(&self) -> Vec<&crate::domain::entities::PlayerCharacter> {
        self.player_characters.values().collect()
    }

    /// Update a player character's location
    pub fn update_pc_location(
        &mut self,
        user_id: &str,
        location_id: crate::domain::value_objects::LocationId,
    ) -> Result<(), String> {
        if let Some(pc) = self.player_characters.get_mut(user_id) {
            pc.update_location(location_id);
            Ok(())
        } else {
            Err(format!("Player character not found for user_id: {}", user_id))
        }
    }

    /// Get the entire conversation history
    pub fn get_full_history(&self) -> &[ConversationTurn] {
        &self.conversation_history
    }

    /// Clear all conversation history
    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    /// Set the maximum history length
    ///
    /// When set, the history will be trimmed if it exceeds this length.
    ///
    /// # Arguments
    /// * `max_length` - New maximum length (must be > 0)
    pub fn set_max_history_length(&mut self, max_length: usize) {
        assert!(max_length > 0, "max_history_length must be greater than 0");
        self.max_history_length = max_length;
        // Trim history if it now exceeds the new maximum
        if self.conversation_history.len() > max_length {
            let excess = self.conversation_history.len() - max_length;
            self.conversation_history.drain(0..excess);
        }
    }

    /// Get the current number of turns in history
    pub fn history_length(&self) -> usize {
        self.conversation_history.len()
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

    /// Send a message only to the DM(s)
    /// If multiple DMs exist with the same user_id (multiple tabs), send to all of them
    pub fn send_to_dm(&self, message: &ServerMessage) {
        // Send to all DMs with the same user_id as the session's dm_user_id
        // This allows multiple DM tabs/windows to receive messages
        let target_user_id = self.dm_user_id.as_ref();
        
        for participant in self.participants.values() {
            if participant.role == ParticipantRole::DungeonMaster {
                // If we have a dm_user_id set, only send to DMs with that user_id
                // Otherwise, send to any DM (backward compatibility)
                if let Some(target_id) = target_user_id {
                    if participant.user_id == *target_id {
                        if let Err(e) = participant.sender.send(message.clone()) {
                            tracing::warn!("Failed to send message to DM {}: {}", participant.client_id, e);
                        }
                    }
                } else {
                    // No dm_user_id set yet, send to any DM (first one found)
                    if let Err(e) = participant.sender.send(message.clone()) {
                        tracing::warn!("Failed to send message to DM {}: {}", participant.client_id, e);
                    }
                    // Only send to first DM if no dm_user_id is set (backward compatibility)
                    break;
                }
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
    #[allow(dead_code)] // Kept for future session stats/UI features
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Check if the session is empty
    pub fn is_empty(&self) -> bool {
        self.participants.is_empty()
    }

    /// Store a pending approval request
    pub fn add_pending_approval(&mut self, approval: PendingApproval) {
        self.pending_approvals
            .insert(approval.request_id.clone(), approval);
    }

    /// Retrieve a pending approval request by ID
    pub fn get_pending_approval(&self, request_id: &str) -> Option<&PendingApproval> {
        self.pending_approvals.get(request_id)
    }

    /// Get a mutable pending approval request
    pub fn get_pending_approval_mut(&mut self, request_id: &str) -> Option<&mut PendingApproval> {
        self.pending_approvals.get_mut(request_id)
    }

    /// Remove a pending approval request (after it's been processed)
    pub fn remove_pending_approval(&mut self, request_id: &str) -> Option<PendingApproval> {
        self.pending_approvals.remove(request_id)
    }

    /// Send a message to a specific participant by user ID
    pub fn send_to_participant(&self, user_id: &str, message: &ServerMessage) {
        for participant in self.participants.values() {
            if participant.user_id == user_id {
                if let Err(e) = participant.sender.send(message.clone()) {
                    tracing::warn!(
                        "Failed to send message to participant {}: {}",
                        participant.client_id,
                        e
                    );
                }
            }
        }
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
    #[allow(dead_code)] // Kept for comprehensive error handling
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
        let pending = session.pending_approvals.get(request_id)?;
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
        let mut characters = std::collections::HashMap::new();
        for character in &snapshot.characters {
            characters.insert(
                character.name.clone(),
                CharacterContextInfo {
                    name: character.name.clone(),
                    archetype: format!("{:?}", character.current_archetype),
                    wants: character
                        .wants
                        .iter()
                        .map(|w| format!("{:?}", w))
                        .collect(),
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

    #[test]
    fn test_add_player_action() {
        let mut session = GameSession::new(
            WorldId::new(),
            create_test_snapshot(create_test_world()),
            30,
        );

        session.add_player_action("Alice", "I try to negotiate with the merchant");

        assert_eq!(session.history_length(), 1);
        let history = session.get_full_history();
        assert_eq!(history[0].speaker, "Alice");
        assert_eq!(history[0].content, "I try to negotiate with the merchant");
        assert!(history[0].is_player);
    }

    #[test]
    fn test_add_npc_response() {
        let mut session = GameSession::new(
            WorldId::new(),
            create_test_snapshot(create_test_world()),
            30,
        );

        session.add_npc_response("Merchant", "That will cost you 50 gold pieces");

        assert_eq!(session.history_length(), 1);
        let history = session.get_full_history();
        assert_eq!(history[0].speaker, "Merchant");
        assert_eq!(history[0].content, "That will cost you 50 gold pieces");
        assert!(!history[0].is_player);
    }

    #[test]
    fn test_conversation_history_sequence() {
        let mut session = GameSession::new(
            WorldId::new(),
            create_test_snapshot(create_test_world()),
            30,
        );

        session.add_player_action("Bob", "I cast fireball");
        session.add_npc_response("Guard", "That's not happening");
        session.add_player_action("Bob", "I try running away");
        session.add_npc_response("Guard", "You cannot escape!");

        assert_eq!(session.history_length(), 4);

        let history = session.get_full_history();
        assert_eq!(history[0].speaker, "Bob");
        assert_eq!(history[1].speaker, "Guard");
        assert_eq!(history[2].speaker, "Bob");
        assert_eq!(history[3].speaker, "Guard");
    }

    #[test]
    fn test_history_length_limit() {
        let mut session = GameSession::new(
            WorldId::new(),
            create_test_snapshot(create_test_world()),
            30,
        );

        // Set a small limit for testing
        session.set_max_history_length(5);

        // Add 10 turns
        for i in 1..=10 {
            session.add_player_action("Player", &format!("Action {}", i));
        }

        // Should only have 5 turns
        assert_eq!(session.history_length(), 5);

        // Check that we have the last 5 turns
        let history = session.get_full_history();
        assert_eq!(history[0].content, "Action 6");
        assert_eq!(history[4].content, "Action 10");
    }

    #[test]
    fn test_get_recent_history() {
        let mut session = GameSession::new(
            WorldId::new(),
            create_test_snapshot(create_test_world()),
            30,
        );

        // Add 5 turns
        for i in 1..=5 {
            session.add_player_action("Player", &format!("Action {}", i));
        }

        // Get last 3 turns
        let recent = session.get_recent_history(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].content, "Action 3");
        assert_eq!(recent[2].content, "Action 5");
    }

    #[test]
    fn test_get_recent_history_all() {
        let mut session = GameSession::new(
            WorldId::new(),
            create_test_snapshot(create_test_world()),
            30,
        );

        session.add_player_action("Player", "Action 1");
        session.add_player_action("Player", "Action 2");

        // Get all history with 0 (means all)
        let all = session.get_recent_history(0);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_clear_history() {
        let mut session = GameSession::new(
            WorldId::new(),
            create_test_snapshot(create_test_world()),
            30,
        );

        session.add_player_action("Player", "Action 1");
        session.add_npc_response("NPC", "Response 1");
        assert_eq!(session.history_length(), 2);

        session.clear_history();
        assert_eq!(session.history_length(), 0);
        assert!(session.get_full_history().is_empty());
    }

    #[test]
    fn test_set_max_history_length() {
        let mut session = GameSession::new(
            WorldId::new(),
            create_test_snapshot(create_test_world()),
            30,
        );

        // Add 10 turns with default limit (30)
        for i in 1..=10 {
            session.add_player_action("Player", &format!("Action {}", i));
        }
        assert_eq!(session.history_length(), 10);

        // Change limit to 5
        session.set_max_history_length(5);

        // Should trim excess
        assert_eq!(session.history_length(), 5);

        // Verify we have the last 5
        let history = session.get_full_history();
        assert_eq!(history[0].content, "Action 6");
        assert_eq!(history[4].content, "Action 10");
    }

    #[test]
    fn test_conversation_turn_creation() {
        let turn = ConversationTurn::new(
            "Alice".to_string(),
            "Hello, world!".to_string(),
            true,
        );

        assert_eq!(turn.speaker, "Alice");
        assert_eq!(turn.content, "Hello, world!");
        assert!(turn.is_player);
        // Timestamp should be very recent
        let elapsed = Utc::now().signed_duration_since(turn.timestamp);
        assert!(elapsed.num_seconds() < 1);
    }
}
