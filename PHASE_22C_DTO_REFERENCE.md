# Phase 22C - Enhanced Challenge Suggestions DTOs - Complete Reference

**Date**: 2025-12-16
**Status**: COMPLETE
**Layer**: Application & Infrastructure (DTOs)

## Overview

Phase 22C DTOs enable the LLM to suggest skill challenges with pre-defined outcomes and proposed tool effects. These DTOs flow through the system in this path:

```
LLM → EnhancedChallengeSuggestion (app/dto)
    → ApprovalItem (queued for DM)
    → WebSocket message (OutcomeRegenerated/ChallengeDiscarded/AdHocChallengeCreated)
    → Player UI updates
```

## Complete DTO Reference

### Layer: Application / DTO

**File**: `src/application/dto/queue_items.rs`

#### 1. EnhancedChallengeSuggestion

The primary DTO for LLM-suggested challenges with full outcome details.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedChallengeSuggestion {
    /// Optional reference to a predefined challenge (None for ad-hoc)
    pub challenge_id: Option<String>,

    /// Name of the challenge (e.g., "Persuasion Check", "Stealth Attempt")
    pub challenge_name: String,

    /// The skill being tested (e.g., "Persuasion", "Stealth", "Athletics")
    pub skill_name: String,

    /// Difficulty display (e.g., "DC 15", "Moderate", "70%")
    pub difficulty_display: String,

    /// What the NPC says before the challenge
    pub npc_reply: String,

    /// Detailed outcomes for each result tier
    pub outcomes: EnhancedOutcomes,

    /// Internal LLM reasoning (shown to DM only)
    pub reasoning: String,
}
```

**Usage Context**:
- Stored in `ApprovalItem.challenge_suggestion: Option<EnhancedChallengeSuggestion>`
- Sent to DM for review and approval
- Supports both predefined challenges (challenge_id Some) and ad-hoc (challenge_id None)
- Includes NPC dialogue for context

**Example JSON**:
```json
{
  "challenge_id": null,
  "challenge_name": "Persuade the Guard",
  "skill_name": "Persuasion",
  "difficulty_display": "DC 13",
  "npc_reply": "The guard eyes you suspiciously. You'll need to convince them to let you pass.",
  "outcomes": {
    "critical_success": {
      "flavor_text": "Your smooth words completely win over the guard, who even becomes sympathetic to your cause.",
      "scene_direction": "Guard steps aside and waves you through enthusiastically.",
      "proposed_tools": [
        {
          "id": "tool_1",
          "name": "modify_npc_opinion",
          "description": "Increase guard opinion by +20",
          "arguments": {"npc_id": "guard_1", "change": 20}
        }
      ]
    },
    "success": {
      "flavor_text": "Your argument is reasonable and the guard, though cautious, allows you to pass.",
      "scene_direction": "Guard steps aside.",
      "proposed_tools": []
    },
    "failure": {
      "flavor_text": "The guard isn't convinced and refuses to let you pass. You'll need to find another way.",
      "scene_direction": "Guard stands firm, blocking the passage.",
      "proposed_tools": []
    },
    "critical_failure": {
      "flavor_text": "Your words anger the guard, who becomes hostile and threatens to arrest you.",
      "scene_direction": "Guard draws weapon.",
      "proposed_tools": [
        {
          "id": "tool_2",
          "name": "modify_npc_opinion",
          "description": "Decrease guard opinion by -30",
          "arguments": {"npc_id": "guard_1", "change": -30}
        }
      ]
    }
  },
  "reasoning": "The guard is cautious but reasonable. A high persuasion roll could win him over entirely, while a critical failure would anger him."
}
```

---

#### 2. EnhancedOutcomes

Defines outcomes for all four result tiers: critical success, success, failure, critical failure.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedOutcomes {
    /// Outcome for natural 20 or exceptional success (optional)
    #[serde(default)]
    pub critical_success: Option<OutcomeDetail>,

    /// Outcome for meeting or exceeding the DC
    pub success: OutcomeDetail,

    /// Outcome for failing to meet the DC
    pub failure: OutcomeDetail,

    /// Outcome for natural 1 or catastrophic failure (optional)
    #[serde(default)]
    pub critical_failure: Option<OutcomeDetail>,
}
```

**Notes**:
- `success` and `failure` are required
- `critical_success` and `critical_failure` are optional
- Allows for simpler 2-tier or more complex 4-tier challenge designs
- Each outcome tier contains narrative and system effects

