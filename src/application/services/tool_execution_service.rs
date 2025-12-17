//! Tool Execution Service - Executes approved tool calls to modify game state
//!
//! This service handles the execution of game tools that have been approved by the DM.
//! It modifies in-memory session state without persisting to the database, allowing
//! for future expansion with more complex effects.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::SessionManagementPort;
use crate::domain::value_objects::{
    ChangeAmount, GameTool, InfoImportance, RelationshipChange, SessionId,
};

/// Result of executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// Whether the tool executed successfully
    pub success: bool,
    /// Human-readable description of what happened
    pub description: String,
    /// List of state changes that occurred (for broadcasting)
    pub state_changes: Vec<StateChange>,
}

/// Individual state changes caused by tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StateChange {
    /// An item was added to a character's inventory
    ItemAdded {
        character: String,
        item: String,
    },
    /// Information was revealed to the player
    InfoRevealed {
        info: String,
    },
    /// A relationship sentiment was changed
    RelationshipChanged {
        from: String,
        to: String,
        delta: i32,
    },
    /// An event was triggered
    EventTriggered {
        name: String,
    },
    /// An NPC's motivation was modified
    NpcMotivationChanged {
        npc_id: String,
        motivation_type: String,
        new_value: String,
        reason: String,
    },
    /// A character's description was updated
    CharacterDescriptionUpdated {
        character_id: String,
        change_type: String,
        description: String,
    },
    /// An NPC's opinion of a PC changed
    NpcOpinionChanged {
        npc_id: String,
        target_pc_id: String,
        opinion_change: String,
        reason: String,
    },
    /// An item was transferred between characters
    ItemTransferred {
        from_id: String,
        to_id: String,
        item_name: String,
    },
    /// A condition was added to a character
    ConditionAdded {
        character_id: String,
        condition_name: String,
        description: String,
        duration: Option<String>,
    },
    /// A condition was removed from a character
    ConditionRemoved {
        character_id: String,
        condition_name: String,
    },
    /// A character's stat was updated
    CharacterStatUpdated {
        character_id: String,
        stat_name: String,
        delta: i32,
    },
}

/// Errors that can occur during tool execution
#[derive(Debug, thiserror::Error)]
pub enum ToolExecutionError {
    /// Target character was not found in the session
    #[error("Character not found: {0}")]
    CharacterNotFound(String),

    /// Invalid tool parameters
    #[error("Invalid tool parameters: {0}")]
    InvalidParameters(String),

    /// Internal error during execution
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

/// Service for executing approved game tools
pub struct ToolExecutionService;

impl ToolExecutionService {
    /// Create a new tool execution service
    pub fn new() -> Self {
        Self
    }

    /// Execute an approved tool call and modify session state
    ///
    /// # Arguments
    ///
    /// * `tool` - The game tool to execute
    /// * `session` - The game session (will be modified in-place)
    ///
    /// # Returns
    ///
    /// A `ToolExecutionResult` describing what happened, or a `ToolExecutionError` if execution failed
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use wrldbldr_engine::domain::GameTool;
    /// use wrldbldr_engine::application::services::ToolExecutionService;
    ///
    /// let service = ToolExecutionService::new();
    /// let tool = GameTool::GiveItem {
    ///     item_name: "Mysterious Key".to_string(),
    ///     description: "An ornate bronze key".to_string(),
    /// };
    ///
    /// let result = service.execute_tool(&tool, &mut session).await?;
    /// assert!(result.success);
    /// ```
    #[instrument(skip(self, session))]
    pub async fn execute_tool<S: SessionManagementPort>(
        &self,
        tool: &GameTool,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        match tool {
            GameTool::GiveItem { item_name, description } => {
                self.execute_give_item(item_name, description, session, session_id).await
            }
            GameTool::RevealInfo {
                info_type,
                content,
                importance,
            } => {
                self.execute_reveal_info(info_type, content, importance, session, session_id)
                    .await
            }
            GameTool::ChangeRelationship { change, amount, reason } => {
                self.execute_change_relationship(change, amount, reason, session, session_id)
                    .await
            }
            GameTool::TriggerEvent {
                event_type,
                description,
            } => {
                self.execute_trigger_event(event_type, description, session, session_id)
                    .await
            }
            GameTool::ModifyNpcMotivation {
                npc_id,
                motivation_type,
                new_value,
                reason,
            } => {
                self.execute_modify_npc_motivation(npc_id, motivation_type, new_value, reason, session, session_id)
                    .await
            }
            GameTool::ModifyCharacterDescription {
                character_id,
                change_type,
                description,
            } => {
                self.execute_modify_character_description(character_id, change_type, description, session, session_id)
                    .await
            }
            GameTool::ModifyNpcOpinion {
                npc_id,
                target_pc_id,
                opinion_change,
                reason,
            } => {
                self.execute_modify_npc_opinion(npc_id, target_pc_id, opinion_change, reason, session, session_id)
                    .await
            }
            GameTool::TransferItem {
                from_id,
                to_id,
                item_name,
            } => {
                self.execute_transfer_item(from_id, to_id, item_name, session, session_id)
                    .await
            }
            GameTool::AddCondition {
                character_id,
                condition_name,
                description,
                duration,
            } => {
                self.execute_add_condition(character_id, condition_name, description, duration.as_deref(), session, session_id)
                    .await
            }
            GameTool::RemoveCondition {
                character_id,
                condition_name,
            } => {
                self.execute_remove_condition(character_id, condition_name, session, session_id)
                    .await
            }
            GameTool::UpdateCharacterStat {
                character_id,
                stat_name,
                delta,
            } => {
                self.execute_update_character_stat(character_id, stat_name, *delta, session, session_id)
                    .await
            }
        }
    }

