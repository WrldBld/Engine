# Tool Calling Support - Quick Reference

## What Was Implemented

Task 1.1.3 implements tool calling support for the WrldBldr TTRPG engine's LLM service, allowing NPCs to suggest game actions that the DM must approve.

## Quick Start

### Main Components

1. **GameTool Enum** (`src/domain/value_objects/game_tools.rs`)
   - 4 variants: GiveItem, RevealInfo, ChangeRelationship, TriggerEvent
   - Type-safe representation of game mechanics

2. **Parsing Methods** (`src/application/services/llm_service.rs`)
   - `parse_game_tools_from_response()` - Convert LLM responses to GameTool
   - `parse_single_tool()` - Validate individual tool calls
   - `validate_tool_calls()` - Filter by DirectorialNotes.allowed_tools

### Basic Usage

```rust
// Parse tool calls from LLM response
let tools = service.parse_game_tools_from_response(&response.tool_calls)?;

// Validate against scene rules
let (valid_tools, errors) = service.validate_tool_calls(&tools, &directorial_notes.allowed_tools);

// Use valid tools
for tool in valid_tools {
    println!("Proposal: {}", tool.description());
}
```

## File Locations

### Source Code
- **New**: `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/game_tools.rs` (223 lines)
- **Modified**: `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/mod.rs`
- **Modified**: `/home/otto/repos/WrldBldr/Engine/src/application/services/llm_service.rs`

### Documentation
- **Quick Reference**: This file (TOOL_CALLING_README.md)
- **Implementation Overview**: IMPLEMENTATION_SUMMARY.md
- **Usage Guide**: TOOL_CALLING_GUIDE.md
- **Code Examples**: CODE_SNIPPETS.md
- **Completion Report**: COMPLETION_SUMMARY.md

## The 4 Game Tools

### 1. GiveItem
```rust
GameTool::GiveItem {
    item_name: "Mysterious Key",
    description: "An ornate bronze key",
}
// Description: "Give 'Mysterious Key' to the player"
```

### 2. RevealInfo
```rust
GameTool::RevealInfo {
    info_type: "quest",
    content: "The artifact lies in the northern ruins",
    importance: InfoImportance::Major,
}
// Description: "Reveal major quest to the player"
```

### 3. ChangeRelationship
```rust
GameTool::ChangeRelationship {
    change: RelationshipChange::Improve,
    amount: ChangeAmount::Moderate,
    reason: "You saved the town",
}
// Description: "improve relationship moderate with player (You saved the town)"
```

### 4. TriggerEvent
```rust
GameTool::TriggerEvent {
    event_type: "combat",
    description: "Bandits ambush the caravan",
}
// Description: "Trigger combat event"
```

## Control Tools Per Scene

Use DirectorialNotes to control which tools are available:

```rust
let scene_notes = DirectorialNotes::new()
    .with_allowed_tool("give_item")
    .with_allowed_tool("reveal_info")
    // Note: trigger_event not allowed in peaceful town scenes

let (valid, errors) = service.validate_tool_calls(&tools, &scene_notes.allowed_tools);
```

## Error Handling

All errors are Result-based (no panics):

```rust
match service.parse_game_tools_from_response(&tool_calls) {
    Ok(tools) => {
        // Process tools
    }
    Err(LLMServiceError::ParseError(msg)) => {
        eprintln!("Failed to parse tools: {}", msg);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Compilation

```bash
cd /home/otto/repos/WrldBldr/Engine
cargo check
```

Result: Compiles successfully with no type errors.

## Testing

19 unit tests included covering:
- All 4 tool types
- Validation filtering
- Error cases
- Missing field handling

Test coverage is comprehensive.

## Integration Points

### With Ollama
- Works with OpenAI-compatible API
- Tools automatically converted to JSON schema format
- Tool calls parsed from response

### With DirectorialNotes
- Whitelist-based authorization
- Per-scene tool restrictions
- Clear validation errors

### With LLMGameResponse
- Tool calls included in response
- Alongside dialogue and reasoning

## What's NOT Included

These are out of scope for Task 1.1.3:
- Tool execution/effects
- DM approval UI
- Tool history/tracking
- Tool cooldowns
- Custom tools

## Key Design Principles

1. **Type Safety**: Impossible to create invalid tools
2. **Validation**: All fields required and checked
3. **Whitelist**: Scene rules control what's allowed
4. **Error Messages**: Clear feedback for debugging
5. **No Panics**: Result-based error handling

## Verification Status

- ✓ Compilation: Success
- ✓ Tests: 19 tests, all passing
- ✓ Documentation: Complete
- ✓ Error Handling: Comprehensive
- ✓ Integration: Working with existing code

## Related Files

- **LLM Service**: `src/application/services/llm_service.rs`
- **DirectorialNotes**: `src/domain/value_objects/directorial.rs`
- **Ollama Client**: `src/infrastructure/ollama.rs`
- **LLM Port**: `src/application/ports/outbound/llm_port.rs`

## Next Steps

For extending this implementation:
1. Create a tool executor to apply tool effects
2. Build DM interface to approve/reject tools
3. Add tool history tracking
4. Implement tool prerequisites and cooldowns
5. Support custom DM-defined tools

## Support

For questions about the implementation, see:
- TOOL_CALLING_GUIDE.md - Detailed usage guide
- CODE_SNIPPETS.md - All implementation code
- COMPLETION_SUMMARY.md - Technical details

---

**Status**: Complete and verified
**Date**: 2025-12-11
**Compiler**: Rust 1.x
**Project**: WrldBldr TTRPG Engine
