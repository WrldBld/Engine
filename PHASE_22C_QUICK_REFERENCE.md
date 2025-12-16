# Phase 22C - Quick Reference Card

## Location of Phase 22C DTOs

```
Application Layer:
  src/application/dto/queue_items.rs
    - EnhancedChallengeSuggestion
    - EnhancedOutcomes
    - OutcomeDetail

Infrastructure Layer:
  src/infrastructure/websocket/messages.rs
    - OutcomeDetailData
    - AdHocOutcomes
    - ClientMessage::RegenerateOutcome
    - ClientMessage::DiscardChallenge
    - ClientMessage::CreateAdHocChallenge
    - ServerMessage::OutcomeRegenerated
    - ServerMessage::ChallengeDiscarded
    - ServerMessage::AdHocChallengeCreated

Domain Layer:
  src/domain/value_objects/dice.rs
    - DiceRollInput
```

## Import Statements

```rust
// Application DTOs
use crate::application::dto::{
    EnhancedChallengeSuggestion,
    EnhancedOutcomes,
    OutcomeDetail,
};

// WebSocket Messages
use crate::infrastructure::websocket::messages::{
    ClientMessage,
    ServerMessage,
    OutcomeDetailData,
    AdHocOutcomes,
};

// Domain Value Objects
use crate::domain::value_objects::DiceRollInput;
```

## Core Structures (Copy-Paste Ready)

### EnhancedChallengeSuggestion
```rust
EnhancedChallengeSuggestion {
    challenge_id: Option<String>,      // None for ad-hoc
    challenge_name: String,
    skill_name: String,
    difficulty_display: String,
    npc_reply: String,
    outcomes: EnhancedOutcomes,
    reasoning: String,
}
```

### EnhancedOutcomes
```rust
EnhancedOutcomes {
    critical_success: Option<OutcomeDetail>,
    success: OutcomeDetail,            // Required
    failure: OutcomeDetail,            // Required
    critical_failure: Option<OutcomeDetail>,
}
```

### OutcomeDetail
```rust
OutcomeDetail {
    flavor_text: String,               // Narrative
    scene_direction: String,           // Game effects
    proposed_tools: Vec<ProposedToolInfo>,
}
```

## Message Quick Reference

### Client → Server
```rust
// Regenerate one or all outcomes
ClientMessage::RegenerateOutcome {
    request_id: String,
    outcome_type: Option<String>,      // "success", "failure", etc.
    guidance: Option<String>,
}

// Discard challenge suggestion
ClientMessage::DiscardChallenge {
    request_id: String,
    feedback: Option<String>,
}

// Create challenge without LLM
ClientMessage::CreateAdHocChallenge {
    challenge_name: String,
    skill_name: String,
    difficulty: String,
    target_pc_id: String,
    outcomes: AdHocOutcomes,
}
```

### Server → Client
```rust
// Outcome regenerated
ServerMessage::OutcomeRegenerated {
    request_id: String,
    outcome_type: String,
    new_outcome: OutcomeDetailData,
}

// Challenge discarded
ServerMessage::ChallengeDiscarded {
    request_id: String,
}

// Ad-hoc challenge created
ServerMessage::AdHocChallengeCreated {
    challenge_id: String,
    challenge_name: String,
    target_pc_id: String,
}
```

## DiceRollInput Usage

```rust
// Player enters formula
let input = DiceRollInput::Formula("1d20+5".to_string());
let result = input.resolve()?;

// Player enters manual result
let input = DiceRollInput::ManualResult(18);
let result = input.resolve()?;

// Apply character modifier
let input = DiceRollInput::Formula("1d20".to_string());
let result = input.resolve_with_modifier(3)?;  // Adds +3
```

## Trait/Type Dependencies

```rust
// ProposedToolInfo (used in all outcome details)
use crate::domain::value_objects::ProposedToolInfo;

ProposedToolInfo {
    id: String,
    name: String,
    description: String,
    arguments: serde_json::Value,
}
```

## Common Patterns

### Pattern 1: Create Enhanced Challenge