**Structure Diagram**:
```
EnhancedOutcomes
├── critical_success: Option<OutcomeDetail>  [Optional]
├── success: OutcomeDetail                   [Required]
├── failure: OutcomeDetail                   [Required]
└── critical_failure: Option<OutcomeDetail>  [Optional]
```

---

#### 3. OutcomeDetail

Detailed information about what happens in a specific outcome tier.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDetail {
    /// Narrative flavor text describing what happens
    pub flavor_text: String,

    /// Scene direction (what actions/changes occur)
    pub scene_direction: String,

    /// Tool calls that would be executed for this outcome
    #[serde(default)]
    pub proposed_tools: Vec<ProposedToolInfo>,
}
```

**Fields**:
- **flavor_text**: The narrative description read to all players (e.g., "The guard steps aside...")
- **scene_direction**: Behind-the-scenes system effects (e.g., "Guard stands at passage entrance")
- **proposed_tools**: List of game state changes to execute (e.g., modify relationship, give item)

**Narrative vs System**:
- **Flavor text** = What players hear/see
- **Scene direction** = What the system does (for logging/tracking)
- **Proposed tools** = Automatic game state modifications (via tool execution)

---

### Layer: Infrastructure / WebSocket Messages

**File**: `src/infrastructure/websocket/messages.rs`

#### 1. OutcomeDetailData

WebSocket representation of outcome details (mirrors OutcomeDetail from application layer).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDetailData {
    /// Narrative flavor text
    pub flavor_text: String,

    /// Scene direction (what happens)
    pub scene_direction: String,

    /// Proposed tool calls for this outcome
    #[serde(default)]
    pub proposed_tools: Vec<ProposedToolInfo>,
}
```

**Why separate from OutcomeDetail?**
- Maintains layer separation (application DTOs vs WebSocket wire format)
- Allows different serialization strategies if needed
- Clear boundary between application and infrastructure layers
- May have different evolution paths for backward compatibility

---

#### 2. AdHocOutcomes

