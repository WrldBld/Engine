# Phase 22C - Enhanced Challenge Suggestions DTOs - Implementation Status

**Date**: 2025-12-16
**Status**: DTOs Implemented, Services Pending
**Priority**: HIGH

## Overview

Phase 22C implements the data structures (DTOs) for enhanced skill challenge suggestions with detailed outcomes and tool execution receipts. The DTOs are fully defined and properly layered according to hexagonal architecture.

## What's Already Implemented

### 1. Application Layer DTOs (`src/application/dto/queue_items.rs`)

All required DTOs are fully implemented:

#### `EnhancedChallengeSuggestion`
```rust
pub struct EnhancedChallengeSuggestion {
    pub challenge_id: Option<String>,  // None for ad-hoc
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub npc_reply: String,
    pub outcomes: EnhancedOutcomes,
    pub reasoning: String,
}
```

- Supports both predefined challenges (with challenge_id) and ad-hoc challenges (challenge_id = None)
- Includes NPC reply text for flavor
- Contains detailed outcome structure
- Includes internal reasoning (shown to DM only)

#### `EnhancedOutcomes`
```rust
pub struct EnhancedOutcomes {
    pub critical_success: Option<OutcomeDetail>,
    pub success: OutcomeDetail,
    pub failure: OutcomeDetail,
    pub critical_failure: Option<OutcomeDetail>,
}
```

- Supports all four outcome tiers
- Critical success/failure are optional for simpler challenges
- Success and failure are required

#### `OutcomeDetail`
```rust
pub struct OutcomeDetail {
    pub flavor_text: String,
    pub scene_direction: String,
    pub proposed_tools: Vec<ProposedToolInfo>,
}
```

- Narrative flavor text for the outcome
- Scene direction (what changes in the game world)
- Tool receipt list (what tools/effects will be executed)

### 2. WebSocket Message Layer (`src/infrastructure/websocket/messages.rs`)

#### New Server Messages (Already Defined)

**`OutcomeRegenerated`** - DM regenerates a single outcome or all outcomes
```rust
OutcomeRegenerated {
    request_id: String,
    outcome_type: String,  // "success", "failure", "critical_success", "critical_failure", "all"
    new_outcome: OutcomeDetailData,
}
```

**`ChallengeDiscarded`** - DM rejects the challenge suggestion
```rust
ChallengeDiscarded {
    request_id: String,
}
```

**`AdHocChallengeCreated`** - Confirmation that ad-hoc challenge was created
```rust
AdHocChallengeCreated {
    challenge_id: String,
    challenge_name: String,
    target_pc_id: String,
}
```

#### Supporting Data Types

**`OutcomeDetailData`** - WebSocket representation of outcome details
```rust
pub struct OutcomeDetailData {
    pub flavor_text: String,
    pub scene_direction: String,
    pub proposed_tools: Vec<ProposedToolInfo>,
}
```

**`AdHocOutcomes`** - Outcomes provided by DM for ad-hoc challenges
```rust
pub struct AdHocOutcomes {
    pub success: String,
    pub failure: String,
    pub critical_success: Option<String>,
    pub critical_failure: Option<String>,
}
```

#### New Client Messages (Already Defined)

**`RegenerateOutcome`** - Request to regenerate outcome(s)
```rust
RegenerateOutcome {
    request_id: String,
    outcome_type: Option<String>,  // None = regenerate all
    guidance: Option<String>,       // DM guidance for regeneration
}
```

**`DiscardChallenge`** - Request to discard challenge suggestion
```rust
DiscardChallenge {
    request_id: String,
    feedback: Option<String>,
}
```

**`CreateAdHocChallenge`** - Request to create a challenge without LLM
```rust
CreateAdHocChallenge {
    challenge_name: String,
    skill_name: String,
    difficulty: String,
    target_pc_id: String,
    outcomes: AdHocOutcomes,
}
```

### 3. Domain Layer Value Objects

#### `DiceRollInput` (Already Implemented in `domain/value_objects/dice.rs`)