```rust
let suggestion = EnhancedChallengeSuggestion {
    challenge_id: Some("ch_123".to_string()),
    challenge_name: "Persuade Guard".to_string(),
    skill_name: "Persuasion".to_string(),
    difficulty_display: "DC 13".to_string(),
    npc_reply: "The guard eyes you.".to_string(),
    outcomes: EnhancedOutcomes {
        critical_success: Some(OutcomeDetail { ... }),
        success: OutcomeDetail { ... },
        failure: OutcomeDetail { ... },
        critical_failure: None,
    },
    reasoning: "Guard is cautious but reasonable.".to_string(),
};
```

### Pattern 2: Send Regeneration Request

```rust
// In WebSocket handler
let msg = ClientMessage::RegenerateOutcome {
    request_id: request_id.clone(),
    outcome_type: Some("success".to_string()),
    guidance: Some("More dramatic, with magic involved".to_string()),
};
// Send to server...
```

### Pattern 3: Handle Regenerated Outcome

```rust
// In message handler
match message {
    ServerMessage::OutcomeRegenerated { request_id, outcome_type, new_outcome } => {
        // new_outcome is OutcomeDetailData with flavor_text, scene_direction, proposed_tools
        update_approval_ui(&request_id, &outcome_type, &new_outcome);
    }
    // ...
}
```

### Pattern 4: Create Ad-hoc Challenge

```rust
let msg = ClientMessage::CreateAdHocChallenge {
    challenge_name: "Jump the Chasm".to_string(),
    skill_name: "Athletics".to_string(),
    difficulty: "DC 14".to_string(),
    target_pc_id: "pc_warrior".to_string(),
    outcomes: AdHocOutcomes {
        success: "You land safely on the other side.".to_string(),
        failure: "You don't make it and fall!".to_string(),
        critical_success: Some("You leap with impressive acrobatics.".to_string()),
        critical_failure: Some("You slip and tumble down.".to_string()),
    },
};
```

## Dice Formula Examples

Supported by `DiceRollInput::Formula`:
- `1d20` - Single d20
- `1d20+5` - d20 plus modifier
- `2d6` - Two d6
- `2d6+3` - Two d6 plus modifier
- `1d100` - Percentile
- `3d8-1` - Multiple dice with negative modifier

## Testing Checklist

- [ ] EnhancedChallengeSuggestion serializes/deserializes to JSON
- [ ] OutcomeDetail with empty proposed_tools list works
- [ ] AdHocOutcomes with only success/failure works
- [ ] DiceRollInput::Formula parses valid formulas
- [ ] DiceRollInput::Manual works with positive/negative values
- [ ] WebSocket messages round-trip through serde
- [ ] RegenerateOutcome with outcome_type = None works

## Common Mistakes to Avoid

❌ **Don't**: Put serde on domain entities
✓ **Do**: Use OutcomeDetail (application DTO) not domain value objects directly

❌ **Don't**: Forget challenge_id can be None (for ad-hoc)
✓ **Do**: Check `challenge_id.is_none()` to detect ad-hoc challenges

❌ **Don't**: Assume critical_success/critical_failure exist
✓ **Do**: Use Option types and pattern match

❌ **Don't**: Parse dice formulas yourself
✓ **Do**: Use `DiceRollInput::Formula().resolve()`

❌ **Don't**: Send OutcomeDetail directly in WebSocket
✓ **Do**: Use OutcomeDetailData which is separate from application DTO

## File Sizes

- `queue_items.rs`: ~190 lines (Phase 22C DTOs)
- `messages.rs`: ~420 lines (all WebSocket messages including Phase 22C)
- `dice.rs`: ~180 lines (all dice-related including DiceRollInput)

## Phase 22C Completion Status

- [x] EnhancedChallengeSuggestion implemented
- [x] EnhancedOutcomes implemented
- [x] OutcomeDetail implemented
- [x] WebSocket messages defined
- [x] DiceRollInput implemented
- [ ] LLM regeneration service (Phase 22C Services)
- [ ] Outcome trigger execution (Phase 22C Services)
- [ ] WebSocket handlers (Phase 22D)

