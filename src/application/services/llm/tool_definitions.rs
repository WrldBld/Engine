//! Tool definitions for LLM game mechanics

use crate::application::ports::outbound::ToolDefinition;

/// Get the tool definitions for game mechanics
pub fn get_game_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "give_item".to_string(),
            description: "Give an item to the player character".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "item_name": {
                        "type": "string",
                        "description": "Name of the item to give"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description of the item"
                    }
                },
                "required": ["item_name", "description"]
            }),
        },
        ToolDefinition {
            name: "reveal_info".to_string(),
            description: "Reveal plot-relevant information to the player".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "info_type": {
                        "type": "string",
                        "description": "Category of information (lore, quest, character, location)"
                    },
                    "content": {
                        "type": "string",
                        "description": "The information being revealed"
                    },
                    "importance": {
                        "type": "string",
                        "enum": ["minor", "major", "critical"],
                        "description": "How important this information is to the plot"
                    }
                },
                "required": ["info_type", "content", "importance"]
            }),
        },
        ToolDefinition {
            name: "change_relationship".to_string(),
            description: "Modify the NPC's relationship with the player".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "change": {
                        "type": "string",
                        "enum": ["improve", "worsen"],
                        "description": "Direction of relationship change"
                    },
                    "amount": {
                        "type": "string",
                        "enum": ["slight", "moderate", "significant"],
                        "description": "Magnitude of the change"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Why the relationship changed"
                    }
                },
                "required": ["change", "amount", "reason"]
            }),
        },
        ToolDefinition {
            name: "trigger_event".to_string(),
            description: "Trigger a game event or narrative beat".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "event_type": {
                        "type": "string",
                        "description": "Type of event (combat, discovery, social, environmental)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description of what happens"
                    }
                },
                "required": ["event_type", "description"]
            }),
        },
        ToolDefinition {
            name: "modify_npc_motivation".to_string(),
            description: "Modify an NPC's motivation (goal, fear, desire, or secret)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "npc_id": {
                        "type": "string",
                        "description": "ID of the NPC to modify"
                    },
                    "motivation_type": {
                        "type": "string",
                        "description": "Type of motivation (goal, fear, desire, secret)"
                    },
                    "new_value": {
                        "type": "string",
                        "description": "The new motivation value"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Why the motivation is changing"
                    }
                },
                "required": ["npc_id", "motivation_type", "new_value", "reason"]
            }),
        },
        ToolDefinition {
            name: "modify_character_description".to_string(),
            description: "Update a character's description (appearance, personality, or backstory)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "character_id": {
                        "type": "string",
                        "description": "ID of the character to modify"
                    },
                    "change_type": {
                        "type": "string",
                        "description": "Type of change (appearance, personality, backstory)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description of what changed"
                    }
                },
                "required": ["character_id", "change_type", "description"]
            }),
        },
        ToolDefinition {
            name: "modify_npc_opinion".to_string(),
            description: "Change an NPC's opinion of a specific player character".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "npc_id": {
                        "type": "string",
                        "description": "ID of the NPC"
                    },
                    "target_pc_id": {
                        "type": "string",
                        "description": "ID of the player character"
                    },
                    "opinion_change": {
                        "type": "string",
                        "description": "How the opinion changes (e.g., 'more trusting', 'suspicious')"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Why the opinion is changing"
                    }
                },
                "required": ["npc_id", "target_pc_id", "opinion_change", "reason"]
            }),
        },
        ToolDefinition {
            name: "transfer_item".to_string(),
            description: "Transfer an item from one character to another".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "from_id": {
                        "type": "string",
                        "description": "ID of the character giving the item"
                    },
                    "to_id": {
                        "type": "string",
                        "description": "ID of the character receiving the item"
                    },
                    "item_name": {
                        "type": "string",
                        "description": "Name of the item being transferred"
                    }
                },
                "required": ["from_id", "to_id", "item_name"]
            }),
        },
        ToolDefinition {
            name: "add_condition".to_string(),
            description: "Add a condition to a character (e.g., Poisoned, Frightened)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "character_id": {
                        "type": "string",
                        "description": "ID of the character"
                    },
                    "condition_name": {
                        "type": "string",
                        "description": "Name of the condition"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description of the condition's effects"
                    },
                    "duration": {
                        "type": "string",
                        "description": "Duration of the condition (e.g., '1 hour', 'until rest', 'permanent')"
                    }
                },
                "required": ["character_id", "condition_name", "description"]
            }),
        },
        ToolDefinition {
            name: "remove_condition".to_string(),
            description: "Remove a condition from a character".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "character_id": {
                        "type": "string",
                        "description": "ID of the character"
                    },
                    "condition_name": {
                        "type": "string",
                        "description": "Name of the condition to remove"
                    }
                },
                "required": ["character_id", "condition_name"]
            }),
        },
        ToolDefinition {
            name: "update_character_stat".to_string(),
            description: "Update a character's stat value (e.g., health, mana, gold)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "character_id": {
                        "type": "string",
                        "description": "ID of the character"
                    },
                    "stat_name": {
                        "type": "string",
                        "description": "Name of the stat to update"
                    },
                    "delta": {
                        "type": "integer",
                        "description": "Amount to change the stat by (positive or negative)"
                    }
                },
                "required": ["character_id", "stat_name", "delta"]
            }),
        },
    ]
}
