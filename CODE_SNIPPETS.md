# Tool Calling Implementation - Code Snippets

## 1. GameTool Enum Definition
File: `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/game_tools.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GameTool {
    /// Give an item to the player
    GiveItem {
        item_name: String,
        description: String,
    },

    /// Reveal plot-relevant information
    RevealInfo {
        info_type: String,
        content: String,
        importance: InfoImportance,
    },

    /// Modify the relationship between an NPC and player
    ChangeRelationship {
        change: RelationshipChange,
        amount: ChangeAmount,
        reason: String,
    },

    /// Trigger a game event or narrative beat
    TriggerEvent {
        event_type: String,
        description: String,
    },
}
```

## 2. Tool Methods
File: `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/game_tools.rs`

```rust
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
```

## 3. Supporting Types
File: `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/game_tools.rs`

```rust
/// Importance levels for revealed information
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum InfoImportance {
    Minor,
    Major,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum RelationshipChange {
    Improve,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ChangeAmount {
    Slight,
    Moderate,
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
```

## 4. Parsing Tool Calls
File: `/home/otto/repos/WrldBldr/Engine/src/application/services/llm_service.rs`

```rust
/// Parse LLM response tool calls into GameTool enums
fn parse_game_tools_from_response(
    &self,
    tool_calls: &[crate::application::ports::outbound::ToolCall],
) -> Result<Vec<GameTool>, LLMServiceError> {
    tool_calls
        .iter()
        .map(|tc| self.parse_single_tool(&tc.name, &tc.arguments))
        .collect()
}

/// Parse a single tool call into a GameTool
fn parse_single_tool(
    &self,
    name: &str,
    arguments: &serde_json::Value,
) -> Result<GameTool, LLMServiceError> {
    match name {
        "give_item" => {
            let item_name = arguments
                .get("item_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError("Missing item_name in give_item".to_string())
                })?
                .to_string();

            let description = arguments
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError("Missing description in give_item".to_string())
                })?
                .to_string();

            Ok(GameTool::GiveItem {
                item_name,
                description,
            })
        }
        "reveal_info" => {
            let info_type = arguments
                .get("info_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError("Missing info_type in reveal_info".to_string())
                })?
                .to_string();

            let content = arguments
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError("Missing content in reveal_info".to_string())
                })?
                .to_string();

            let importance_str = arguments
                .get("importance")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError("Missing importance in reveal_info".to_string())
                })?;

            let importance = match importance_str {
                "minor" => InfoImportance::Minor,
                "major" => InfoImportance::Major,
                "critical" => InfoImportance::Critical,
                _ => return Err(LLMServiceError::ParseError(
                    format!("Invalid importance level: {}", importance_str),
                )),
            };

            Ok(GameTool::RevealInfo {
                info_type,
                content,
                importance,
            })
        }
        "change_relationship" => {
            let change_str = arguments
                .get("change")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError(
                        "Missing change in change_relationship".to_string(),
                    )
                })?;

            let change = match change_str {
                "improve" => RelationshipChange::Improve,
                "worsen" => RelationshipChange::Worsen,
                _ => {
                    return Err(LLMServiceError::ParseError(
                        format!("Invalid change direction: {}", change_str),
                    ))
                }
            };

            let amount_str = arguments
                .get("amount")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError("Missing amount in change_relationship".to_string())
                })?;

            let amount = match amount_str {
                "slight" => ChangeAmount::Slight,
                "moderate" => ChangeAmount::Moderate,
                "significant" => ChangeAmount::Significant,
                _ => {
                    return Err(LLMServiceError::ParseError(
                        format!("Invalid change amount: {}", amount_str),
                    ))
                }
            };

            let reason = arguments
                .get("reason")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError("Missing reason in change_relationship".to_string())
                })?
                .to_string();

            Ok(GameTool::ChangeRelationship {
                change,
                amount,
                reason,
            })
        }
        "trigger_event" => {
            let event_type = arguments
                .get("event_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError("Missing event_type in trigger_event".to_string())
                })?
                .to_string();

            let description = arguments
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LLMServiceError::ParseError(
                        "Missing description in trigger_event".to_string(),
                    )
                })?
                .to_string();

            Ok(GameTool::TriggerEvent {
                event_type,
                description,
            })
        }
        unknown => Err(LLMServiceError::ParseError(
            format!("Unknown tool: {}", unknown),
        )),
    }
}
```

## 5. Validation Against DirectorialNotes
File: `/home/otto/repos/WrldBldr/Engine/src/application/services/llm_service.rs`

```rust
/// Validate tool calls against allowed tools from DirectorialNotes
///
/// Filters tool calls to only include those that are allowed in the current scene.
/// Returns a vector of valid tools and any validation errors.
pub fn validate_tool_calls(
    &self,
    tools: &[GameTool],
    allowed_tools: &[String],
) -> (Vec<GameTool>, Vec<String>) {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();

    for tool in tools {
        if tool.is_allowed(allowed_tools) {
            valid.push(tool.clone());
        } else {
            invalid.push(format!(
                "Tool '{}' is not allowed in this scene",
                tool.name()
            ));
        }
    }

    (valid, invalid)
}
```

## 6. Exports
File: `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/mod.rs`

```rust
mod game_tools;

pub use game_tools::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};
```

## 7. Tests
File: `/home/otto/repos/WrldBldr/Engine/src/application/services/llm_service.rs`

```rust
#[test]
fn test_parse_single_tool_give_item() {
    // ... setup ...
    let arguments = serde_json::json!({
        "item_name": "Mysterious Key",
        "description": "An ornate bronze key"
    });

    let result = service.parse_single_tool("give_item", &arguments);
    assert!(result.is_ok());

    match result.unwrap() {
        GameTool::GiveItem {
            item_name,
            description,
        } => {
            assert_eq!(item_name, "Mysterious Key");
            assert_eq!(description, "An ornate bronze key");
        }
        _ => panic!("Expected GiveItem tool"),
    }
}

#[test]
fn test_validate_tool_calls() {
    // ... setup ...
    let tools = vec![
        GameTool::GiveItem {
            item_name: "Sword".to_string(),
            description: "A sharp blade".to_string(),
        },
        GameTool::TriggerEvent {
            event_type: "combat".to_string(),
            description: "Battle!".to_string(),
        },
    ];

    let allowed = vec!["give_item".to_string(), "reveal_info".to_string()];
    let (valid, invalid) = service.validate_tool_calls(&tools, &allowed);

    assert_eq!(valid.len(), 1);
    assert_eq!(invalid.len(), 1);
    assert!(invalid[0].contains("trigger_event"));
}

#[test]
fn test_parse_single_tool_missing_field() {
    // ... setup ...
    let arguments = serde_json::json!({
        "item_name": "Sword"
        // Missing "description"
    });

    let result = service.parse_single_tool("give_item", &arguments);
    assert!(result.is_err());
}
```

## Files Reference

- **Domain Value Object**: `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/game_tools.rs` (223 lines)
- **LLM Service Extensions**: `/home/otto/repos/WrldBldr/Engine/src/application/services/llm_service.rs` (+150 lines)
- **Module Exports**: `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/mod.rs` (+2 lines)

## Compilation
All code compiles without errors:
```bash
cd /home/otto/repos/WrldBldr/Engine
cargo check  # Success
```