DM-provided outcomes when creating challenges without LLM.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdHocOutcomes {
    /// What happens on success
    pub success: String,

    /// What happens on failure
    pub failure: String,

    /// Optional critical success outcome
    #[serde(default)]
    pub critical_success: Option<String>,

    /// Optional critical failure outcome
    #[serde(default)]
    pub critical_failure: Option<String>,
}
```

**Usage**:
- Sent in `ClientMessage::CreateAdHocChallenge`
- Contains plain text descriptions (no tool calls)
- DM provides the narrative directly, no tool receipts
- Simpler than `EnhancedOutcomes` (strings instead of `OutcomeDetail` objects)

**Example**:
```json
{
  "success": "The merchant agrees to lower the price by 10%.",
  "failure": "The merchant refuses to negotiate and walks away.",
  "critical_success": "The merchant is so impressed that they offer a friend discount.",
  "critical_failure": "The merchant is insulted and demands you leave the shop."
}
```

---

## Client → Server Messages

### RegenerateOutcome

DM requests the LLM regenerate one or more outcomes.

```rust
ClientMessage::RegenerateOutcome {
    /// The approval request ID this relates to
    request_id: String,

    /// Which outcome to regenerate ("success", "failure", "critical_success", "critical_failure")
    /// If None, regenerate all outcomes
    outcome_type: Option<String>,

    /// Optional guidance for regeneration
    guidance: Option<String>,
}
```

**Examples**:

Regenerate single outcome:
```json
{
  "type": "RegenerateOutcome",
  "request_id": "req_12345",
  "outcome_type": "success",
  "guidance": "Make it more dramatic"
}
```

Regenerate all outcomes:
```json
{
  "type": "RegenerateOutcome",
  "request_id": "req_12345",
  "outcome_type": null,
  "guidance": "This challenge is for a merchant negotiation, be more mercantile in tone"
}
```

---

### DiscardChallenge

DM rejects the challenge suggestion entirely.

```rust
ClientMessage::DiscardChallenge {
    /// The approval request ID containing the challenge
    request_id: String,

    /// Feedback on why discarding (optional, for LLM learning)
    feedback: Option<String>,
}
```

**Examples**:

Discard with feedback (may trigger regeneration):
```json
{
  "type": "DiscardChallenge",
  "request_id": "req_12345",
  "feedback": "Combat isn't appropriate here, suggest a social challenge instead"
}
```

Discard without regeneration:
```json
{
  "type": "DiscardChallenge",
  "request_id": "req_12345",
  "feedback": null
}
```

---

### CreateAdHocChallenge

DM creates a challenge without LLM involvement.

```rust
ClientMessage::CreateAdHocChallenge {
    /// Name of the challenge
    challenge_name: String,

    /// Skill being tested
    skill_name: String,

    /// Difficulty display (e.g., "DC 15", "Hard")
    difficulty: String,

    /// Target PC ID
    target_pc_id: String,

    /// Outcome descriptions
    outcomes: AdHocOutcomes,
}
```

**Example**:
```json
{
  "type": "CreateAdHocChallenge",
  "challenge_name": "Jump the Chasm",
  "skill_name": "Athletics",
  "difficulty": "DC 14",
  "target_pc_id": "pc_warrior",
  "outcomes": {
    "success": "You make the jump and land safely on the other side.",
    "failure": "You don't make it. You're falling!",
    "critical_success": "You leap across with impressive acrobatic flair.",
    "critical_failure": "You slip at the edge and tumble down into the darkness."
  }
}
```

---

## Server → Client Messages

### OutcomeRegenerated

Sent when DM's regeneration request completes.

```rust
ServerMessage::OutcomeRegenerated {
    /// The approval request ID this relates to
    request_id: String,

    /// Which outcome was regenerated
    outcome_type: String,

    /// New outcome details
    new_outcome: OutcomeDetailData,
}
```

**Example**:
```json
{
  "type": "OutcomeRegenerated",
  "request_id": "req_12345",
  "outcome_type": "success",
  "new_outcome": {
    "flavor_text": "Your compelling argument wins the merchant over, and they offer an even better deal.",
    "scene_direction": "Merchant smiles and extends hand for a deal.",
    "proposed_tools": [
      {
        "id": "tool_1",
        "name": "give_item",
        "description": "Give discount coupon",
        "arguments": {"item": "merchant_discount_10pct"}
      }
    ]
  }
}
```

---

### ChallengeDiscarded

Confirmation that challenge was discarded.

```rust
ServerMessage::ChallengeDiscarded {
    request_id: String,
}
```

**Simple confirmation**:
```json
{
  "type": "ChallengeDiscarded",
  "request_id": "req_12345"
}
```

---

### AdHocChallengeCreated

Confirmation that ad-hoc challenge was created and sent to player.

```rust
ServerMessage::AdHocChallengeCreated {
    challenge_id: String,
    challenge_name: String,
    target_pc_id: String,
}
```

**Example**:
```json
{
  "type": "AdHocChallengeCreated",
  "challenge_id": "ch_adhoc_001",
  "challenge_name": "Jump the Chasm",
  "target_pc_id": "pc_warrior"
}
```

---

## Domain Layer Value Objects

### DiceRollInput

Represents player input for a challenge roll.

**File**: `src/domain/value_objects/dice.rs`

```rust
pub enum DiceRollInput {
    /// Roll dice using a formula string like "1d20+5"
    Formula(String),

    /// Use a manual result (physical dice roll)
    ManualResult(i32),
}

impl DiceRollInput {
    /// Resolve the input to a roll result
    pub fn resolve(&self) -> Result<DiceRollResult, DiceParseError>

    /// Resolve with an additional modifier (from character skills)
    pub fn resolve_with_modifier(&self, skill_modifier: i32)
        -> Result<DiceRollResult, DiceParseError>
}
```

**Supported Formula Syntax**:
- `1d20` - Roll one d20
- `2d6+3` - Roll two d6s, add 3
- `1d20-1` - Roll d20, subtract 1
- `3d8` - Roll three d8s

**Usage Examples**:

```rust
// Player enters "1d20+5"
let input = DiceRollInput::Formula("1d20+5".to_string());
let result = input.resolve()?;  // Rolls and calculates

// Player enters manual result "18"
let input = DiceRollInput::ManualResult(18);
let result = input.resolve()?;  // Uses 18 directly

