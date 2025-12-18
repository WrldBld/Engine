# WrldBldr Neo4j Schema Implementation Strategy

**Version:** 2.0  
**Created:** 2025-12-17  
**Status:** APPROVED - Ready for Implementation

This document defines the implementation order for the graph-first schema defined in `neo4j-graph-schema.md`.

> **Note:** No data migration is required. Development proceeds with fresh worlds created from scratch.

---

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [Implementation Scope](#2-implementation-scope)
3. [Implementation Order](#3-implementation-order)
4. [Phase Details](#4-phase-details)

---

## 1. Current State Analysis

### 1.1 Current Entity Structure

| Entity | Status | Changes Needed |
|--------|--------|----------------|
| Character | Exists | Remove `wants`, `inventory` fields; use edges |
| Location | Exists | Remove `parent_id`, `backdrop_regions`, `grid_map_id`; use edges |
| Challenge | Exists | Remove `skill_id`, `scene_id`, `prerequisite_challenges`; use edges |
| NarrativeEvent | Exists | Remove ID fields; use edges for triggers/effects |
| Scene | Exists | Remove ID fields; use edges |
| Skill | Exists | Minimal changes |
| StoryEvent | Exists | Add timeline edges |
| EventChain | Exists | Use edges for event membership |
| World | Exists | Good - already uses containment edges |
| PlayerCharacter | Exists | Add location edges |
| Relationship | Exists | **Already uses edges** - good model |
| Want | Value Object | Promote to entity node |
| Goal | New | Create as node type |
| BackdropRegion | Embedded | Extract to node type |
| Item | May exist | Ensure node type exists |
| Session | May exist | Ensure node type exists |

### 1.2 Repository Status

The `Neo4jRelationshipRepository` already uses edges correctly - this is the pattern to follow for all other repositories.

---

## 2. Implementation Scope

### 2.1 Fields to Remove (Replace with Edges)

| Entity | Remove Field | Replace With Edge |
|--------|--------------|-------------------|
| Character | `wants: Vec<Want>` | `HAS_WANT` → Want |
| Character | `inventory: Vec<ItemId>` | `POSSESSES` → Item |
| Location | `parent_id: Option<LocationId>` | `CONTAINS_LOCATION` |
| Location | `backdrop_regions: Vec<BackdropRegion>` | `HAS_REGION` → BackdropRegion |
| Location | `grid_map_id: Option<GridMapId>` | `HAS_TACTICAL_MAP` → GridMap |
| Challenge | `skill_id: SkillId` | `REQUIRES_SKILL` → Skill |
| Challenge | `scene_id: Option<SceneId>` | `TIED_TO_SCENE` → Scene |
| Challenge | `prerequisite_challenges: Vec<ChallengeId>` | `REQUIRES_COMPLETION_OF` → Challenge |
| NarrativeEvent | `featured_npcs: Vec<CharacterId>` | `FEATURES_NPC` → Character |
| NarrativeEvent | `scene_id`, `location_id`, `act_id` | Various edges |
| NarrativeEvent | `chain_id`, `chain_position` | `CONTAINS_EVENT` from EventChain |

### 2.2 New Node Types

| Node | Purpose |
|------|---------|
| Want | Character desires (promoted from value object) |
| Goal | Abstract want targets |
| BackdropRegion | Clickable regions (extracted from Location) |
| Item | Inventory items |
| Session | Game session tracking |

### 2.3 New Edge Types

| Edge | Purpose |
|------|---------|
| `VIEWS_AS_HELPER/OPPONENT/SENDER/RECEIVER` | Actantial model |
| `TARGETS` | Want → target |
| `CONNECTED_TO` | Location navigation |
| `HOME_LOCATION`, `WORKS_AT`, `FREQUENTS`, `AVOIDS` | Character-Location |
| `AVAILABLE_AT`, `ON_SUCCESS_UNLOCKS` | Challenge-Location |
| `TRIGGERED_BY_*`, `EFFECT_*` | NarrativeEvent triggers/effects |
| `CHAINS_TO` | Event chaining |

---

## 3. Implementation Order

### 3.1 Dependency Graph

```
Level 0 (No dependencies):
  World, Skill, Item, Goal

Level 1 (Depends on Level 0):
  Location, Character, Act

Level 2 (Depends on Level 1):
  Want, BackdropRegion, GridMap, Scene, PlayerCharacter
  Location connections, Character-Location edges, Character-Item edges

Level 3 (Depends on Level 2):
  Actantial edges, Challenge, InteractionTemplate

Level 4 (Depends on Level 3):
  NarrativeEvent, EventChain

Level 5 (Depends on Level 4):
  StoryEvent, Session
```

### 3.2 Implementation Phases

| Phase | Focus | Est. Time | Status |
|-------|-------|-----------|--------|
| 0.A | Core Schema Design | - | ✅ Complete |
| 0.B | Location System | 1-2 days | ✅ Complete |
| 0.C | Character System | 2-3 days | ✅ Complete |
| 0.D | Scene & Interaction System | 1 day | ✅ Complete |
| 0.E | Challenge System | 1-2 days | ✅ Complete |
| 0.F | Narrative Event System | 2 days | ✅ Complete |
| 0.G | Story Event System | 1 day | ✅ Complete |
| 0.H | Service Updates | 1-2 days | ✅ Complete |

---

## 4. Phase Details

### 4.1 Phase 0.B: Location System

**Goal:** Location hierarchy, connections, and regions as edges.

**Domain Changes:**
```rust
// Location: REMOVE these fields
- parent_id: Option<LocationId>
- backdrop_regions: Vec<BackdropRegion>
- grid_map_id: Option<GridMapId>

// Location: ADD
+ atmosphere: Option<String>

// NEW entity: BackdropRegion
pub struct BackdropRegion {
    pub id: BackdropRegionId,
    pub name: String,
    pub bounds_x: u32,
    pub bounds_y: u32,
    pub bounds_width: u32,
    pub bounds_height: u32,
    pub description: Option<String>,
}
```

**Repository Methods:**
- `create_backdrop_region(location_id, region)`
- `get_backdrop_regions(location_id)`
- `create_connection(from_id, to_id, props)`
- `get_connections(location_id)`
- `get_children(location_id)` - via CONTAINS_LOCATION
- `get_parent(location_id)`
- `set_parent(child_id, parent_id)`

---

### 4.2 Phase 0.C: Character System

**Goal:** Wants as nodes, Actantial edges, inventory edges, character-location edges.

**Domain Changes:**
```rust
// Character: REMOVE these fields
- wants: Vec<Want>
- inventory: Vec<ItemId>

// Want: Promote to entity
pub struct Want {
    pub id: WantId,
    pub description: String,
    pub intensity: f32,
    pub known_to_player: bool,
    pub created_at: DateTime<Utc>,
}
// Target handled by TARGETS edge, not embedded

// NEW: Goal entity for abstract targets
pub struct Goal {
    pub id: GoalId,
    pub name: String,
    pub description: Option<String>,
}

// NEW value objects for character-location
pub struct CharacterLocationInfo {
    pub home: Option<HomeLocation>,
    pub work: Option<WorkLocation>,
    pub frequents: Vec<FrequentedLocation>,
    pub avoids: Vec<AvoidedLocation>,
}
```

**Repository Methods:**
- `create_want(character_id, want)` + `HAS_WANT` edge
- `get_wants(character_id)`
- `set_want_target(want_id, target_node_id)`
- `add_actantial_view(subject_id, role, target_id, want_id, reason)`
- `get_actantial_views(character_id)`
- `add_inventory_item(character_id, item_id, quantity, equipped)`
- `get_inventory(character_id)`
- `set_home_location(character_id, location_id, description)`
- `set_work_location(character_id, location_id, role, schedule)`
- `add_frequented_location(character_id, location_id, ...)`
- `add_avoided_location(character_id, location_id, reason)`
- `get_npcs_at_location(location_id, time_of_day)` - graph query

---

### 4.3 Phase 0.D: Scene & Interaction System

**Goal:** Scene-Location, Scene-Character edges.

**Domain Changes:**
```rust
// Scene: Use edges instead of ID fields
- location_id  // use AT_LOCATION edge
// Keep: time_context, directorial_notes, entry_conditions, order

// InteractionTemplate: Use edges
// TARGETS_CHARACTER, TARGETS_ITEM, TARGETS_REGION edges
```

**Repository Methods:**
- `set_scene_location(scene_id, location_id)`
- `add_scene_character(scene_id, character_id, role, entrance_cue)`
- `get_scene_characters(scene_id)`

---

### 4.4 Phase 0.E: Challenge System

**Goal:** Challenge edges for skill, location, prerequisites, unlocks.

**Domain Changes:**
```rust
// Challenge: REMOVE these fields
- skill_id: SkillId
- scene_id: Option<SceneId>
- prerequisite_challenges: Vec<ChallengeId>

// ADD via edges:
// REQUIRES_SKILL, TIED_TO_SCENE, REQUIRES_COMPLETION_OF
// AVAILABLE_AT, ON_SUCCESS_UNLOCKS (new)
```

**Repository Methods:**
- `set_challenge_skill(challenge_id, skill_id)`
- `set_challenge_scene(challenge_id, scene_id)`
- `add_prerequisite(challenge_id, prereq_id, success_required)`
- `get_prerequisites(challenge_id)`
- `add_challenge_location(challenge_id, location_id, always_available, time_restriction)`
- `set_unlock_location(challenge_id, location_id)`
- `get_available_challenges(location_id, time_of_day, completed_ids)`

---

### 4.5 Phase 0.F: Narrative Event System ✅ COMPLETE

**Goal:** Convert NarrativeEvent associations to graph edges.

**Status:** Completed 2025-12-17

**Domain Changes (Implemented):**
```rust
// NarrativeEvent: REMOVED these embedded fields
- featured_npcs: Vec<CharacterId>  // → FEATURES_NPC edge
- scene_id: Option<SceneId>        // → TIED_TO_SCENE edge
- location_id: Option<LocationId>  // → TIED_TO_LOCATION edge
- act_id: Option<ActId>            // → BELONGS_TO_ACT edge
- chain_id: Option<EventChainId>   // → CONTAINS_EVENT edge (from EventChain)
- chain_position: Option<u32>      // → CONTAINS_EVENT edge property

// NEW edge support structs added:
+ FeaturedNpc { character_id, role: Option<String> }
+ EventChainMembership { chain_id, position, is_completed }

// Keep JSON for non-entity triggers/effects (as designed):
- trigger_conditions (flag/stat triggers - complex nested structures)
- outcomes (complex outcome definitions with effects)
```

**Repository Methods (Implemented in NarrativeEventRepositoryPort):**

TIED_TO_SCENE edges:
- `tie_to_scene(event_id, scene_id)` - Create edge
- `get_tied_scene(event_id)` - Get associated scene
- `untie_from_scene(event_id)` - Remove edge

TIED_TO_LOCATION edges:
- `tie_to_location(event_id, location_id)` - Create edge
- `get_tied_location(event_id)` - Get associated location
- `untie_from_location(event_id)` - Remove edge

BELONGS_TO_ACT edges:
- `assign_to_act(event_id, act_id)` - Create edge
- `get_act(event_id)` - Get associated act
- `unassign_from_act(event_id)` - Remove edge

FEATURES_NPC edges:
- `add_featured_npc(event_id, FeaturedNpc)` - Create edge with optional role
- `get_featured_npcs(event_id)` - Get all featured NPCs
- `remove_featured_npc(event_id, character_id)` - Remove edge
- `update_featured_npc_role(event_id, character_id, role)` - Update edge property

Chain membership (CONTAINS_EVENT edge from EventChain side):
- `get_chain_memberships(event_id)` - Query chains this event belongs to

Query by relationship:
- `list_by_scene(scene_id)` - Events tied to a scene
- `list_by_location(location_id)` - Events tied to a location
- `list_by_act(act_id)` - Events belonging to an act
- `list_by_featured_npc(character_id)` - Events featuring an NPC

**DTO Changes:**
- `NarrativeEventResponseDto` - Lightweight list view (no edge data)
- `NarrativeEventDetailResponseDto` - Full view with edge data
- `ChainMembershipDto`, `FeaturedNpcDto` - Edge data DTOs

**Files Modified:**
- `src/domain/entities/narrative_event.rs` - Entity changes
- `src/domain/entities/mod.rs` - Export new types
- `src/application/ports/outbound/repository_port.rs` - Port expansion
- `src/infrastructure/persistence/narrative_event_repository.rs` - Implementation
- `src/application/dto/narrative_event.rs` - DTO updates
- `src/infrastructure/websocket_helpers.rs` - LLM context compatibility

**Note:** Entity-based triggers (TRIGGERED_BY_ENTERING, etc.) were NOT implemented in this phase.
The trigger_conditions JSON field already handles these via NarrativeTriggerType enum variants.
These edge-based triggers can be added in a future phase if graph-based trigger evaluation is needed.

---

### 4.6 Phase 0.G: Story Event System ✅ COMPLETE

**Goal:** Timeline edges for StoryEvent.

**Status:** Completed 2025-12-17

**Domain Changes (Implemented):**
```rust
// StoryEvent: REMOVED these embedded fields
- session_id: SessionId           // → OCCURRED_IN_SESSION edge
- scene_id: Option<SceneId>       // → OCCURRED_IN_SCENE edge
- location_id: Option<LocationId> // → OCCURRED_AT edge
- involved_characters: Vec<CharacterId> // → INVOLVES edge
- triggered_by: Option<NarrativeEventId> // → TRIGGERED_BY_NARRATIVE edge

// NEW edge support struct:
+ InvolvedCharacter { character_id, role: String }
  // Roles: "Actor", "Target", "Speaker", "Witness"

// Keep JSON for StoryEventType (complex discriminated union)
```

**Repository Methods (Implemented in StoryEventRepositoryPort):**

OCCURRED_IN_SESSION edge:
- `set_session(event_id, session_id)` - Create edge
- `get_session(event_id)` - Get associated session

OCCURRED_AT edge (Location):
- `set_location(event_id, location_id)` - Create edge
- `get_location(event_id)` - Get associated location
- `remove_location(event_id)` - Remove edge

OCCURRED_IN_SCENE edge:
- `set_scene(event_id, scene_id)` - Create edge
- `get_scene(event_id)` - Get associated scene
- `remove_scene(event_id)` - Remove edge

INVOLVES edge (with role):
- `add_involved_character(event_id, InvolvedCharacter)` - Create edge with role
- `get_involved_characters(event_id)` - Get all involved characters
- `remove_involved_character(event_id, character_id)` - Remove edge

TRIGGERED_BY_NARRATIVE edge:
- `set_triggered_by(event_id, narrative_event_id)` - Create edge
- `get_triggered_by(event_id)` - Get triggering narrative event
- `remove_triggered_by(event_id)` - Remove edge

RECORDS_CHALLENGE edge:
- `set_recorded_challenge(event_id, challenge_id)` - Create edge
- `get_recorded_challenge(event_id)` - Get recorded challenge
- `remove_recorded_challenge(event_id)` - Remove edge

Query by relationship:
- `list_by_narrative_event(narrative_event_id)` - Events triggered by NE
- `list_by_challenge(challenge_id)` - Events recording a challenge
- `list_by_scene(scene_id)` - Events in a scene

**DTO Changes:**
- `StoryEventResponseDto` - Now has `with_edges()` constructor
- `InvolvedCharacterResponseDto` - Edge data DTO

**Files Modified:**
- `src/domain/entities/story_event.rs` - Entity changes
- `src/domain/entities/mod.rs` - Export InvolvedCharacter
- `src/application/ports/outbound/repository_port.rs` - Port expansion (20 methods)
- `src/infrastructure/persistence/story_event_repository.rs` - Implementation
- `src/application/services/story_event_service.rs` - Uses edge methods
- `src/application/dto/story_event.rs` - DTO updates

---

### 4.7 Phase 0.H: Service Updates ✅ COMPLETE

**Goal:** Update services to use graph queries.

**Status:** Completed 2025-12-17

**Service Updates:**

| Service | Status | Changes Made |
|---------|--------|--------------|
| CharacterService | ✅ | Uses edge methods for wants; location/inventory deferred |
| LocationService | ✅ | Already uses all edge methods (no changes needed) |
| ChallengeService | ✅ | Already uses all edge methods (no changes needed) |
| SceneService | ✅ | Updated create/get/add/remove to use edges |
| NarrativeEventService | ✅ | Added 18 edge methods for scene/location/act/NPC/chain |
| StoryEventService | ✅ | Done in 0.G - all record_* methods use edges |

**SceneService Changes:**
- `create_scene()` - Now creates AT_LOCATION and FEATURES_CHARACTER edges after node creation
- `get_scene_with_relations()` - Fetches location via edge (fallback to entity field)
- `add_character()` - Uses `add_featured_character()` edge method
- `remove_character()` - Uses `remove_featured_character()` edge method
- `update_featured_characters()` - Diff-based edge updates

**NarrativeEventService Changes:**
Added service methods that delegate to repository edge methods:
- Scene tie: `tie_to_scene()`, `get_tied_scene()`, `untie_from_scene()`
- Location tie: `tie_to_location()`, `get_tied_location()`, `untie_from_location()`
- Act: `assign_to_act()`, `get_act()`, `unassign_from_act()`
- NPCs: `add_featured_npc()`, `get_featured_npcs()`, `remove_featured_npc()`, `update_featured_npc_role()`
- Chains: `get_chain_memberships()`
- Queries: `list_by_scene()`, `list_by_location()`, `list_by_act()`, `list_by_featured_npc()`

**Files Modified:**
- `src/application/services/scene_service.rs` - Edge method usage
- `src/application/services/narrative_event_service.rs` - 18 new service methods

---

## 5. Phase 0 Complete Summary

Phase 0 (Neo4j Data Model Foundation) is now complete. All entity relationships that were previously stored as embedded JSON fields are now stored as proper graph edges.

### Key Achievements:
1. **Location System**: Hierarchy, connections, regions, grid maps all via edges
2. **Character System**: Wants, inventory, actantial views, location associations via edges
3. **Scene System**: Location and featured characters via edges
4. **Challenge System**: Skill requirements, prerequisites, availability via edges
5. **Narrative Event System**: Scene/location/act ties, featured NPCs, chain membership via edges
6. **Story Event System**: Session, location, scene, involved characters, triggers, challenges via edges

### Design Decisions:
- Complex nested data (triggers, outcomes, StoryEventType) kept as JSON - these aren't entity relationships
- DTOs have `with_edges()` constructors for detail views, simple `from()` for list views
- Services create edges after creating nodes (graph-first pattern)
- Fallback to embedded fields during migration for backward compatibility

---

---

## 6. Phase 1: LLM Context Enhancement ✅ COMPLETE

**Status:** Completed 2025-12-17

### Implementation Summary

| Task | Status | Files Created/Modified |
|------|--------|------------------------|
| Context Budget Config | ✅ | `domain/value_objects/context_budget.rs` |
| Token Counting | ✅ | Added to `context_budget.rs` |
| LLM Context Service | ✅ | `application/services/llm_context_service.rs` |
| Summarization | ✅ | Added to `llm_context_service.rs` |
| Prompt Builder Updates | ✅ | `application/services/llm/prompt_builder.rs` |

### Key Components

**ContextBudgetConfig:**
- Per-category token limits (scene, character, challenges, etc.)
- Default configuration with 8000 total token budget
- Added to `AppSettings` for persistence

**TokenCounter:**
- Three counting methods: `CharacterApprox`, `WordApprox`, `Hybrid`
- `llama_tuned()` preset for Ollama models
- Methods: `count_tokens()`, `exceeds_token_budget()`, `truncate_to_budget()`

**LLMContextService:**
- Builds context from graph traversal (not embedded JSON)
- Methods: `build_scene_context()`, `build_character_context()`, `build_challenges_context()`, `build_narrative_events_context()`
- Uses repository ports for data access

**Summarization System:**
- `SummarizationPrompts` - Category-specific summarization prompts
- `SummarizationRequest` - Request struct with prompt building
- `SummarizationPlanner` - Plans which categories need summarization

---

## 7. Phase 2: Trigger System ✅ COMPLETE

**Status:** Completed 2025-12-17

### Implementation Summary

| Task | Status | Files Created/Modified |
|------|--------|------------------------|
| Engine Evaluation Service | ✅ | `application/services/trigger_evaluation_service.rs` |
| LLM Parsing | ✅ | Already in `application/services/llm/mod.rs` |
| Queue & UI | ✅ | Uses existing `ApprovalItem` with `NarrativeEventSuggestionInfo` |
| Effect Execution | ✅ | `application/services/event_effect_executor.rs` |

### Key Components

**TriggerEvaluationService:**
- Evaluates narrative event triggers against game state
- `GameStateSnapshot` - Captures current state (location, flags, inventory, etc.)
- `TriggeredEventCandidate` - Encapsulates triggered event with source (Engine/LLM/DmManual)
- `evaluate_triggers()` - Checks all active events for satisfied triggers
- `build_game_state_snapshot()` - Builds state from repositories
- `create_llm_suggestion()` - Validates LLM-suggested triggers

**EventEffectExecutor:**
- Executes all `EventEffect` types from narrative event outcomes
- Supported effects: SetFlag, EnableChallenge, DisableChallenge, EnableEvent, DisableEvent, RevealInformation, GiveItem, TakeItem, ModifyRelationship, ModifyStat, TriggerScene, StartCombat, AddReward, Custom
- Uses repository ports for database changes
- Logs effects to conversation history for DM awareness

**Integration Points:**
- Services added to `GameServices` struct in `infrastructure/state/game_services.rs`
- Wired up in `AppState::new()` with dependency injection
- Ready for integration with game loop and approval flow

### Architecture Notes

- Both services follow hexagonal architecture (depend on repository ports)
- LLM suggestion parsing was already implemented - just needed validation
- `NarrativeEventSuggestionInfo` flows through existing approval queue
- `NarrativeEventApprovalService` handles DM decisions

---

## Revision History

| Date | Version | Change |
|------|---------|--------|
| 2025-12-17 | 1.0 | Initial migration strategy |
| 2025-12-17 | 2.0 | Simplified to implementation-only (no data migration) |
| 2025-12-17 | 2.1 | Phase 0.F (Narrative Event System) completed |
| 2025-12-17 | 2.2 | Phase 0.G (Story Event System) completed |
| 2025-12-17 | 2.3 | Phase 0.H (Service Updates) completed - Phase 0 COMPLETE |
| 2025-12-17 | 2.4 | Phase 1 (LLM Context) completed |
| 2025-12-17 | 2.5 | Phase 2 (Trigger System) completed |
