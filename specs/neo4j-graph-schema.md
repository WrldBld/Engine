# WrldBldr Neo4j Graph Schema

**Version:** 2.0
**Created:** 2025-12-17
**Status:** APPROVED - Ready for Implementation

This document defines the complete Neo4j graph schema for WrldBldr, including all node types, edge types, properties, and constraints.

---

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [Node Types](#2-node-types)
3. [Edge Types](#3-edge-types)
4. [Constraints & Indexes](#4-constraints--indexes)
5. [Query Patterns](#5-query-patterns)
6. [JSON Blob Exceptions](#6-json-blob-exceptions)
7. [Visual Schema Diagram](#7-visual-schema-diagram)

---

## 1. Design Principles

### 1.1 Graph-First Design

All relationships between entities MUST be modeled as Neo4j edges, not as ID references in properties or JSON blobs.

**DO:**
```cypher
(character:Character)-[:POSSESSES {quantity: 1}]->(item:Item)
```

**DON'T:**
```cypher
(:Character {inventory: "[\"item-uuid-1\", \"item-uuid-2\"]"})
```

### 1.2 Properties on Edges

Relationship metadata belongs on the edge, not in a separate "join" node.

**DO:**
```cypher
(npc:Character)-[:FREQUENTS {
    frequency: "Often",
    time_of_day: "Evening",
    reason: "Meets contacts"
}]->(location:Location)
```

**DON'T:**
```cypher
(npc:Character)-[:HAS_VISIT]->(visit:Visit {frequency: "Often"})-[:TO]->(location:Location)
```

### 1.3 Acceptable JSON Exceptions

JSON properties are acceptable ONLY for:
- Configuration data (not referencing other entities)
- Deeply nested non-relational structures
- Spatial/grid data (2D arrays)
- Form data (character sheets)

See [Section 6](#6-json-blob-exceptions) for the complete list.

### 1.4 Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Node Labels | PascalCase | `Character`, `NarrativeEvent` |
| Edge Types | SCREAMING_SNAKE_CASE | `VIEWS_AS_HELPER`, `CONNECTED_TO` |
| Properties | snake_case | `created_at`, `is_active` |
| IDs | UUID string | `"550e8400-e29b-41d4-a716-446655440000"` |

---

## 2. Node Types

### 2.1 World Structure Nodes

#### World
Top-level container for a campaign setting.

```cypher
(:World {
    id: String!,              // UUID - Primary key
    name: String!,
    description: String,
    rule_system: String,      // JSON - RuleSystemConfig (acceptable)
    created_at: DateTime!,
    updated_at: DateTime!
})
```

#### Act
Story arc corresponding to a Monomyth stage.

```cypher
(:Act {
    id: String!,
    name: String!,
    stage: String!,           // MonomythStage enum value
    description: String,
    order: Integer!           // Ordering within world
})
```

**MonomythStage values:**
- `OrdinaryWorld`, `CallToAdventure`, `RefusalOfTheCall`, `MeetingTheMentor`
- `CrossingTheThreshold`, `TestsAlliesEnemies`, `ApproachToInnermostCave`
- `Ordeal`, `Reward`, `TheRoadBack`, `Resurrection`, `ReturnWithElixir`

#### Goal
Abstract desire target (for Wants that don't target a Character or Item).

```cypher
(:Goal {
    id: String!,
    name: String!,
    description: String
})
```

---

### 2.2 Location Nodes

#### Location
Physical or conceptual place in the world.

```cypher
(:Location {
    id: String!,
    name: String!,
    description: String,
    location_type: String!,   // "Interior", "Exterior", "Abstract"
    backdrop_asset: String,   // Path to backdrop image
    atmosphere: String        // Sensory/emotional description
})
```

#### BackdropRegion
Clickable region within a location's backdrop image.

```cypher
(:BackdropRegion {
    id: String!,
    name: String!,
    bounds_x: Integer!,
    bounds_y: Integer!,
    bounds_width: Integer!,
    bounds_height: Integer!,
    description: String
})
```

#### GridMap
Tactical map for combat (out of MVP scope but included for completeness).

```cypher
(:GridMap {
    id: String!,
    name: String!,
    width: Integer!,
    height: Integer!,
    tile_size: Integer!,
    tilesheet_asset: String!,
    tiles: String!            // JSON - 2D tile array (acceptable)
})
```

---

### 2.3 Character Nodes

#### Character
NPC (Non-Player Character).

```cypher
(:Character {
    id: String!,
    name: String!,
    description: String,
    sprite_asset: String,
    portrait_asset: String,
    base_archetype: String!,      // CampbellArchetype enum
    current_archetype: String!,   // CampbellArchetype enum
    is_alive: Boolean!,
    is_active: Boolean!
})
```

**CampbellArchetype values:**
- `Hero`, `Mentor`, `ThresholdGuardian`, `Herald`
- `Shapeshifter`, `Shadow`, `Trickster`, `Ally`

#### PlayerCharacter
Player-controlled character.

```cypher
(:PlayerCharacter {
    id: String!,
    user_id: String!,
    name: String!,
    description: String,
    sprite_asset: String,
    portrait_asset: String,
    sheet_data: String,       // JSON - CharacterSheetData (acceptable per ADR-001)
    created_at: DateTime!,
    last_active_at: DateTime
})
```

#### Want
Character desire (Actantial model).

```cypher
(:Want {
    id: String!,
    description: String!,
    intensity: Float!,        // 0.0 to 1.0
    known_to_player: Boolean!,
    created_at: DateTime!
})
```

#### Item
Object that can be possessed or interacted with.

```cypher
(:Item {
    id: String!,
    name: String!,
    description: String,
    item_type: String,        // "Weapon", "Consumable", "Key", etc.
    is_unique: Boolean!,
    properties: String        // JSON - item-specific properties (acceptable)
})
```

---

### 2.4 Skill & Challenge Nodes

#### Skill
Ability that can be tested in challenges.

```cypher
(:Skill {
    id: String!,
    name: String!,
    description: String,
    category: String!,        // "Physical", "Mental", "Social", "Magical"
    base_attribute: String,   // Attribute this skill derives from
    is_custom: Boolean!,
    is_hidden: Boolean!,
    order: Integer!
})
```

#### Challenge
Skill check that can be triggered.

```cypher
(:Challenge {
    id: String!,
    name: String!,
    description: String,
    challenge_type: String!,  // "Physical", "Mental", "Social", "Magical"
    difficulty: String!,      // "Easy", "Medium", "Hard", "Very Hard"
    difficulty_class: Integer!,
    outcomes: String!,        // JSON - ChallengeOutcomes (acceptable - complex nested)
    trigger_conditions: String, // JSON - non-entity trigger conditions
    active: Boolean!,
    is_favorite: Boolean!,
    order: Integer!
})
```

---

### 2.5 Scene & Interaction Nodes

#### Scene
Narrative unit within an act.

```cypher
(:Scene {
    id: String!,
    name: String!,
    time_context: String!,    // "Morning", "Afternoon", "Evening", "Night"
    backdrop_override: String, // Override location's backdrop
    directorial_notes: String, // JSON - DirectorialNotes (acceptable)
    entry_conditions: String,  // JSON - non-entity conditions (acceptable)
    order: Integer!
})
```

#### InteractionTemplate
Available action in a scene.

```cypher
(:InteractionTemplate {
    id: String!,
    name: String!,
    interaction_type: String!, // "Dialogue", "Examine", "UseItem", "Custom"
    prompt_hints: String,
    allowed_tools: String,    // JSON - string array (acceptable)
    is_available: Boolean!,
    order: Integer!
})
```

---

### 2.6 Event Nodes

#### NarrativeEvent
DM-designed future event with triggers and outcomes.

```cypher
(:NarrativeEvent {
    id: String!,
    name: String!,
    description: String,
    tags: List<String>,
    trigger_logic: String!,   // "All", "Any", "AtLeast(N)"
    scene_direction: String,
    suggested_opening: String,
    is_active: Boolean!,
    is_triggered: Boolean!,
    triggered_at: DateTime,
    selected_outcome: String,
    is_repeatable: Boolean!,
    trigger_count: Integer!,
    delay_turns: Integer!,
    expires_after_turns: Integer,
    priority: Integer!,
    is_favorite: Boolean!,
    created_at: DateTime!,
    updated_at: DateTime!,
    -- Non-entity triggers/effects remain as JSON
    trigger_conditions_json: String,  // JSON - flag/stat-based triggers
    outcomes_json: String             // JSON - complex outcome definitions
})
```

#### StoryEvent
Immutable record of past gameplay event.

```cypher
(:StoryEvent {
    id: String!,
    event_type: String!,      // StoryEventType discriminator
    event_data: String!,      // JSON - StoryEventType data (acceptable)
    timestamp: DateTime!,
    game_time: String,
    summary: String!,
    is_hidden: Boolean!,
    tags: List<String>
})
```

#### EventChain
Linked sequence of narrative events.

```cypher
(:EventChain {
    id: String!,
    name: String!,
    description: String,
    is_active: Boolean!,
    current_position: Integer!,
    color: String,            // Hex color for UI
    is_favorite: Boolean!,
    created_at: DateTime!,
    updated_at: DateTime!
})
```

---

### 2.7 Session Nodes

#### Session
Active game session.

```cypher
(:Session {
    id: String!,
    name: String,
    status: String!,          // "Waiting", "Active", "Paused", "Ended"
    created_at: DateTime!,
    started_at: DateTime,
    ended_at: DateTime
})
```

---

## 3. Edge Types

### 3.1 World Structure Edges

#### CONTAINS_ACT
World contains Act.

```cypher
(world:World)-[:CONTAINS_ACT {
    order: Integer!           // Position in world's act list
}]->(act:Act)
```

#### CONTAINS_LOCATION
World contains top-level Location, or Location contains child Location.

```cypher
(world:World)-[:CONTAINS_LOCATION]->(location:Location)
(parent:Location)-[:CONTAINS_LOCATION]->(child:Location)
```

#### CONTAINS_CHARACTER
World contains Character.

```cypher
(world:World)-[:CONTAINS_CHARACTER]->(character:Character)
```

#### CONTAINS_SKILL
World contains Skill.

```cypher
(world:World)-[:CONTAINS_SKILL]->(skill:Skill)
```

#### CONTAINS_ITEM
World contains Item definition.

```cypher
(world:World)-[:CONTAINS_ITEM]->(item:Item)
```

#### CONTAINS_CHALLENGE
World contains Challenge.

```cypher
(world:World)-[:CONTAINS_CHALLENGE]->(challenge:Challenge)
```

#### CONTAINS_NARRATIVE_EVENT
World contains NarrativeEvent.

```cypher
(world:World)-[:CONTAINS_NARRATIVE_EVENT]->(event:NarrativeEvent)
```

#### CONTAINS_EVENT_CHAIN
World contains EventChain.

```cypher
(world:World)-[:CONTAINS_EVENT_CHAIN]->(chain:EventChain)
```

#### CONTAINS_GOAL
World contains Goal.

```cypher
(world:World)-[:CONTAINS_GOAL]->(goal:Goal)
```

#### CONTAINS_SCENE
Act contains Scene.

```cypher
(act:Act)-[:CONTAINS_SCENE {
    order: Integer!
}]->(scene:Scene)
```

---

### 3.2 Location Edges

#### CONNECTED_TO
Navigation connection between locations.

```cypher
(from:Location)-[:CONNECTED_TO {
    connection_type: String!,  // "Door", "Path", "Stairs", "Portal", "Road", etc.
    description: String,
    bidirectional: Boolean!,
    travel_time: Integer,      // In game-time units (0 = instant)
    is_locked: Boolean!,
    lock_description: String,  // Description of what's needed to unlock
    unlocked_by_challenge_id: String  // Challenge that unlocks this (if any)
}]->(to:Location)
```

#### HAS_TACTICAL_MAP
Location has tactical grid map.

```cypher
(location:Location)-[:HAS_TACTICAL_MAP]->(map:GridMap)
```

#### HAS_REGION
Location has clickable backdrop region.

```cypher
(location:Location)-[:HAS_REGION]->(region:BackdropRegion)
```

---

### 3.3 Character-Location Edges

#### CURRENTLY_AT
PlayerCharacter's current location.

```cypher
(pc:PlayerCharacter)-[:CURRENTLY_AT]->(location:Location)
```

#### STARTED_AT
PlayerCharacter's starting location.

```cypher
(pc:PlayerCharacter)-[:STARTED_AT]->(location:Location)
```

#### HOME_LOCATION
NPC's home/residence.

```cypher
(npc:Character)-[:HOME_LOCATION {
    description: String       // "Lives in the apartment above the tavern"
}]->(location:Location)
```

#### WORKS_AT
NPC's workplace.

```cypher
(npc:Character)-[:WORKS_AT {
    role: String!,            // "Bartender", "Guard", "Shopkeeper"
    schedule: String          // "Morning", "Evening", "Night", null=always
}]->(location:Location)
```

#### FREQUENTS
NPC regularly visits location.

```cypher
(npc:Character)-[:FREQUENTS {
    frequency: String!,       // "Rarely", "Sometimes", "Often", "Always"
    time_of_day: String!,     // "Morning", "Afternoon", "Evening", "Night", "Any"
    day_of_week: String,      // "Monday", "Weekends", null="Any"
    reason: String,           // "Meets contacts", "Gambling", etc.
    since: DateTime
}]->(location:Location)
```

#### AVOIDS
NPC avoids location.

```cypher
(npc:Character)-[:AVOIDS {
    reason: String!           // "Bad memories", "Enemies there", etc.
}]->(location:Location)
```

---

### 3.4 Character-Character Edges (Social)

#### RELATES_TO
Social/emotional relationship between characters.

```cypher
(from:Character)-[:RELATES_TO {
    relationship_type: String!,  // See RelationshipType values below
    sentiment: Float!,           // -1.0 (hatred) to 1.0 (love)
    known_to_player: Boolean!,
    established_at: DateTime,
    description: String          // "Former comrades in the war"
}]->(to:Character)
```

**RelationshipType values:**
- `Family_Parent`, `Family_Child`, `Family_Sibling`, `Family_Spouse`, `Family_Extended`
- `Romantic`
- `Professional`
- `Rivalry`
- `Friendship`
- `Mentorship`
- `Enmity`
- `Custom`

#### ARCHETYPE_CHANGED
Record of archetype transition (self-referential).

```cypher
(character:Character)-[:ARCHETYPE_CHANGED {
    from_archetype: String!,
    to_archetype: String!,
    reason: String!,
    changed_at: DateTime!,
    order: Integer!           // Sequence number for ordering
}]->(character:Character)
```

---

### 3.5 Character-Item Edges

#### POSSESSES
Character owns/carries item.

```cypher
(character:Character)-[:POSSESSES {
    quantity: Integer!,
    equipped: Boolean!,
    acquired_at: DateTime!,
    acquisition_method: String  // "Found", "Purchased", "Gifted", "Looted", "Crafted", "Inherited"
}]->(item:Item)
```

Also applies to PlayerCharacter:
```cypher
(pc:PlayerCharacter)-[:POSSESSES {...}]->(item:Item)
```

---

### 3.6 Actantial Model Edges

#### HAS_WANT
Character has a desire.

```cypher
(character:Character)-[:HAS_WANT {
    priority: Integer!,       // 1 = primary want, 2 = secondary, etc.
    acquired_at: DateTime!
}]->(want:Want)
```

#### TARGETS
Want targets something (the OBJECT in actantial terms).

```cypher
(want:Want)-[:TARGETS]->(target)
// Where target can be:
//   (:Character) - wants something from/about a person
//   (:Item) - wants a specific item
//   (:Goal) - wants an abstract outcome
```

#### VIEWS_AS_HELPER
Subject sees target as helping their want.

```cypher
(subject:Character)-[:VIEWS_AS_HELPER {
    want_id: String!,         // Which want this relates to
    reason: String!,          // "Saved my life"
    assigned_at: DateTime!
}]->(helper:Character)
```

#### VIEWS_AS_OPPONENT
Subject sees target as opposing their want.

```cypher
(subject:Character)-[:VIEWS_AS_OPPONENT {
    want_id: String!,
    reason: String!,          // "Killed my family"
    assigned_at: DateTime!
}]->(opponent:Character)
```

#### VIEWS_AS_SENDER
Subject sees target as having initiated/motivated their want.

```cypher
(subject:Character)-[:VIEWS_AS_SENDER {
    want_id: String!,
    reason: String!,          // "My father's dying wish"
    assigned_at: DateTime!
}]->(sender:Character)
```

#### VIEWS_AS_RECEIVER
Subject sees target as benefiting from their want's fulfillment.

```cypher
(subject:Character)-[:VIEWS_AS_RECEIVER {
    want_id: String!,
    reason: String!,          // "My village will be safe"
    assigned_at: DateTime!
}]->(receiver:Character)
```

---

### 3.7 Scene & Interaction Edges

#### AT_LOCATION
Scene takes place at location.

```cypher
(scene:Scene)-[:AT_LOCATION]->(location:Location)
```

#### BELONGS_TO_ACT
Scene belongs to act.

```cypher
(scene:Scene)-[:BELONGS_TO_ACT]->(act:Act)
```

#### FEATURES_CHARACTER
Scene features character.

```cypher
(scene:Scene)-[:FEATURES_CHARACTER {
    role: String!,            // "Primary", "Secondary", "Background"
    entrance_cue: String      // "Already present", "Enters midway", etc.
}]->(character:Character)
```

#### BELONGS_TO_SCENE
Interaction belongs to scene.

```cypher
(interaction:InteractionTemplate)-[:BELONGS_TO_SCENE]->(scene:Scene)
```

#### TARGETS_CHARACTER
Interaction targets character.

```cypher
(interaction:InteractionTemplate)-[:TARGETS_CHARACTER]->(character:Character)
```

#### TARGETS_ITEM
Interaction targets item.

```cypher
(interaction:InteractionTemplate)-[:TARGETS_ITEM]->(item:Item)
```

#### TARGETS_REGION
Interaction targets backdrop region.

```cypher
(interaction:InteractionTemplate)-[:TARGETS_REGION]->(region:BackdropRegion)
```

#### REQUIRES_ITEM
Interaction requires item to be available.

```cypher
(interaction:InteractionTemplate)-[:REQUIRES_ITEM {
    consumed: Boolean!        // Item consumed when interaction used?
}]->(item:Item)
```

#### REQUIRES_CHARACTER_PRESENT
Interaction requires character to be present.

```cypher
(interaction:InteractionTemplate)-[:REQUIRES_CHARACTER_PRESENT]->(character:Character)
```

---

### 3.8 Challenge Edges

#### REQUIRES_SKILL
Challenge tests skill.

```cypher
(challenge:Challenge)-[:REQUIRES_SKILL]->(skill:Skill)
```

#### AVAILABLE_AT
Challenge available at location.

```cypher
(challenge:Challenge)-[:AVAILABLE_AT {
    always_available: Boolean!,
    time_restriction: String   // "Morning", "Night", null=any
}]->(location:Location)
```

#### TIED_TO_SCENE
Challenge tied to specific scene.

```cypher
(challenge:Challenge)-[:TIED_TO_SCENE]->(scene:Scene)
```

#### REQUIRES_COMPLETION_OF
Challenge requires another challenge to be completed first.

```cypher
(challenge:Challenge)-[:REQUIRES_COMPLETION_OF {
    success_required: Boolean!  // Must have succeeded, not just attempted?
}]->(prerequisite:Challenge)
```

#### ON_SUCCESS_UNLOCKS
Successful challenge completion unlocks location access.

```cypher
(challenge:Challenge)-[:ON_SUCCESS_UNLOCKS]->(location:Location)
// Note: This sets the CONNECTED_TO edge's is_locked to false
```

---

### 3.9 Narrative Event Edges

#### TIED_TO_LOCATION
Event tied to location.

```cypher
(event:NarrativeEvent)-[:TIED_TO_LOCATION]->(location:Location)
```

#### TIED_TO_SCENE
Event tied to scene.

```cypher
(event:NarrativeEvent)-[:TIED_TO_SCENE]->(scene:Scene)
```

#### BELONGS_TO_ACT
Event belongs to act.

```cypher
(event:NarrativeEvent)-[:BELONGS_TO_ACT]->(act:Act)
```

#### FEATURES_NPC
Event features NPC.

```cypher
(event:NarrativeEvent)-[:FEATURES_NPC]->(character:Character)
```

#### PART_OF_CHAIN
Event is part of event chain.

```cypher
(chain:EventChain)-[:CONTAINS_EVENT {
    position: Integer!,
    is_completed: Boolean!
}]->(event:NarrativeEvent)
```

#### CHAINS_TO
Event chains to another event.

```cypher
(event:NarrativeEvent)-[:CHAINS_TO {
    delay_turns: Integer!,
    chain_reason: String
}]->(next:NarrativeEvent)
```

#### TRIGGERED_BY_ENTERING
Event triggered when PC enters location.

```cypher
(event:NarrativeEvent)-[:TRIGGERED_BY_ENTERING]->(location:Location)
```

#### TRIGGERED_BY_TALKING_TO
Event triggered when PC talks to character.

```cypher
(event:NarrativeEvent)-[:TRIGGERED_BY_TALKING_TO]->(character:Character)
```

#### TRIGGERED_BY_CHALLENGE_COMPLETE
Event triggered when challenge completed.

```cypher
(event:NarrativeEvent)-[:TRIGGERED_BY_CHALLENGE_COMPLETE {
    success_required: Boolean  // null=any, true=success, false=failure
}]->(challenge:Challenge)
```

#### TRIGGERED_BY_EVENT_COMPLETE
Event triggered when another event completes.

```cypher
(event:NarrativeEvent)-[:TRIGGERED_BY_EVENT_COMPLETE {
    outcome_required: String   // Specific outcome name, or null=any
}]->(previous:NarrativeEvent)
```

#### ENABLES_CHALLENGE
Event enables a challenge when triggered.

```cypher
(event:NarrativeEvent)-[:ENABLES_CHALLENGE {
    outcome: String           // Which outcome triggers this, null=any
}]->(challenge:Challenge)
```

#### DISABLES_CHALLENGE
Event disables a challenge when triggered.

```cypher
(event:NarrativeEvent)-[:DISABLES_CHALLENGE {
    outcome: String
}]->(challenge:Challenge)
```

#### EFFECT_GIVES_ITEM
Event gives item to PC.

```cypher
(event:NarrativeEvent)-[:EFFECT_GIVES_ITEM {
    outcome: String,
    quantity: Integer!
}]->(item:Item)
```

#### EFFECT_TRIGGERS_SCENE
Event transitions to scene.

```cypher
(event:NarrativeEvent)-[:EFFECT_TRIGGERS_SCENE {
    outcome: String
}]->(scene:Scene)
```

---

### 3.10 Story Event (Timeline) Edges

#### OCCURRED_IN_SESSION
Story event occurred in session.

```cypher
(event:StoryEvent)-[:OCCURRED_IN_SESSION]->(session:Session)
```

#### BELONGS_TO_WORLD
Story event belongs to world.

```cypher
(event:StoryEvent)-[:BELONGS_TO_WORLD]->(world:World)
```

#### OCCURRED_AT
Story event occurred at location.

```cypher
(event:StoryEvent)-[:OCCURRED_AT]->(location:Location)
```

#### OCCURRED_IN_SCENE
Story event occurred in scene.

```cypher
(event:StoryEvent)-[:OCCURRED_IN_SCENE]->(scene:Scene)
```

#### INVOLVES
Story event involves character.

```cypher
(event:StoryEvent)-[:INVOLVES {
    role: String!             // "Actor", "Target", "Witness", "Speaker", etc.
}]->(character:Character)
```

Also for PlayerCharacter:
```cypher
(event:StoryEvent)-[:INVOLVES {...}]->(pc:PlayerCharacter)
```

#### TRIGGERED_BY_NARRATIVE
Story event was triggered by narrative event.

```cypher
(event:StoryEvent)-[:TRIGGERED_BY_NARRATIVE]->(narrative:NarrativeEvent)
```

#### RECORDS_CHALLENGE
Story event records challenge attempt.

```cypher
(event:StoryEvent)-[:RECORDS_CHALLENGE]->(challenge:Challenge)
```

---

### 3.11 Session Edges

#### USES_WORLD
Session uses world.

```cypher
(session:Session)-[:USES_WORLD]->(world:World)
```

#### HAS_PLAYER_CHARACTER
Session has player character.

```cypher
(session:Session)-[:HAS_PLAYER_CHARACTER]->(pc:PlayerCharacter)
```

#### PLAYS_IN
Player character plays in world.

```cypher
(pc:PlayerCharacter)-[:PLAYS_IN]->(world:World)
```

---

## 4. Constraints & Indexes

### 4.1 Uniqueness Constraints

```cypher
// All entities have unique IDs
CREATE CONSTRAINT world_id IF NOT EXISTS FOR (w:World) REQUIRE w.id IS UNIQUE;
CREATE CONSTRAINT act_id IF NOT EXISTS FOR (a:Act) REQUIRE a.id IS UNIQUE;
CREATE CONSTRAINT location_id IF NOT EXISTS FOR (l:Location) REQUIRE l.id IS UNIQUE;
CREATE CONSTRAINT character_id IF NOT EXISTS FOR (c:Character) REQUIRE c.id IS UNIQUE;
CREATE CONSTRAINT pc_id IF NOT EXISTS FOR (pc:PlayerCharacter) REQUIRE pc.id IS UNIQUE;
CREATE CONSTRAINT want_id IF NOT EXISTS FOR (w:Want) REQUIRE w.id IS UNIQUE;
CREATE CONSTRAINT goal_id IF NOT EXISTS FOR (g:Goal) REQUIRE g.id IS UNIQUE;
CREATE CONSTRAINT item_id IF NOT EXISTS FOR (i:Item) REQUIRE i.id IS UNIQUE;
CREATE CONSTRAINT skill_id IF NOT EXISTS FOR (s:Skill) REQUIRE s.id IS UNIQUE;
CREATE CONSTRAINT challenge_id IF NOT EXISTS FOR (c:Challenge) REQUIRE c.id IS UNIQUE;
CREATE CONSTRAINT scene_id IF NOT EXISTS FOR (s:Scene) REQUIRE s.id IS UNIQUE;
CREATE CONSTRAINT interaction_id IF NOT EXISTS FOR (i:InteractionTemplate) REQUIRE i.id IS UNIQUE;
CREATE CONSTRAINT narrative_event_id IF NOT EXISTS FOR (ne:NarrativeEvent) REQUIRE ne.id IS UNIQUE;
CREATE CONSTRAINT story_event_id IF NOT EXISTS FOR (se:StoryEvent) REQUIRE se.id IS UNIQUE;
CREATE CONSTRAINT event_chain_id IF NOT EXISTS FOR (ec:EventChain) REQUIRE ec.id IS UNIQUE;
CREATE CONSTRAINT session_id IF NOT EXISTS FOR (s:Session) REQUIRE s.id IS UNIQUE;
CREATE CONSTRAINT region_id IF NOT EXISTS FOR (r:BackdropRegion) REQUIRE r.id IS UNIQUE;
CREATE CONSTRAINT grid_map_id IF NOT EXISTS FOR (g:GridMap) REQUIRE g.id IS UNIQUE;
```

### 4.2 Indexes for Common Queries

```cypher
// World membership lookups
CREATE INDEX character_world IF NOT EXISTS FOR ()-[r:CONTAINS_CHARACTER]-() ON (r);
CREATE INDEX location_world IF NOT EXISTS FOR ()-[r:CONTAINS_LOCATION]-() ON (r);

// Name searches
CREATE INDEX character_name IF NOT EXISTS FOR (c:Character) ON (c.name);
CREATE INDEX location_name IF NOT EXISTS FOR (l:Location) ON (l.name);
CREATE INDEX item_name IF NOT EXISTS FOR (i:Item) ON (i.name);

// Status filters
CREATE INDEX character_active IF NOT EXISTS FOR (c:Character) ON (c.is_active);
CREATE INDEX challenge_active IF NOT EXISTS FOR (c:Challenge) ON (c.active);
CREATE INDEX event_active IF NOT EXISTS FOR (ne:NarrativeEvent) ON (ne.is_active);
CREATE INDEX event_triggered IF NOT EXISTS FOR (ne:NarrativeEvent) ON (ne.is_triggered);

// Timeline queries
CREATE INDEX story_event_timestamp IF NOT EXISTS FOR (se:StoryEvent) ON (se.timestamp);

// Archetype queries
CREATE INDEX character_archetype IF NOT EXISTS FOR (c:Character) ON (c.current_archetype);
```

---

## 5. Query Patterns

### 5.1 Get Character's Full Context for LLM

```cypher
// Comprehensive character context query
MATCH (npc:Character {id: $npc_id})

// Wants and targets
OPTIONAL MATCH (npc)-[hw:HAS_WANT]->(want:Want)
OPTIONAL MATCH (want)-[:TARGETS]->(target)

// Actantial relationships
OPTIONAL MATCH (npc)-[vh:VIEWS_AS_HELPER]->(helper:Character)
OPTIONAL MATCH (npc)-[vo:VIEWS_AS_OPPONENT]->(opponent:Character)
OPTIONAL MATCH (npc)-[vs:VIEWS_AS_SENDER]->(sender:Character)
OPTIONAL MATCH (npc)-[vr:VIEWS_AS_RECEIVER]->(receiver:Character)

// Location relationships
OPTIONAL MATCH (npc)-[home:HOME_LOCATION]->(homeLoc:Location)
OPTIONAL MATCH (npc)-[work:WORKS_AT]->(workLoc:Location)
OPTIONAL MATCH (npc)-[freq:FREQUENTS]->(freqLoc:Location)

// Social relationships
OPTIONAL MATCH (npc)-[rel:RELATES_TO]->(other:Character)

// Inventory
OPTIONAL MATCH (npc)-[poss:POSSESSES]->(item:Item)
WHERE poss.equipped = true OR poss.quantity > 0

RETURN npc,
       collect(DISTINCT {
           want: want.description, 
           intensity: want.intensity,
           target: coalesce(target.name, 'abstract goal'),
           priority: hw.priority
       }) as wants,
       collect(DISTINCT {name: helper.name, reason: vh.reason}) as helpers,
       collect(DISTINCT {name: opponent.name, reason: vo.reason}) as opponents,
       collect(DISTINCT {name: sender.name, reason: vs.reason}) as senders,
       collect(DISTINCT {name: receiver.name, reason: vr.reason}) as receivers,
       collect(DISTINCT {location: homeLoc.name, desc: home.description}) as homes,
       collect(DISTINCT {location: workLoc.name, role: work.role, schedule: work.schedule}) as workplaces,
       collect(DISTINCT {location: freqLoc.name, when: freq.time_of_day, frequency: freq.frequency, reason: freq.reason}) as frequents,
       collect(DISTINCT {name: other.name, type: rel.relationship_type, sentiment: rel.sentiment}) as relationships,
       collect(DISTINCT {name: item.name, equipped: poss.equipped, quantity: poss.quantity}) as inventory
```

### 5.2 Find NPCs at Location

```cypher
// Find all NPCs who should be at a location at a given time
MATCH (location:Location {id: $location_id})

// Residents
OPTIONAL MATCH (resident:Character {is_active: true})-[:HOME_LOCATION]->(location)

// Workers (check schedule)
OPTIONAL MATCH (worker:Character {is_active: true})-[w:WORKS_AT]->(location)
WHERE w.schedule IS NULL OR w.schedule = $time_of_day OR w.schedule = 'Any'

// Visitors (check schedule)
OPTIONAL MATCH (visitor:Character {is_active: true})-[f:FREQUENTS]->(location)
WHERE (f.time_of_day = 'Any' OR f.time_of_day = $time_of_day)
  AND (f.day_of_week IS NULL OR f.day_of_week = 'Any' OR f.day_of_week = $day_of_week)

// Exclude those who avoid the location
OPTIONAL MATCH (avoider:Character)-[:AVOIDS]->(location)

WITH location, 
     collect(DISTINCT resident) as residents,
     collect(DISTINCT worker) as workers,
     collect(DISTINCT visitor) as visitors,
     collect(DISTINCT avoider) as avoiders

// Combine and filter
RETURN [npc IN (residents + workers + visitors) WHERE NOT npc IN avoiders | npc] as npcs_present
```

### 5.3 Find Available Challenges at Location

```cypher
MATCH (location:Location {id: $location_id})
MATCH (challenge:Challenge {active: true})-[:AVAILABLE_AT]->(location)

// Check time restrictions
WHERE challenge.AVAILABLE_AT.time_restriction IS NULL 
   OR challenge.AVAILABLE_AT.time_restriction = $time_of_day

// Check prerequisites
OPTIONAL MATCH (challenge)-[:REQUIRES_COMPLETION_OF]->(prereq:Challenge)

WITH challenge, collect(prereq.id) as prereq_ids

// Filter to those with met prerequisites
WHERE all(pid IN prereq_ids WHERE pid IN $completed_challenge_ids)
   OR size(prereq_ids) = 0

// Check not disabled by triggered events
OPTIONAL MATCH (disabler:NarrativeEvent {is_triggered: true})-[:DISABLES_CHALLENGE]->(challenge)
WHERE disabler IS NULL

RETURN challenge
ORDER BY challenge.order
```

### 5.4 Evaluate Narrative Event Triggers

```cypher
// Find narrative events that might trigger based on current context
MATCH (event:NarrativeEvent {is_active: true, is_triggered: false})

// Check location triggers
OPTIONAL MATCH (event)-[:TRIGGERED_BY_ENTERING]->(loc:Location)
WHERE loc.id = $current_location_id

// Check character triggers
OPTIONAL MATCH (event)-[:TRIGGERED_BY_TALKING_TO]->(char:Character)
WHERE char.id = $interacted_character_id

// Check challenge triggers
OPTIONAL MATCH (event)-[:TRIGGERED_BY_CHALLENGE_COMPLETE]->(chal:Challenge)
WHERE chal.id IN $recently_completed_challenges

// Check event triggers
OPTIONAL MATCH (event)-[:TRIGGERED_BY_EVENT_COMPLETE]->(prev:NarrativeEvent)
WHERE prev.id IN $recently_triggered_events

WITH event,
     loc IS NOT NULL as location_trigger_met,
     char IS NOT NULL as character_trigger_met,
     chal IS NOT NULL as challenge_trigger_met,
     prev IS NOT NULL as event_trigger_met

// Evaluate based on trigger_logic
WHERE (event.trigger_logic = 'Any' AND (location_trigger_met OR character_trigger_met OR challenge_trigger_met OR event_trigger_met))
   OR (event.trigger_logic = 'All' AND location_trigger_met AND character_trigger_met)
   // Add more logic as needed

RETURN event
ORDER BY event.priority DESC
```

### 5.5 Get Location Hierarchy

```cypher
// Get full location hierarchy from a starting point
MATCH path = (root:Location)-[:CONTAINS_LOCATION*0..10]->(descendant:Location)
WHERE root.id = $location_id

RETURN nodes(path) as hierarchy, length(path) as depth
ORDER BY depth
```

### 5.6 Find Narrative Tension (Mutual Opponents)

```cypher
// Find pairs of characters who see each other as opponents
MATCH (a:Character)-[:VIEWS_AS_OPPONENT]->(b:Character)-[:VIEWS_AS_OPPONENT]->(a)
WHERE a.id < b.id  // Avoid duplicates

RETURN a.name as character1, b.name as character2
```

### 5.7 Find Potential Allies (Shared Opponents)

```cypher
// Find characters who share opponents
MATCH (char:Character {id: $character_id})-[:VIEWS_AS_OPPONENT]->(enemy:Character)
MATCH (potential:Character)-[:VIEWS_AS_OPPONENT]->(enemy)
WHERE potential.id <> char.id

RETURN potential.name as ally_name, 
       collect(enemy.name) as shared_enemies,
       count(enemy) as enemy_count
ORDER BY enemy_count DESC
```

---

## 6. JSON Blob Exceptions

The following fields remain as JSON because they don't represent entity relationships:

| Node | Field | Reason |
|------|-------|--------|
| World | `rule_system` | Configuration data, not entity reference |
| GridMap | `tiles` | 2D spatial array, not relational |
| PlayerCharacter | `sheet_data` | Form data per ADR-001 |
| Item | `properties` | Item-specific configuration |
| Scene | `directorial_notes` | Nested configuration |
| Scene | `entry_conditions` | Flag/stat-based conditions (non-entity) |
| Challenge | `outcomes` | Complex nested outcome definitions |
| Challenge | `trigger_conditions` | Flag/stat-based triggers (non-entity) |
| NarrativeEvent | `trigger_conditions_json` | Flag/stat-based triggers |
| NarrativeEvent | `outcomes_json` | Complex outcome definitions |
| StoryEvent | `event_data` | Event-type-specific data blob |
| InteractionTemplate | `allowed_tools` | String array |

---

## 7. Visual Schema Diagram

```
                                    ┌─────────────────┐
                                    │      World      │
                                    └────────┬────────┘
                    ┌───────────────────────┼───────────────────────┐
                    │                       │                       │
           CONTAINS_ACT             CONTAINS_LOCATION        CONTAINS_*
                    │                       │                       │
                    ▼                       ▼                       ▼
             ┌──────────┐           ┌──────────────┐         ┌──────────┐
             │   Act    │           │   Location   │         │Character │
             └────┬─────┘           └──────┬───────┘         │  Item    │
                  │                        │                 │  Skill   │
          CONTAINS_SCENE          CONTAINS_LOCATION          │Challenge │
                  │                  CONNECTED_TO            │  Goal    │
                  ▼                    HAS_REGION            │  etc.    │
             ┌──────────┐              HAS_MAP               └──────────┘
             │  Scene   │                  │
             └────┬─────┘                  │
                  │                        │
          AT_LOCATION ─────────────────────┘
          FEATURES_CHARACTER ──────────────────┐
                                               │
                                               ▼
    ┌─────────────────────────────────────────────────────────────────────┐
    │                           Character                                  │
    │  ┌──────────┐  ┌──────────────┐  ┌─────────────┐  ┌──────────────┐  │
    │  │   Want   │  │ Actantial    │  │  Inventory  │  │  Locations   │  │
    │  │          │  │ Relationships│  │             │  │              │  │
    │  │ HAS_WANT │  │ VIEWS_AS_*   │  │  POSSESSES  │  │ HOME_LOCATION│  │
    │  │ TARGETS  │  │              │  │             │  │ WORKS_AT     │  │
    │  └──────────┘  └──────────────┘  └─────────────┘  │ FREQUENTS    │  │
    │                                                    └──────────────┘  │
    └─────────────────────────────────────────────────────────────────────┘
                                        │
                            RELATES_TO (social)
                            ARCHETYPE_CHANGED
                                        │
                                        ▼
    ┌─────────────────────────────────────────────────────────────────────┐
    │                          Challenge                                   │
    │  REQUIRES_SKILL ────────────────────────────────▶ Skill             │
    │  AVAILABLE_AT ──────────────────────────────────▶ Location          │
    │  REQUIRES_COMPLETION_OF ────────────────────────▶ Challenge         │
    │  ON_SUCCESS_UNLOCKS ────────────────────────────▶ Location          │
    └─────────────────────────────────────────────────────────────────────┘
                                        │
                            ENABLES_CHALLENGE
                            DISABLES_CHALLENGE
                                        │
                                        ▼
    ┌─────────────────────────────────────────────────────────────────────┐
    │                       NarrativeEvent                                 │
    │  TIED_TO_LOCATION ──────────────────────────────▶ Location          │
    │  FEATURES_NPC ──────────────────────────────────▶ Character         │
    │  CHAINS_TO ─────────────────────────────────────▶ NarrativeEvent    │
    │  TRIGGERED_BY_* ────────────────────────────────▶ Various           │
    │  EFFECT_* ──────────────────────────────────────▶ Various           │
    └─────────────────────────────────────────────────────────────────────┘
                                        │
                            TRIGGERED_BY_NARRATIVE
                                        │
                                        ▼
    ┌─────────────────────────────────────────────────────────────────────┐
    │                         StoryEvent                                   │
    │  OCCURRED_AT ───────────────────────────────────▶ Location          │
    │  INVOLVES ──────────────────────────────────────▶ Character/PC      │
    │  RECORDS_CHALLENGE ─────────────────────────────▶ Challenge         │
    └─────────────────────────────────────────────────────────────────────┘
```

---

## Revision History

| Date | Version | Change |
|------|---------|--------|
| 2025-12-17 | 2.0 | Complete rewrite for graph-first design |
