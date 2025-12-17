//! Tool call parsing and validation

use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{
    ChangeAmount, GameTool, InfoImportance, RelationshipChange,
};

use super::{LLMServiceError, ProposedToolCall};

/// A proposed tool call that requires DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedToolCall {
    /// Name of the tool to call
    pub tool_name: String,
    /// Arguments for the tool call
    pub arguments: serde_json::Value,
    /// Human-readable description of what this will do
    pub description: String,
}

/// Parse tool calls from the LLM response into ProposedToolCall format
pub fn parse_tool_calls(response: &str) -> Vec<ProposedToolCall> {
    // Try to parse tool calls from JSON in the response
    // This handles cases where the model returns tool calls in the text
    let mut calls = Vec::new();

    // Look for JSON objects that might be tool calls
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            let potential_json = &response[start..=end];
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(potential_json) {
                if let Some(tool_name) = value.get("tool").and_then(|v| v.as_str()) {
                    calls.push(ProposedToolCall {
                        tool_name: tool_name.to_string(),
                        arguments: value
                            .get("arguments")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        description: value
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
        }
    }

    calls
}

/// Convert LLM ToolCall format to ProposedToolCall
pub fn parse_tool_calls_from_response(
    tool_calls: &[crate::application::ports::outbound::ToolCall],
) -> Vec<ProposedToolCall> {
    tool_calls
        .iter()
        .map(|tc| {
            let description = generate_tool_description(&tc.name, &tc.arguments);
            ProposedToolCall {
                tool_name: tc.name.clone(),
                arguments: tc.arguments.clone(),
                description,
            }
        })
        .collect()
}

/// Generate a human-readable description of a tool call
pub fn generate_tool_description(name: &str, arguments: &serde_json::Value) -> String {
    match name {
        "give_item" => {
            let item = arguments
                .get("item_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown item");
            format!("Give '{}' to the player", item)
        }
        "reveal_info" => {
            let info_type = arguments
                .get("info_type")
                .and_then(|v| v.as_str())
                .unwrap_or("information");
            let importance = arguments
                .get("importance")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("Reveal {} {} to the player", importance, info_type)
        }
        "change_relationship" => {
            let change = arguments
                .get("change")
                .and_then(|v| v.as_str())
                .unwrap_or("change");
            let amount = arguments
                .get("amount")
                .and_then(|v| v.as_str())
                .unwrap_or("slightly");
            format!("{} relationship {} with player", change, amount)
        }
        "trigger_event" => {
            let event_type = arguments
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("event");
            format!("Trigger {} event", event_type)
        }
        "modify_npc_motivation" => {
            let npc_id = arguments.get("npc_id").and_then(|v| v.as_str()).unwrap_or("NPC");
            let motivation_type = arguments.get("motivation_type").and_then(|v| v.as_str()).unwrap_or("motivation");
            format!("Modify {}'s {}", npc_id, motivation_type)
        }
        "modify_character_description" => {
            let character_id = arguments.get("character_id").and_then(|v| v.as_str()).unwrap_or("character");
            let change_type = arguments.get("change_type").and_then(|v| v.as_str()).unwrap_or("description");
            format!("Update {}'s {}", character_id, change_type)
        }
        "modify_npc_opinion" => {
            let npc_id = arguments.get("npc_id").and_then(|v| v.as_str()).unwrap_or("NPC");
            let target_pc_id = arguments.get("target_pc_id").and_then(|v| v.as_str()).unwrap_or("PC");
            format!("Change {}'s opinion of {}", npc_id, target_pc_id)
        }
        "transfer_item" => {
            let item = arguments.get("item_name").and_then(|v| v.as_str()).unwrap_or("item");
            let from = arguments.get("from_id").and_then(|v| v.as_str()).unwrap_or("giver");
            let to = arguments.get("to_id").and_then(|v| v.as_str()).unwrap_or("receiver");
            format!("Transfer '{}' from {} to {}", item, from, to)
        }
        "add_condition" => {
            let character_id = arguments.get("character_id").and_then(|v| v.as_str()).unwrap_or("character");
            let condition = arguments.get("condition_name").and_then(|v| v.as_str()).unwrap_or("condition");
            format!("Add '{}' condition to {}", condition, character_id)
        }
        "remove_condition" => {
            let character_id = arguments.get("character_id").and_then(|v| v.as_str()).unwrap_or("character");
            let condition = arguments.get("condition_name").and_then(|v| v.as_str()).unwrap_or("condition");
            format!("Remove '{}' condition from {}", condition, character_id)
        }
        "update_character_stat" => {
            let character_id = arguments.get("character_id").and_then(|v| v.as_str()).unwrap_or("character");
            let stat = arguments.get("stat_name").and_then(|v| v.as_str()).unwrap_or("stat");
            let delta = arguments.get("delta").and_then(|v| v.as_i64()).unwrap_or(0);
            let sign = if delta >= 0 { "+" } else { "" };
            format!("Update {}'s {} by {}{}", character_id, stat, sign, delta)
        }
        _ => format!("Call {} with provided arguments", name),
    }
}

/// Parse a single tool call into a GameTool
pub fn parse_single_tool(
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
        "modify_npc_motivation" => {
            let npc_id = arguments
                .get("npc_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing npc_id in modify_npc_motivation".to_string()))?
                .to_string();
            let motivation_type = arguments
                .get("motivation_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing motivation_type in modify_npc_motivation".to_string()))?
                .to_string();
            let new_value = arguments
                .get("new_value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing new_value in modify_npc_motivation".to_string()))?
                .to_string();
            let reason = arguments
                .get("reason")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing reason in modify_npc_motivation".to_string()))?
                .to_string();
            Ok(GameTool::ModifyNpcMotivation { npc_id, motivation_type, new_value, reason })
        }
        "modify_character_description" => {
            let character_id = arguments
                .get("character_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing character_id in modify_character_description".to_string()))?
                .to_string();
            let change_type = arguments
                .get("change_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing change_type in modify_character_description".to_string()))?
                .to_string();
            let description = arguments
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing description in modify_character_description".to_string()))?
                .to_string();
            Ok(GameTool::ModifyCharacterDescription { character_id, change_type, description })
        }
        "modify_npc_opinion" => {
            let npc_id = arguments
                .get("npc_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing npc_id in modify_npc_opinion".to_string()))?
                .to_string();
            let target_pc_id = arguments
                .get("target_pc_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing target_pc_id in modify_npc_opinion".to_string()))?
                .to_string();
            let opinion_change = arguments
                .get("opinion_change")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing opinion_change in modify_npc_opinion".to_string()))?
                .to_string();
            let reason = arguments
                .get("reason")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing reason in modify_npc_opinion".to_string()))?
                .to_string();
            Ok(GameTool::ModifyNpcOpinion { npc_id, target_pc_id, opinion_change, reason })
        }
        "transfer_item" => {
            let from_id = arguments
                .get("from_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing from_id in transfer_item".to_string()))?
                .to_string();
            let to_id = arguments
                .get("to_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing to_id in transfer_item".to_string()))?
                .to_string();
            let item_name = arguments
                .get("item_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing item_name in transfer_item".to_string()))?
                .to_string();
            Ok(GameTool::TransferItem { from_id, to_id, item_name })
        }
        "add_condition" => {
            let character_id = arguments
                .get("character_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing character_id in add_condition".to_string()))?
                .to_string();
            let condition_name = arguments
                .get("condition_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing condition_name in add_condition".to_string()))?
                .to_string();
            let description = arguments
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing description in add_condition".to_string()))?
                .to_string();
            let duration = arguments.get("duration").and_then(|v| v.as_str()).map(|s| s.to_string());
            Ok(GameTool::AddCondition { character_id, condition_name, description, duration })
        }
        "remove_condition" => {
            let character_id = arguments
                .get("character_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing character_id in remove_condition".to_string()))?
                .to_string();
            let condition_name = arguments
                .get("condition_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing condition_name in remove_condition".to_string()))?
                .to_string();
            Ok(GameTool::RemoveCondition { character_id, condition_name })
        }
        "update_character_stat" => {
            let character_id = arguments
                .get("character_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing character_id in update_character_stat".to_string()))?
                .to_string();
            let stat_name = arguments
                .get("stat_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| LLMServiceError::ParseError("Missing stat_name in update_character_stat".to_string()))?
                .to_string();
            let delta = arguments
                .get("delta")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| LLMServiceError::ParseError("Missing or invalid delta in update_character_stat".to_string()))?
                as i32;
            Ok(GameTool::UpdateCharacterStat { character_id, stat_name, delta })
        }
        unknown => Err(LLMServiceError::ParseError(
            format!("Unknown tool: {}", unknown),
        )),
    }
}

