# Tool Calling Support - Implementation Guide

## Overview
This document explains how the tool calling system works in WrldBldr's LLM service.

## Architecture

```
LLM Response
    ↓
[Raw Tool Calls from Ollama]
    ↓
parse_game_tools_from_response()
    ↓
[Strongly-typed GameTool enums with validation]
    ↓
validate_tool_calls() [against DirectorialNotes.allowed_tools]
    ↓
[Filtered tools ready for DM approval]
```

## Core Types

### GameTool Enum

The main enum representing all possible game actions an NPC can suggest:

```rust
pub enum GameTool {
    GiveItem {
        item_name: String,
        description: String,
    },
    RevealInfo {
        info_type: String,
        content: String,
        importance: InfoImportance,  // Minor | Major | Critical
    },
    ChangeRelationship {
        change: RelationshipChange,  // Improve | Worsen
        amount: ChangeAmount,         // Slight | Moderate | Significant
        reason: String,
    },
    TriggerEvent {
        event_type: String,
        description: String,
    },
}
```

## Usage Example

### 1. Basic Tool Parsing

```rust
use wrldbldr_engine::application::services::LLMService;
use wrldbldr_engine::domain::GameTool;

// Assume we have an LLM service instance
let service = LLMService::new(ollama_client);

// Parse tool calls from LLM response
let raw_tool_calls: Vec<ToolCall> = response.tool_calls;
let tools: Result<Vec<GameTool>> = service.parse_game_tools_from_response(&raw_tool_calls);

match tools {
    Ok(parsed_tools) => {
        for tool in parsed_tools {
            println!("Proposed: {}", tool.description());
        }
    }
    Err(e) => eprintln!("Parse error: {}", e),
}
```

### 2. Validation Against Scene Rules

```rust
use wrldbldr_engine::domain::DirectorialNotes;

// Get scene directorial notes (passed from game state)
let notes = DirectorialNotes::new()
    .with_allowed_tool("give_item")
    .with_allowed_tool("reveal_info")
    // Note: trigger_event not allowed in this scene
    .with_forbidden_topic("politics");

// Validate tools against scene rules
let (valid_tools, errors) = service.validate_tool_calls(&parsed_tools, &notes.allowed_tools);

if !errors.is_empty() {
    println!("Invalid tools proposed:");
    for error in errors {
        println!("  - {}", error);  // e.g., "Tool 'trigger_event' is not allowed in this scene"
    }
}

// Only valid_tools should be sent to DM for approval
for tool in valid_tools {
    println!("Valid proposal: {}", tool.description());
}
```

### 3. Full Example: Processing NPC Response

```rust
let request = GamePromptRequest {
    player_action: PlayerActionContext {
        action_type: "speak".to_string(),
        target: Some("Bartender".to_string()),
        dialogue: Some("What's the local news?".to_string()),
    },
    scene_context: SceneContext {
        scene_name: "The Tavern".to_string(),
        location_name: "Port City".to_string(),
        time_context: "Evening".to_string(),
        present_characters: vec!["Bartender".to_string()],
    },
    directorial_notes: String::new(),
    conversation_history: vec![],
    responding_character: CharacterContext {
        name: "Tavern Keeper".to_string(),
        archetype: "Weathered sailor turned innkeeper".to_string(),
        current_mood: Some("Relaxed".to_string()),
        wants: vec!["Make profit".to_string(), "Help the town".to_string()],
        relationship_to_player: Some("Friendly".to_string()),
    },
};

let directorial_notes = DirectorialNotes::new()
    .with_allowed_tool("give_item")      // Can give quests/items
    .with_allowed_tool("reveal_info")    // Can share rumors
    .with_allowed_tool("change_relationship"); // Can bond with player

// Generate response with tools enabled
let response = service.generate_npc_response(request).await?;

// Response includes dialogue AND proposed tools
println!("NPC says: {}", response.npc_dialogue);

// Parse and validate tool calls
let raw_calls: Vec<ToolCall> = response.tool_calls.clone();
let parsed_tools = service.parse_game_tools_from_response(&raw_calls)?;
let (valid_tools, _errors) = service.validate_tool_calls(&parsed_tools, &directorial_notes.allowed_tools);

// Display to DM for approval
for tool in valid_tools {
    println!("NPC suggests: {}", tool.description());
}
```

## Tool Descriptions

Each tool variant has a semantic description:

```rust
// GiveItem example
let tool = GameTool::GiveItem {
    item_name: "Worn Map".to_string(),
    description: "An old treasure map with mysterious markings".to_string(),
};
println!("{}", tool.description());
// Output: "Give 'Worn Map' to the player"

// RevealInfo example
let tool = GameTool::RevealInfo {
    info_type: "quest".to_string(),
    content: "There's a dungeon three days north".to_string(),
    importance: InfoImportance::Major,
};
println!("{}", tool.description());
// Output: "Reveal major quest to the player"

// ChangeRelationship example
let tool = GameTool::ChangeRelationship {
    change: RelationshipChange::Improve,
    amount: ChangeAmount::Moderate,
    reason: "You helped defend the town".to_string(),
};
println!("{}", tool.description());
// Output: "improve relationship moderate with player (You helped defend the town)"
```

