//! Game tools that can be called by the LLM
//!
//! Defines the enum of game mechanics that NPCs can suggest through the LLM.
//! Each tool represents a specific game action that requires DM approval.

/// Available tools/actions that an NPC can suggest via the LLM
///
/// These tools represent game mechanics that can be triggered by NPC behavior.
/// Tool calls are proposed by the LLM but require DM approval before execution.
///
/// # Examples
///
/// ```ignore
/// use wrldbldr_engine::domain::GameTool;
///
/// let tool = GameTool::GiveItem {
///     item_name: "Mysterious Key".to_string(),
///     description: "An ornate bronze key".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameTool {
    /// Give an item to the player
    ///
    /// # Fields
    /// - `item_name`: The name of the item
    /// - `description`: A description of the item
    GiveItem {
        item_name: String,
        description: String,
    },

    /// Reveal plot-relevant information
    ///
    /// # Fields
    /// - `info_type`: Category of information (lore, quest, character, location)
    /// - `content`: The information being revealed
    /// - `importance`: How critical this info is (minor, major, critical)
    RevealInfo {
        info_type: String,
        content: String,
        importance: InfoImportance,
    },

    /// Modify the relationship between an NPC and player
    ///
    /// # Fields
    /// - `change`: Direction of change (improve or worsen)
    /// - `amount`: Magnitude of change (slight, moderate, significant)
    /// - `reason`: Why the relationship is changing
    ChangeRelationship {
        change: RelationshipChange,
        amount: ChangeAmount,
        reason: String,
    },

    /// Trigger a game event or narrative beat
    ///
    /// # Fields
    /// - `event_type`: Type of event (combat, discovery, social, environmental)
    /// - `description`: What happens
    TriggerEvent {
        event_type: String,
        description: String,
    },
}

impl GameTool {
    /// Get the tool name for this variant
    pub fn name(&self) -> &'static str {
        match self {
            Self::GiveItem { .. } => "give_item",
            Self::RevealInfo { .. } => "reveal_info",
            Self::ChangeRelationship { .. } => "change_relationship",
            Self::TriggerEvent { .. } => "trigger_event",
        }
    }

    /// Check if this tool is allowed
    pub fn is_allowed(&self, allowed_tools: &[String]) -> bool {
        allowed_tools.iter().any(|tool| tool == self.name())
    }

    /// Get a human-readable description of what this tool will do
    pub fn description(&self) -> String {
        match self {
            Self::GiveItem { item_name, .. } => format!("Give '{}' to the player", item_name),
            Self::RevealInfo {
                importance,
                info_type,
                ..
            } => format!(
                "Reveal {} {} to the player",
                importance.as_str(),
                info_type
            ),
            Self::ChangeRelationship {
                change,
                amount,
                reason,
            } => format!(
                "{} relationship {} with player ({})",
                change.as_str(),
                amount.as_str(),
                reason
            ),
            Self::TriggerEvent { event_type, .. } => format!("Trigger {} event", event_type),
        }
    }
}

/// Importance levels for revealed information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InfoImportance {
    /// Minor plot detail
    Minor,
    /// Important to the current story arc
    Major,
    /// Critical revelation that changes everything
    Critical,
}

impl InfoImportance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minor => "minor",
            Self::Major => "major",
            Self::Critical => "critical",
        }
    }
}

/// Direction of relationship change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationshipChange {
    /// Improve the relationship
    Improve,
    /// Worsen the relationship
    Worsen,
}

impl RelationshipChange {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Improve => "improve",
            Self::Worsen => "worsen",
        }
    }
}

/// Magnitude of change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeAmount {
    /// Small change
    Slight,
    /// Medium change
    Moderate,
    /// Large change
    Significant,
}

impl ChangeAmount {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Slight => "slight",
            Self::Moderate => "moderate",
            Self::Significant => "significant",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_tool_names() {
        assert_eq!(
            GameTool::GiveItem {
                item_name: "Key".to_string(),
                description: "A key".to_string()
            }
            .name(),
            "give_item"
        );
        assert_eq!(
            GameTool::RevealInfo {
                info_type: "lore".to_string(),
                content: "History".to_string(),
                importance: InfoImportance::Major
            }
            .name(),
            "reveal_info"
        );
        assert_eq!(
            GameTool::ChangeRelationship {
                change: RelationshipChange::Improve,
                amount: ChangeAmount::Moderate,
                reason: "Good conversation".to_string()
            }
            .name(),
            "change_relationship"
        );
        assert_eq!(
            GameTool::TriggerEvent {
                event_type: "combat".to_string(),
                description: "Battle starts".to_string()
            }
            .name(),
            "trigger_event"
        );
    }

    #[test]
    fn test_tool_allowed() {
        let tool = GameTool::GiveItem {
            item_name: "Item".to_string(),
            description: "Desc".to_string(),
        };

        let allowed = vec!["give_item".to_string(), "reveal_info".to_string()];
        assert!(tool.is_allowed(&allowed));

        let not_allowed = vec!["trigger_event".to_string()];
        assert!(!tool.is_allowed(&not_allowed));
    }

    #[test]
    fn test_tool_descriptions() {
        let give_item = GameTool::GiveItem {
            item_name: "Sword".to_string(),
            description: "A sharp blade".to_string(),
        };
        assert!(give_item.description().contains("Sword"));

        let reveal = GameTool::RevealInfo {
            info_type: "quest".to_string(),
            content: "Find the artifact".to_string(),
            importance: InfoImportance::Critical,
        };
        assert!(reveal.description().contains("critical"));
    }
}
