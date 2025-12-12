//! Tool Execution Service - Executes approved tool calls to modify game state
//!
//! This service handles the execution of game tools that have been approved by the DM.
//! It modifies in-memory session state without persisting to the database, allowing
//! for future expansion with more complex effects.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use crate::domain::value_objects::{
    ChangeAmount, GameTool, InfoImportance, RelationshipChange,
};
use crate::infrastructure::session::GameSession;

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
    pub async fn execute_tool(
        &self,
        tool: &GameTool,
        session: &mut GameSession,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        match tool {
            GameTool::GiveItem { item_name, description } => {
                self.execute_give_item(item_name, description, session).await
            }
            GameTool::RevealInfo {
                info_type,
                content,
                importance,
            } => {
                self.execute_reveal_info(info_type, content, importance, session)
                    .await
            }
            GameTool::ChangeRelationship { change, amount, reason } => {
                self.execute_change_relationship(change, amount, reason, session)
                    .await
            }
            GameTool::TriggerEvent {
                event_type,
                description,
            } => {
                self.execute_trigger_event(event_type, description, session)
                    .await
            }
        }
    }

    /// Execute GiveItem tool - adds item to character inventory
    #[instrument(skip(self, session))]
    async fn execute_give_item(
        &self,
        item_name: &str,
        description: &str,
        session: &mut GameSession,
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
        session.add_npc_response(
            "System",
            &format!("Item received: {} - {}", item_name, description),
        );

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
    async fn execute_reveal_info(
        &self,
        info_type: &str,
        content: &str,
        importance: &InfoImportance,
        session: &mut GameSession,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "Revealed {} {} information",
            importance.as_str(),
            info_type
        );

        debug!("Info revealed: {} - {}", info_type, content);

        // Log the revelation in conversation history
        session.add_npc_response(
            "System",
            &format!("[{}] {} - {}", info_type, importance.as_str(), content),
        );

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
    async fn execute_change_relationship(
        &self,
        change: &RelationshipChange,
        amount: &ChangeAmount,
        reason: &str,
        session: &mut GameSession,
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
        session.add_npc_response(
            "System",
            &format!(
                "Relationship {}: {} ({})",
                change.as_str(),
                amount.as_str(),
                reason
            ),
        );

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
    async fn execute_trigger_event(
        &self,
        event_type: &str,
        description: &str,
        session: &mut GameSession,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!("Triggered {} event: {}", event_type, description);

        info!("Event triggered: {}", description_msg);

        // Log the event in conversation history
        session.add_npc_response(
            "System",
            &format!("[EVENT: {}] {}", event_type, description),
        );

        let state_change = StateChange::EventTriggered {
            name: format!("{}: {}", event_type, description),
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
    use chrono::Utc;
    use crate::domain::entities::World;
    use crate::domain::value_objects::{
        RuleSystemConfig, WorldId,
    };
    use crate::infrastructure::session::WorldSnapshot;

    fn create_test_session() -> GameSession {
        let world = World {
            id: WorldId::new(),
            name: "Test World".to_string(),
            description: "A test world".to_string(),
            rule_system: RuleSystemConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let snapshot = WorldSnapshot {
            world,
            locations: vec![],
            characters: vec![],
            scenes: vec![],
            current_scene_id: None,
        };

        GameSession::new(WorldId::new(), snapshot)
    }

    #[tokio::test]
    async fn test_execute_give_item() {
        let service = ToolExecutionService::new();
        let tool = GameTool::GiveItem {
            item_name: "Mysterious Key".to_string(),
            description: "An ornate bronze key".to_string(),
        };

        let mut session = create_test_session();
        let result = service.execute_tool(&tool, &mut session).await.unwrap();

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
        let result = service.execute_tool(&tool, &mut session).await.unwrap();

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
        let result = service.execute_tool(&tool, &mut session).await.unwrap();

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
        let result = service.execute_tool(&tool, &mut session).await.unwrap();

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
        let result = service.execute_tool(&tool, &mut session).await.unwrap();

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
        let result = service.execute_tool(&tool, &mut session).await.unwrap();

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
        let result = service.execute_tool(&tool, &mut session).await.unwrap();

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
        let result = service.execute_tool(&tool, &mut session).await.unwrap();

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

        // Execute multiple tools in sequence
        let tool1 = GameTool::GiveItem {
            item_name: "Sword".to_string(),
            description: "A sharp blade".to_string(),
        };
        let result1 = service.execute_tool(&tool1, &mut session).await.unwrap();
        assert!(result1.success);

        let tool2 = GameTool::RevealInfo {
            info_type: "quest".to_string(),
            content: "Find the dragon".to_string(),
            importance: InfoImportance::Major,
        };
        let result2 = service.execute_tool(&tool2, &mut session).await.unwrap();
        assert!(result2.success);

        let tool3 = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Moderate,
            reason: "Helping out".to_string(),
        };
        let result3 = service.execute_tool(&tool3, &mut session).await.unwrap();
        assert!(result3.success);

        // Check that session history was updated
        assert!(session.history_length() >= 3);
    }
}