## Error Handling

### Parse Errors
```rust
let result = service.parse_single_tool("give_item", &serde_json::json!({
    "item_name": "Sword",
    // Missing "description"
}));

// Result: Err(LLMServiceError::ParseError("Missing description in give_item"))
```

### Validation Errors
```rust
let tool = GameTool::TriggerEvent {
    event_type: "combat".to_string(),
    description: "Battle!".to_string(),
};

let allowed = vec!["give_item".to_string(), "reveal_info".to_string()];
let (_valid, errors) = service.validate_tool_calls(&[tool], &allowed);

// Result: errors = vec!["Tool 'trigger_event' is not allowed in this scene"]
```

## Integration with DirectorialNotes

DirectorialNotes controls tool availability per scene:

```rust
pub struct DirectorialNotes {
    pub allowed_tools: Vec<String>,  // e.g., vec!["give_item", "reveal_info"]
    // ... other fields
}
```

Scene configuration example:
```rust
// Battle scene - only trigger events
let battle_notes = DirectorialNotes::new()
    .with_allowed_tool("trigger_event")
    .with_tone(ToneGuidance::Tense)
    .with_pacing(PacingGuidance::Urgent);

// Exploration scene - more flexibility
let exploration_notes = DirectorialNotes::new()
    .with_allowed_tool("give_item")
    .with_allowed_tool("reveal_info")
    .with_allowed_tool("trigger_event")
    .with_tone(ToneGuidance::Mysterious)
    .with_pacing(PacingGuidance::Slow);

// Social scene - relationship changes
let social_notes = DirectorialNotes::new()
    .with_allowed_tool("reveal_info")
    .with_allowed_tool("change_relationship")
    .with_tone(ToneGuidance::Lighthearted);
```

## Ollama Integration

The tool calling support works with Ollama's OpenAI-compatible API:

```
Ollama Tools Format (OpenAI compatible):
{
  "type": "function",
  "function": {
    "name": "give_item",
    "description": "Give an item to the player character",
    "parameters": {
      "type": "object",
      "properties": {
        "item_name": { "type": "string" },
        "description": { "type": "string" }
      },
      "required": ["item_name", "description"]
    }
  }
}
```

The service automatically:
1. Converts GameTool definitions to OpenAI tool format
2. Sends tools in the API request
3. Parses tool calls from Ollama's response
4. Converts to strongly-typed GameTool enums

## Thread Safety

All types implement Clone for easy sharing in async contexts:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GameTool { ... }
```

Safe to use with Arc/Mutex in multi-threaded scenarios:

```rust
let tools = Arc::new(Mutex::new(parsed_tools));
// Can be shared across await boundaries
```

## Testing

Unit tests verify:

```rust
#[cfg(test)]
mod tests {
    // Test parsing each tool type
    fn test_parse_single_tool_give_item();

    // Test validation filters correctly
    fn test_validate_tool_calls();

    // Test error handling
    fn test_parse_single_tool_missing_field();

    // Test GameTool methods
    fn test_game_tool_names();
}
```

Run tests:
```bash
cargo test llm_service::tests
```

## Best Practices

1. **Always validate** tool calls against DirectorialNotes.allowed_tools
2. **Handle errors** gracefully - some LLM responses may not include tool calls
3. **Filter before presenting to DM** - only show valid tools
4. **Log tool proposals** - helps debug LLM behavior
5. **Batch multiple tools** - LLM can suggest multiple tools in one response
6. **Preserve reasoning** - LLMGameResponse includes internal_reasoning for context

Example:
```rust
let response = service.generate_npc_response(request).await?;

// Always check both dialogue and tools
if !response.npc_dialogue.is_empty() {
    println!("NPC: {}", response.npc_dialogue);
}

if !response.proposed_tool_calls.is_empty() {
    for call in &response.proposed_tool_calls {
        tracing::info!("Proposed action: {}", call.description);
    }
}

// Internal reasoning helps understand the NPC's motivation
if !response.internal_reasoning.is_empty() {
    println!("(NPC thinking: {})", response.internal_reasoning);
}
```

## Future Extensions

Potential additions:
1. **Tool execution** - Execute validated tools and apply effects
2. **Tool history** - Track which tools were approved/rejected
3. **Custom tools** - Allow DM to define scene-specific tools
4. **Tool cooldowns** - Prevent NPC from using same tool repeatedly
5. **Tool prerequisites** - Some tools only available if conditions met
6. **Undo/Redo** - Revert tool effects if DM rejects them