    /// Execute GiveItem tool - adds item to character inventory
    #[instrument(skip(self, session))]
    async fn execute_give_item<S: SessionManagementPort>(
        &self,
        item_name: &str,
        description: &str,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        // Get the player character from the session
        // For now, we log the item transfer without modifying inventory
        // (the session doesn't have item IDs yet - would be added in a full implementation)

        let description_msg = format!(
            "Gave '{}' to player: {}",
            item_name, description
        );

        debug!("Item transfer: {}", description_msg);

        // Log the action in conversation history
        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("Item received: {} - {}", item_name, description),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::ItemAdded {
            character: "Player".to_string(),
            item: item_name.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute RevealInfo tool - marks information as known to player
    #[instrument(skip(self, session))]
    async fn execute_reveal_info<S: SessionManagementPort>(
        &self,
        info_type: &str,
        content: &str,
        importance: &InfoImportance,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "Revealed {} {} information",
            importance.as_str(),
            info_type
        );

        debug!("Info revealed: {} - {}", info_type, content);

        // Log the revelation in conversation history
        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[{}] {} - {}", info_type, importance.as_str(), content),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::InfoRevealed {
            info: format!("[{}] {}", info_type, content),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute ChangeRelationship tool - updates relationship sentiment
    #[instrument(skip(self, session))]
    async fn execute_change_relationship<S: SessionManagementPort>(
        &self,
        change: &RelationshipChange,
        amount: &ChangeAmount,
        reason: &str,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        // Calculate sentiment delta based on amount
        let delta = match amount {
            ChangeAmount::Slight => 10,
            ChangeAmount::Moderate => 25,
            ChangeAmount::Significant => 50,
        };

        // Apply sign based on improvement/worsening
        let signed_delta = match change {
            RelationshipChange::Improve => delta,
            RelationshipChange::Worsen => -delta,
        };

        let change_str = match change {
            RelationshipChange::Improve => "Improve",
            RelationshipChange::Worsen => "Worsen",
        };

        let description_msg = format!(
            "{} relationship {} with player (reason: {})",
            change_str,
            amount.as_str(),
            reason
        );

        debug!(
            "Relationship change: {} (delta: {})",
            description_msg, signed_delta
        );

        // Log the relationship change in conversation history
        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!(
                    "Relationship {}: {} ({})",
                    change.as_str(),
                    amount.as_str(),
                    reason
                ),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::RelationshipChanged {
            from: "NPC".to_string(),
            to: "Player".to_string(),
            delta: signed_delta,
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute TriggerEvent tool - logs and triggers an event
    #[instrument(skip(self, session))]
    async fn execute_trigger_event<S: SessionManagementPort>(
        &self,
        event_type: &str,
        description: &str,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!("Triggered {} event: {}", event_type, description);

        info!("Event triggered: {}", description_msg);

        // Log the event in conversation history
        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[EVENT: {}] {}", event_type, description),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::EventTriggered {
            name: format!("{}: {}", event_type, description),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute ModifyNpcMotivation tool - updates an NPC's motivation
    #[instrument(skip(self, session))]
    async fn execute_modify_npc_motivation<S: SessionManagementPort>(
        &self,
        npc_id: &str,
        motivation_type: &str,
        new_value: &str,
        reason: &str,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "NPC {} motivation '{}' changed to '{}' ({})",
            npc_id, motivation_type, new_value, reason
        );

        info!("NPC motivation changed: {}", description_msg);

        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[NPC MOTIVATION] {}: {} -> {} ({})", npc_id, motivation_type, new_value, reason),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::NpcMotivationChanged {
            npc_id: npc_id.to_string(),
            motivation_type: motivation_type.to_string(),
            new_value: new_value.to_string(),
            reason: reason.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute ModifyCharacterDescription tool - updates a character's description
    #[instrument(skip(self, session))]
    async fn execute_modify_character_description<S: SessionManagementPort>(
        &self,
        character_id: &str,
        change_type: &str,
        description: &str,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "Character {} {} updated: {}",
            character_id, change_type, description
        );

        info!("Character description updated: {}", description_msg);

        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[CHARACTER UPDATE] {}: {} - {}", character_id, change_type, description),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::CharacterDescriptionUpdated {
            character_id: character_id.to_string(),
            change_type: change_type.to_string(),
            description: description.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute ModifyNpcOpinion tool - changes an NPC's opinion of a PC
    #[instrument(skip(self, session))]
    async fn execute_modify_npc_opinion<S: SessionManagementPort>(
        &self,
        npc_id: &str,
        target_pc_id: &str,
        opinion_change: &str,
        reason: &str,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "NPC {} now {} toward PC {} ({})",
            npc_id, opinion_change, target_pc_id, reason
        );

        info!("NPC opinion changed: {}", description_msg);

        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[NPC OPINION] {} -> {}: {} ({})", npc_id, target_pc_id, opinion_change, reason),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::NpcOpinionChanged {
            npc_id: npc_id.to_string(),
            target_pc_id: target_pc_id.to_string(),
            opinion_change: opinion_change.to_string(),
            reason: reason.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute TransferItem tool - transfers an item between characters
    #[instrument(skip(self, session))]
    async fn execute_transfer_item<S: SessionManagementPort>(
        &self,
        from_id: &str,
        to_id: &str,
        item_name: &str,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "'{}' transferred from {} to {}",
            item_name, from_id, to_id
        );

        info!("Item transferred: {}", description_msg);

        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[ITEM TRANSFER] '{}': {} -> {}", item_name, from_id, to_id),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::ItemTransferred {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            item_name: item_name.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute AddCondition tool - adds a condition to a character
    #[instrument(skip(self, session))]
    async fn execute_add_condition<S: SessionManagementPort>(
        &self,
        character_id: &str,
        condition_name: &str,
        description: &str,
        duration: Option<&str>,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let dur_str = duration.unwrap_or("permanent");
        let description_msg = format!(
            "Condition '{}' added to {} ({}): {}",
            condition_name, character_id, dur_str, description
        );

        info!("Condition added: {}", description_msg);

        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[CONDITION +] {} gained '{}' ({}) - {}", character_id, condition_name, dur_str, description),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::ConditionAdded {
            character_id: character_id.to_string(),
            condition_name: condition_name.to_string(),
            description: description.to_string(),
            duration: duration.map(|s| s.to_string()),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute RemoveCondition tool - removes a condition from a character
    #[instrument(skip(self, session))]
    async fn execute_remove_condition<S: SessionManagementPort>(
        &self,
        character_id: &str,
        condition_name: &str,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "Condition '{}' removed from {}",
            condition_name, character_id
        );

        info!("Condition removed: {}", description_msg);

        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[CONDITION -] {} lost '{}'", character_id, condition_name),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::ConditionRemoved {
            character_id: character_id.to_string(),
            condition_name: condition_name.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute UpdateCharacterStat tool - updates a character's stat
    #[instrument(skip(self, session))]
    async fn execute_update_character_stat<S: SessionManagementPort>(
        &self,
        character_id: &str,
        stat_name: &str,
        delta: i32,
        session: &mut S,
        session_id: SessionId,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let change_str = if delta >= 0 {
            format!("+{}", delta)
        } else {
            format!("{}", delta)
        };

        let description_msg = format!(
            "{} {} changed by {}",
            character_id, stat_name, change_str
        );

        info!("Character stat updated: {}", description_msg);

        session
            .add_to_conversation_history(
                session_id,
                "System",
                &format!("[STAT] {} {} {}", character_id, stat_name, change_str),
            )
            .map_err(|e| ToolExecutionError::ExecutionError(e.to_string()))?;

        let state_change = StateChange::CharacterStatUpdated {
            character_id: character_id.to_string(),
            stat_name: stat_name.to_string(),
            delta,
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }
}

impl Default for ToolExecutionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake implementation of SessionManagementPort for testing
    /// This avoids depending on infrastructure types in application layer tests
    struct FakeSessionManager {
        conversation_history: Vec<(String, String)>,
        session_id: SessionId,
    }

    impl FakeSessionManager {
        fn new() -> Self {
            Self {
                conversation_history: Vec::new(),
                session_id: SessionId::new(),
            }
        }
    }

    impl SessionManagementPort for FakeSessionManager {
        fn get_client_session(&self, _client_id: &str) -> Option<SessionId> {
            Some(self.session_id)
        }

        fn is_client_dm(&self, _client_id: &str) -> bool {
            false
        }

        fn get_client_user_id(&self, _client_id: &str) -> Option<String> {
            None
        }

        fn get_pending_approval(
            &self,
            _session_id: SessionId,
            _request_id: &str,
        ) -> Option<crate::application::ports::outbound::PendingApprovalInfo> {
            None
        }

        fn add_pending_approval(
            &mut self,
            _session_id: SessionId,
            _approval: crate::application::ports::outbound::PendingApprovalInfo,
        ) -> Result<(), crate::application::ports::outbound::SessionManagementError> {
            Ok(())
        }

        fn remove_pending_approval(
            &mut self,
            _session_id: SessionId,
            _request_id: &str,
        ) -> Result<(), crate::application::ports::outbound::SessionManagementError> {
            Ok(())
        }

        fn increment_retry_count(
            &mut self,
            _session_id: SessionId,
            _request_id: &str,
        ) -> Result<u32, crate::application::ports::outbound::SessionManagementError> {
            Ok(0)
        }

        fn broadcast_to_players(
            &self,
            _session_id: SessionId,
            _message: &crate::application::ports::outbound::BroadcastMessage,
        ) -> Result<(), crate::application::ports::outbound::SessionManagementError> {
            Ok(())
        }

        fn send_to_dm(
            &self,
            _session_id: SessionId,
            _message: &crate::application::ports::outbound::BroadcastMessage,
        ) -> Result<(), crate::application::ports::outbound::SessionManagementError> {
            Ok(())
        }

        fn broadcast_except(
            &self,
            _session_id: SessionId,
            _message: &crate::application::ports::outbound::BroadcastMessage,
            _exclude_client: &str,
        ) -> Result<(), crate::application::ports::outbound::SessionManagementError> {
            Ok(())
        }

        fn broadcast_to_session(
            &self,
            _session_id: SessionId,
            _message: &crate::application::ports::outbound::BroadcastMessage,
        ) -> Result<(), crate::application::ports::outbound::SessionManagementError> {
            Ok(())
        }

        fn add_to_conversation_history(
            &mut self,
            _session_id: SessionId,
            speaker: &str,
            text: &str,
        ) -> Result<(), crate::application::ports::outbound::SessionManagementError> {
            self.conversation_history.push((speaker.to_string(), text.to_string()));
            Ok(())
        }

        fn session_has_dm(&self, _session_id: SessionId) -> bool {
            false
        }

        fn get_session_world_context(
            &self,
            _session_id: SessionId,
        ) -> Option<crate::application::ports::outbound::SessionWorldContext> {
            None
        }
    }

    fn create_test_session() -> FakeSessionManager {
        FakeSessionManager::new()
    }

    #[tokio::test]
    async fn test_execute_give_item() {
        let service = ToolExecutionService::new();
        let tool = GameTool::GiveItem {
            item_name: "Mysterious Key".to_string(),
            description: "An ornate bronze key".to_string(),
        };

        let mut session = create_test_session();
        let session_id = session.session_id;
        let result = service.execute_tool(&tool, &mut session, session_id).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("Mysterious Key"));
        assert_eq!(result.state_changes.len(), 1);
        assert!(matches!(
            &result.state_changes[0],
            StateChange::ItemAdded { item, .. } if item == "Mysterious Key"
        ));
    }

    #[tokio::test]
    async fn test_execute_reveal_info_minor() {
        let service = ToolExecutionService::new();
        let tool = GameTool::RevealInfo {
            info_type: "lore".to_string(),
            content: "The ancient civilization was destroyed".to_string(),
            importance: InfoImportance::Minor,
        };

        let mut session = create_test_session();
        let session_id = session.session_id;
        let result = service.execute_tool(&tool, &mut session, session_id).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("minor"));
        assert_eq!(result.state_changes.len(), 1);
    }

    #[tokio::test]
    async fn test_execute_reveal_info_critical() {
        let service = ToolExecutionService::new();
        let tool = GameTool::RevealInfo {
            info_type: "quest".to_string(),
            content: "Your father is alive!".to_string(),
            importance: InfoImportance::Critical,
        };

        let mut session = create_test_session();
        let session_id = session.session_id;
        let result = service.execute_tool(&tool, &mut session, session_id).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("critical"));
        assert!(matches!(
            &result.state_changes[0],
            StateChange::InfoRevealed { info } if info.contains("Your father is alive!")
        ));
    }

    #[tokio::test]
    async fn test_execute_relationship_improve_slight() {
        let service = ToolExecutionService::new();
        let tool = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Slight,
            reason: "Good conversation".to_string(),
        };

        let mut session = create_test_session();
        let session_id = session.session_id;
        let result = service.execute_tool(&tool, &mut session, session_id).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("Improve"));
        assert!(result.description.contains("slight"));
        assert_eq!(result.state_changes.len(), 1);

        if let StateChange::RelationshipChanged { delta, .. } = &result.state_changes[0] {
            assert_eq!(*delta, 10);
        } else {
            panic!("Expected RelationshipChanged");
        }
    }

    #[tokio::test]
    async fn test_execute_relationship_improve_moderate() {
        let service = ToolExecutionService::new();
        let tool = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Moderate,
            reason: "Great help".to_string(),
        };

        let mut session = create_test_session();
        let session_id = session.session_id;
        let result = service.execute_tool(&tool, &mut session, session_id).await.unwrap();

        assert!(result.success);

        if let StateChange::RelationshipChanged { delta, .. } = &result.state_changes[0] {
            assert_eq!(*delta, 25);
        } else {
            panic!("Expected RelationshipChanged");
        }
    }