```rust
pub enum DiceRollInput {
    Formula(String),      // e.g., "1d20+5"
    ManualResult(i32),    // e.g., 18
}

impl DiceRollInput {
    pub fn resolve(&self) -> Result<DiceRollResult, DiceParseError>
    pub fn resolve_with_modifier(&self, skill_modifier: i32) -> Result<DiceRollResult, DiceParseError>
}
```

- Supports both dice formula parsing ("1d20+5") and manual results
- Includes modifier application for skill bonuses
- Handles dice rolling and result calculation

### 4. Export Structure (`src/application/dto/mod.rs`)

All Phase 22C DTOs are properly exported for use by services:

```rust
pub use queue_items::{
    // ... other exports
    EnhancedChallengeSuggestion, EnhancedOutcomes, OutcomeDetail,
    // ...
};
```

## What Still Needs Implementation

### Service Layer (Phase 22C - Second Part)

1. **LLM Service Enhancement** - `regenerate_outcome()` method
   - Accepts current suggestion and guidance
   - Calls LLM to regenerate specific outcome(s)
   - Returns new `OutcomeDetail`

2. **OutcomeTriggerService** - Execute outcome triggers
   - Currently has skeleton (see `outcome_trigger_service.rs`)
   - Needs to execute proposed tools
   - Needs to persist state changes to Neo4j

3. **ChallengeResolutionService Enhancement**
   - Wire in OutcomeTriggerService calls
   - Handle ad-hoc challenge registration
   - Apply character skill modifiers

4. **WebSocket Handlers**
   - RegenerateOutcome handler
   - DiscardChallenge handler
   - CreateAdHocChallenge handler

### Current Compilation Status

The codebase has **3 pre-existing compilation errors** unrelated to Phase 22C DTOs:

1. `ChallengeResolutionService` is missing a generic parameter
2. `GameSession` does not implement `SessionManagementPort` trait
3. Related type mismatch in outcome trigger execution

These errors are architectural issues that need to be fixed in Phase 22A/22B but do not affect the DTOs themselves.

## Layer Compliance

All Phase 22C DTOs follow strict hexagonal architecture:

| Layer | Location | Status |
|-------|----------|--------|
| **Domain** | `domain/value_objects/dice.rs` | `DiceRollInput` ✓ |
| **Application** | `application/dto/queue_items.rs` | `EnhancedChallengeSuggestion`, `EnhancedOutcomes`, `OutcomeDetail` ✓ |
| **Application** | `application/dto/mod.rs` | Proper exports ✓ |
| **Infrastructure** | `infrastructure/websocket/messages.rs` | `OutcomeDetailData`, `AdHocOutcomes` ✓ |
| **Infrastructure** | `infrastructure/websocket/messages.rs` | Client/Server message types ✓ |

### Architecture Rules Verification

- [x] Domain types have NO serde attributes (use DTOs)
- [x] Application DTOs have serde(Serialize, Deserialize)
- [x] WebSocket messages are in infrastructure layer
- [x] No infrastructure imports in application DTOs
- [x] Proper separation of concerns (OutcomeDetail vs OutcomeDetailData)

## Summary

**Phase 22C DTOs are COMPLETE and READY for service implementation.**

The data structures for:
- Enhanced challenge suggestions with detailed outcomes
- Outcome regeneration with DM guidance
- Ad-hoc challenge creation without LLM
- Dice roll input (formula and manual)
- Tool execution receipts

...are all properly defined, exported, and follow the hexagonal architecture pattern.

### Next Steps

1. Fix Phase 22A/22B pre-existing compilation errors
2. Implement service layer methods for regeneration and triggers
3. Implement WebSocket handlers for new client messages
4. Wire service calls through application/services/

### Files Modified

- `/home/otto/repos/WrldBldr/Engine/src/application/dto/queue_items.rs` - DTOs already defined
- `/home/otto/repos/WrldBldr/Engine/src/application/dto/mod.rs` - Exports already in place
- `/home/otto/repos/WrldBldr/Engine/src/infrastructure/websocket/messages.rs` - Message types already defined
- `/home/otto/repos/WrldBldr/Engine/src/domain/value_objects/dice.rs` - DiceRollInput already defined

### No Changes Needed

The Phase 22C DTOs are already implemented. No modifications were required during this task.