/// Validate tool calls against allowed tools from DirectorialNotes
///
/// Filters tool calls to only include those that are allowed in the current scene.
/// Returns a vector of valid tools and any validation errors.
pub fn validate_tool_calls(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_calls() {
        let response = r#"Some text {"tool": "give_item", "arguments": {"item_name": "key"}, "description": "Give key"} more text"#;

        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "give_item");
    }

    #[test]
    fn test_parse_single_tool_give_item() {
        let arguments = serde_json::json!({
            "item_name": "Mysterious Key",
            "description": "An ornate bronze key"
        });

        let result = parse_single_tool("give_item", &arguments);
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
        let (valid, invalid) = validate_tool_calls(&tools, &allowed);

        assert_eq!(valid.len(), 1);
        assert_eq!(invalid.len(), 1);
        assert!(invalid[0].contains("trigger_event"));
    }

    #[test]
    fn test_parse_single_tool_missing_field() {
        let arguments = serde_json::json!({
            "item_name": "Sword"
            // Missing "description"
        });

        let result = parse_single_tool("give_item", &arguments);
        assert!(result.is_err());
    }

    #[test]
    fn test_game_tool_names() {
        let give_item = GameTool::GiveItem {
            item_name: "Key".to_string(),
            description: "A key".to_string(),
        };
        assert_eq!(give_item.name(), "give_item");

        let reveal = GameTool::RevealInfo {
            info_type: "lore".to_string(),
            content: "History".to_string(),
            importance: InfoImportance::Minor,
        };
        assert_eq!(reveal.name(), "reveal_info");
    }
}