    #[tokio::test]
    async fn test_execute_relationship_improve_significant() {
        let service = ToolExecutionService::new();
        let tool = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Significant,
            reason: "Saved their life".to_string(),
        };

        let mut session = create_test_session();
        let session_id = session.session_id;
        let result = service.execute_tool(&tool, &mut session, session_id).await.unwrap();

        assert!(result.success);

        if let StateChange::RelationshipChanged { delta, .. } = &result.state_changes[0] {
            assert_eq!(*delta, 50);
        } else {
            panic!("Expected RelationshipChanged");
        }
    }

    #[tokio::test]
    async fn test_execute_relationship_worsen() {
        let service = ToolExecutionService::new();
        let tool = GameTool::ChangeRelationship {
            change: RelationshipChange::Worsen,
            amount: ChangeAmount::Significant,
            reason: "Betrayal".to_string(),
        };

        let mut session = create_test_session();
        let session_id = session.session_id;
        let result = service.execute_tool(&tool, &mut session, session_id).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("Worsen"));

        if let StateChange::RelationshipChanged { delta, .. } = &result.state_changes[0] {
            assert_eq!(*delta, -50);
        } else {
            panic!("Expected RelationshipChanged");
        }
    }

    #[tokio::test]
    async fn test_execute_trigger_event() {
        let service = ToolExecutionService::new();
        let tool = GameTool::TriggerEvent {
            event_type: "combat".to_string(),
            description: "A group of bandits appears!".to_string(),
        };

        let mut session = create_test_session();
        let session_id = session.session_id;
        let result = service.execute_tool(&tool, &mut session, session_id).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("combat"));
        assert!(result.description.contains("bandits"));
        assert_eq!(result.state_changes.len(), 1);
        assert!(matches!(
            &result.state_changes[0],
            StateChange::EventTriggered { .. }
        ));
    }

    #[tokio::test]
    async fn test_multiple_tools_sequence() {
        let service = ToolExecutionService::new();
        let mut session = create_test_session();
        let session_id = session.session_id;

        // Execute multiple tools in sequence
        let tool1 = GameTool::GiveItem {
            item_name: "Sword".to_string(),
            description: "A sharp blade".to_string(),
        };
        let result1 = service.execute_tool(&tool1, &mut session, session_id).await.unwrap();
        assert!(result1.success);

        let tool2 = GameTool::RevealInfo {
            info_type: "quest".to_string(),
            content: "Find the dragon".to_string(),
            importance: InfoImportance::Major,
        };
        let result2 = service.execute_tool(&tool2, &mut session, session_id).await.unwrap();
        assert!(result2.success);

        let tool3 = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Moderate,
            reason: "Helping out".to_string(),
        };
        let result3 = service.execute_tool(&tool3, &mut session, session_id).await.unwrap();
        assert!(result3.success);

        // Check that session history was updated
        assert!(session.conversation_history.len() >= 3);
    }
}