// Apply character skill modifier
let input = DiceRollInput::Formula("1d20".to_string());
let result = input.resolve_with_modifier(3)?;  // Adds character's +3 bonus
```

---

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      Skill Challenge Flow                        │
└─────────────────────────────────────────────────────────────────┘

Player Action
    ↓
[LLM Suggests Challenge]
    ↓
EnhancedChallengeSuggestion ← [Application DTO Layer]
    ├── challenge_name: String
    ├── skill_name: String
    ├── difficulty_display: String
    ├── npc_reply: String
    ├── outcomes: EnhancedOutcomes
    │   ├── critical_success: Option<OutcomeDetail>
    │   ├── success: OutcomeDetail
    │   ├── failure: OutcomeDetail
    │   └── critical_failure: Option<OutcomeDetail>
    └── reasoning: String
    ↓
ApprovalItem (queued for DM) ← [Queue Item]
    ├── challenge_suggestion: Option<EnhancedChallengeSuggestion>
    └── ...other fields...
    ↓
[DM Reviews in UI]
    ├─→ [DM Approves] → Challenge Prompt sent to Player
    ├─→ [DM Regenerates Outcome] → RegenerateOutcome message
    │   ↓
    │   OutcomeRegenerated message ← [WebSocket]
    │   └── new_outcome: OutcomeDetailData
    └─→ [DM Discards] → ChallengeDiscarded message
    ↓
Player Receives ChallengePrompt
    ├── challenge_id: String
    ├── challenge_name: String
    ├── skill_name: String
    ├── difficulty_display: String
    ├── character_modifier: i32
    ├── suggested_dice: Option<String>
    └── rule_system_hint: Option<String>
    ↓
Player Enters Dice Roll
    ├── DiceInputType::Formula("1d20+3")
    └── DiceInputType::Manual(18)
    ↓
DiceRollInput ← [Domain VO]
    └── resolve() → DiceRollResult
    ↓
Challenge Resolution
    ├── Determine outcome tier (critical_success, success, failure, critical_failure)
    ├── Execute proposed_tools from OutcomeDetail
    └── Broadcast ChallengeResolved
```

---

## Integration Points

### Where EnhancedChallengeSuggestion is Used

1. **LLM Service** (planned Phase 22C service implementation)
   - Generates `EnhancedChallengeSuggestion` from challenge prompt
   - Returns to caller for queuing

2. **DM Approval Queue**
   - Stores in `ApprovalItem.challenge_suggestion`
   - Retrieved when DM views approval

3. **WebSocket Handler** (planned Phase 22D implementation)
   - Receives `RegenerateOutcome` with current suggestion
   - Passes to LLM service for regeneration
   - Returns regenerated `OutcomeDetail` in `OutcomeRegenerated` message

4. **Ad-hoc Challenge Path** (alternative to LLM)
   - DM creates challenge via `CreateAdHocChallenge`
   - No `EnhancedChallengeSuggestion` needed
   - Uses simplified `AdHocOutcomes` instead

---

## Testing Considerations

### Unit Test Cases

1. **Serialization/Deserialization**
   - Verify `EnhancedChallengeSuggestion` JSON round-trips correctly
   - Test with `critical_success` and `critical_failure` as None
   - Test with nested `ProposedToolInfo` arrays

2. **Optional Fields**
   - EnhancedChallengeSuggestion with `challenge_id: None` (ad-hoc)
   - EnhancedChallengeSuggestion with `challenge_id: Some(...)` (predefined)
   - EnhancedOutcomes with only success/failure (no criticals)

3. **DiceRollInput Resolution**
   - Formula parsing: "1d20+5", "2d6-1", "3d8", etc.
   - Manual input: i32 values
   - Modifier application: formula + skill_modifier

### Integration Test Cases

1. **WebSocket Message Flow**
   - Send RegenerateOutcome → Receive OutcomeRegenerated
   - Send DiscardChallenge → Receive ChallengeDiscarded
   - Send CreateAdHocChallenge → Receive AdHocChallengeCreated

2. **DM Approval Workflow**
   - Challenge appears in DM panel
   - DM can regenerate individual outcomes
   - DM can discard and request non-challenge response
   - UI updates in real-time with new outcomes

---

## Summary

Phase 22C provides comprehensive DTOs for:

1. **LLM-generated challenges** with detailed outcomes and tool effects
2. **Outcome regeneration** with DM guidance
3. **Ad-hoc challenges** created directly by DM without LLM
4. **Dice rolling** with formula parsing and skill modifiers

All DTOs follow hexagonal architecture with proper layer separation and are ready for service implementation.

