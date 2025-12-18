//! GameSession and related types

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use crate::application::dto::WorldSnapshot;
use crate::domain::value_objects::{GameTime, ProposedToolInfo, SessionId, TimeOfDay, WorldId};
use crate::infrastructure::websocket::{ParticipantRole, ServerMessage};

use super::conversation::ConversationTurn;
use super::ClientId;

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
    /// In-game time tracking (Phase 23C)
    game_time: GameTime,
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
            game_time: GameTime::new(),
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
        if pc.session_id != Some(self.id) {
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

    /// Get the character name for a user (if they have a selected character)
    pub fn get_character_name_for_user(&self, user_id: &str) -> Option<String> {
        self.player_characters.get(user_id).map(|pc| pc.name.clone())
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

    // =========================================================================
    // Game Time (Phase 23C)
    // =========================================================================

    /// Get a reference to the game time
    pub fn game_time(&self) -> &GameTime {
        &self.game_time
    }

    /// Get a mutable reference to the game time
    pub fn game_time_mut(&mut self) -> &mut GameTime {
        &mut self.game_time
    }

    /// Advance game time by hours (convenience method)
    pub fn advance_time_hours(&mut self, hours: u32) {
        self.game_time.advance_hours(hours);
    }

    /// Advance game time by days (convenience method)
    pub fn advance_time_days(&mut self, days: u32) {
        self.game_time.advance_days(days);
    }

    /// Get the current time of day
    pub fn time_of_day(&self) -> TimeOfDay {
        self.game_time.time_of_day()
    }

    /// Get a human-readable game time display
    pub fn display_game_time(&self) -> String {
        self.game_time.display_date()
    }

    /// Check if game time is paused
    pub fn is_time_paused(&self) -> bool {
        self.game_time.is_paused()
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
}
