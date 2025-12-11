# TTRPG Domain Specification
## Stories, Theatres, and Narrative-Driven Game Management

### Executive Summary

This document defines the domain model for an LLM-integrated TTRPG management system grounded in two foundational narrative theories:

1. **Hero with a Thousand Faces (Campbell's Monomyth)** - Shapes character *archetypal journeys* and narrative progression through universal story patterns
2. **Actantial Modeling (Greimas)** - Defines character *relationships, desires, and functions* within the narrative ecosystem

The model is designed as a directed property graph optimized for Neo4j, with LLM-friendly semantics that enable generative content creation while maintaining narrative coherence.

---

## Part 1: Core Domain Concepts

### 1.1 The Four Pillars

#### Pillar 1: Stories
A **Story** is the narrative container that orchestrates all other elements. It is stateful, evolving through acts and scenes.

**Properties:**
- `id`: Unique identifier
- `title`: Story name
- `premise`: High-level narrative concept
- `theme`: Central thematic exploration
- `status`: CONCEPT → OUTLINE → ACTIVE → COMPLETED
- `currentAct`: Which act we're in (0-3)
- `currentScene`: Which scene within the act
- `createdAt`, `updatedAt`: Temporal tracking

**Semantic Intent:** The story is the *world container* and *state machine*. It's the context that gives meaning to all narrative elements.

---

#### Pillar 2: Theatres
A **Theatre** represents a named location, organization, or thematic realm within the story. It's where scenes unfold and where characters interact.

**Properties:**
- `id`: Unique identifier
- `name`: Theatre name (e.g., "The Court of the Shadow King", "The Library of Lost Memories")
- `description`: Atmospheric description
- `type`: LOCATION | ORGANIZATION | REALM | FACTION
- `atmosphere`: Emotional/sensory tone
- `rulesOfEnchantment`: Special rules or laws that apply here
- `population`: Characters present or associated
- `resources`: What can be found/used here
- `secrets`: Hidden knowledge or dangers
- `accessRequirements`: How to enter/exit

**Semantic Intent:** Theatres are *stages for action*. They constrain what's possible and provide environmental narrative weight.

---

#### Pillar 3: Characters
A **Character** is a conscious agent with desires, powers, and a narrative function. Characters exist across two dimensionalities:

**Monomythic Dimension** (Campbell):
- `archetypeRole`: HERO | MENTOR | HERALD | SHADOW | THRESHOLD_GUARDIAN | ALLY | SHAPE_SHIFTER | TRICKSTER
- `monomythStage`: WHERE in the hero's journey are they positioned?
- `knownActs`: Which parts of the monomyth have they experienced?

**Actantial Dimension** (Greimas):
- `actantFunction`: SUBJECT | OBJECT | SENDER | RECEIVER | HELPER | OPPONENT
- `desire`: What do they want? (can be abstract)
- `power`: What enables them? (physical, magical, social, knowledge-based)
- `obstacle`: What prevents them? (internal or external)

**Base Properties:**
- `id`: Unique identifier
- `name`: Character name
- `essence`: One sentence capturing their core nature
- `appearance`: Physical/visual description
- `background`: Origin story fragment
- `beliefs`: Core convictions
- `flaws`: Character weaknesses/contradictions
- `capabilities`: What they can do
- `status`: INTRODUCED | ACTIVE | DORMANT | DEPARTED | DECEASED
- `alignment`: HOW do they typically act? (moral, practical, emotional dimensions)

**Semantic Intent:** Characters are *agents of change*. Their dual dimensionality (monomythic + actantial) allows us to model both their inner journey and their relational function in the plot.

---

#### Pillar 4: Scenes
A **Scene** is a discrete narrative unit where action occurs, typically bounded by location and time.

**Properties:**
- `id`: Unique identifier
- `title`: Scene name
- `description`: What happens here
- `act`: Which act (1-3, or 0 for preamble)
- `sequence`: Order within the act
- `type`: EXPOSITION | CONFRONTATION | REVELATION | PIVOT | CLIMAX | RESOLUTION
- `setting`: Primary theatre
- `participants`: Characters present
- `dramaticTension`: What's at stake?
- `objectives`: What needs to happen?
- `complications`: What could go wrong?
- `resolution`: How it ends (if predetermined) or its state
- `discoveries`: New information revealed
- `consequences`: How it changes the story state

**Semantic Intent:** Scenes are the *atomic unit of narrative action*. They're where generative content gets anchored to specific narrative contexts.

---

### 1.2 The Monomythic Framework (Campbell's Hero's Journey)

The Hero's Journey is divided into **three major phases**, each containing **5-6 stages**:

#### Phase 1: DEPARTURE (Stages 1-5)
Character leaves their ordinary world and commits to the adventure.

| Stage | Name | Narrative Function | Character Arc |
|-------|------|-------------------|----------------|
| 1 | Call to Adventure | Inciting incident; the problem emerges | Hero becomes aware of imbalance |
| 2 | Refusal of the Call | Hesitation; fear of change | Hero's attachment to old world |
| 3 | Supernatural Aid | Mentor appears; magical gift | Hero gains tool or wisdom |
| 4 | Crossing the First Threshold | Point of no return | Hero commits to journey |
| 5 | Belly of the Whale | Full immersion in new world | Hero's old self dissolves |

**Domain Modeling:**
```
Character -[:EXPERIENCES]-> MonomythStage
MonomythStage -[:OCCURS_IN]-> Scene
Character -[:MEETS]-> Mentor (actant function: HELPER)
Character -[:RECEIVES]-> MagicalGift
Scene -[:BELONGS_TO]-> Phase
Phase -[:BELONGS_TO]-> Story
```

---

#### Phase 2: INITIATION (Stages 6-11)
Character faces trials, meets allies/enemies, and undergoes transformation.

| Stage | Name | Narrative Function | Character Arc |
|-------|------|-------------------|----------------|
| 6 | Road of Trials | Multiple challenges test resolve | Hero learns and adapts |
| 7 | Meeting with the Goddess | Deep connection; love, wisdom, or grace | Hero's inner world shifts |
| 8 | Woman as Temptress | Seduction away from quest OR shadow self | Hero faces their desires/fears |
| 9 | Atonement with the Father | Confrontation with ultimate authority | Hero integrates power/accepts responsibility |
| 10 | Apotheosis | Death and rebirth; transcendence | Hero becomes something new |
| 11 | The Ultimate Boon | Achievement; the goal is won | Hero possesses what they sought |

**Domain Modeling:**
```
Character -[:FACES_TRIAL]-> Challenge
Challenge -[:LOCATED_IN]-> Theatre
Character -[:FORMS_BOND]-> AnotherCharacter (with BOND_TYPE and STRENGTH)
Character -[:CONFRONTS]-> Opponent (actant function: OPPONENT)
Character -[:UNDERGOES_TRANSFORMATION]-> Transformation
```

---

#### Phase 3: RETURN (Stages 12-17)
Character brings wisdom back and integrates the new self.

| Stage | Name | Narrative Function | Character Arc |
|-------|------|-------------------|----------------|
| 12 | Refusal of the Return | Desire to stay in new world | Hero's reluctance to leave |
| 13 | The Magic Flight | Escape with the prize; pursuit | Hero demonstrates new power |
| 14 | Rescue from Without | Outside force intervenes | Help from unexpected quarters |
| 15 | The Crossing of the Return Threshold | Reintegration begins | Hero bridges worlds |
| 16 | Master of Two Worlds | Integration complete | Hero operates in both worlds |
| 17 | Freedom to Live | New equilibrium achieved | Hero at peace with transformation |

**Domain Modeling:**
```
Character -[:INTEGRATES]-> Transformation
Transformation -[:AFFECTS]-> Theatre
Character -[:TEACHES]-> AnotherCharacter (knowledge transfer)
Story -[:REACHES]-> Resolution
```

---

### 1.3 The Actantial Framework (Greimas)

The Actantial Model provides six semantic roles that map to narrative functions:

#### The Six Actants

**1. SUBJECT**
- The character whose quest/desire drives the action
- Psychological: Has motivation and agency
- Multiple characters can be SUBJECT in different scenes
- Example: The warrior seeking redemption

**2. OBJECT**
- What the SUBJECT desires (often abstract)
- Can be: a physical thing, a state, knowledge, a person, a power
- Must be *achievable* (otherwise no story tension)
- Example: The warrior seeks "redemption" (abstract) or "the Sacred Sword" (concrete)

**3. SENDER (Destiner)**
- Who/what initiates the quest or provides motivation
- Can be: a person, a prophecy, fate, circumstance, internal drive
- Example: The dying elder tells the warrior, "You alone can restore balance"

**4. RECEIVER (Destinee)**
- Who benefits from the SUBJECT achieving the OBJECT
- Often the SUBJECT themselves, but can be others
- Creates moral weight: "Who benefits?" reveals character values
- Example: The entire kingdom benefits if the warrior succeeds

**5. HELPER (Adjuvant)**
- What/who enables the SUBJECT to pursue the OBJECT
- Can be: tools, allies, powers, knowledge, circumstances
- Example: The ancient sword, the wise mentor, the secret passage

**6. OPPONENT (Adversary)**
- What/who blocks the SUBJECT from achieving the OBJECT
- Can be: enemies, internal flaws, environmental obstacles, divine will
- Example: The corrupted king, the warrior's own fear, the cursed lands

#### The Three Actantial Axes

```
SENDER → OBJECT ← RECEIVER
   ↑                    ↓
HELPER ← SUBJECT → OPPONENT
```

**Axis of Will/Desire:** SUBJECT ↔ OBJECT
- "What does the character want?"
- Drives the plot forward

**Axis of Power:** HELPER ← SUBJECT → OPPONENT
- "What enables and blocks the character?"
- Determines how conflicts arise and resolve

**Axis of Knowledge:** SENDER → SUBJECT ← RECEIVER
- "Who initiates and who benefits?"
- Reveals deeper thematic meaning

#### Key Semantic Insight: Actant ≠ Actor

A single **actor** (character) can occupy multiple **actant** roles:
- A character can be SUBJECT in their own story but HELPER in another's
- A character can be their own OPPONENT (internal conflict)
- A character can switch roles mid-story (the traitor, the redemption arc)

---

## Part 2: Data Model for Neo4j

### 2.1 Node Types

```cypher
// Core Narrative Nodes
label :Story
  properties:
    id (string, primary key)
    title (string)
    premise (string)
    theme (string)
    status (enum: CONCEPT, OUTLINE, ACTIVE, COMPLETED)
    currentAct (integer 0-3)
    currentScene (integer)
    createdAt (datetime)
    updatedAt (datetime)
    metadata (map: for LLM context, style notes, etc.)

label :Theatre
  properties:
    id (string, primary key)
    name (string)
    description (string, rich text)
    type (enum: LOCATION, ORGANIZATION, REALM, FACTION)
    atmosphere (string, sensory/emotional)
    rulesOfEnchantment (list[string])
    secrets (list[string])
    accessRequirements (string)
    state (map: mutable properties like population, resources)

label :Character
  properties:
    id (string, primary key)
    name (string)
    essence (string, one sentence)
    appearance (string)
    background (string)
    beliefs (list[string])
    flaws (list[string])
    capabilities (list[string])
    status (enum: INTRODUCED, ACTIVE, DORMANT, DEPARTED, DECEASED)
    // Monomythic dimension
    archetypeRole (enum: HERO, MENTOR, HERALD, SHADOW, THRESHOLD_GUARDIAN, ALLY, SHAPE_SHIFTER, TRICKSTER)
    currentMonomythStage (integer 1-17)
    completedStages (list[integer])
    // Actantial dimension
    primaryActantFunction (enum: SUBJECT, OBJECT, SENDER, RECEIVER, HELPER, OPPONENT)
    desire (string)
    power (string)
    obstacle (string)
    alignment (map: moral, practical, emotional dimensions)

label :Scene
  properties:
    id (string, primary key)
    title (string)
    description (string)
    act (integer 0-3)
    sequence (integer)
    type (enum: EXPOSITION, CONFRONTATION, REVELATION, PIVOT, CLIMAX, RESOLUTION)
    dramaticTension (string)
    objectives (list[string])
    complications (list[string])
    discoveries (list[string])
    consequences (list[string])
    resolution (string, nullable)
    timestamp (datetime)
    generatedContent (boolean, whether LLM-generated)

// Narrative Structure Nodes
label :MonomythStage
  properties:
    stageNumber (integer 1-17)
    phaseName (enum: DEPARTURE, INITIATION, RETURN)
    stageName (string: "Call to Adventure", "Road of Trials", etc.)
    description (string)
    archetype (string)

label :ActantFunction
  properties:
    functionName (enum: SUBJECT, OBJECT, SENDER, RECEIVER, HELPER, OPPONENT)
    description (string)
    semantics (string)

label :Challenge
  properties:
    id (string, primary key)
    name (string)
    description (string)
    difficulty (enum: MINOR, MODERATE, MAJOR, CLIMACTIC)
    type (enum: COMBAT, SOCIAL, PUZZLE, MORAL, ENVIRONMENTAL)
    consequences (map: success outcomes, failure outcomes)

label :Transformation
  properties:
    id (string, primary key)
    name (string)
    description (string)
    scope (enum: PERSONAL, SOCIAL, COSMIC)
    before (string, character state before)
    after (string, character state after)
    trigger (string, what caused it)

label :Bond
  properties:
    id (string, primary key)
    type (enum: ALLY, RIVAL, MENTOR, LOVE, HATRED, NEUTRAL, COMPLEX)
    strength (integer 1-10)
    history (string, how they met)
    sharedGoals (list[string])
    conflicts (list[string])

label :Gift
  properties:
    id (string, primary key)
    name (string)
    description (string)
    powers (list[string])
    origin (string)
    giver (relationship to Character, nullable)
    symbolism (string, what does it represent?)
```

---

### 2.2 Relationship Types

```cypher
// Story Structure
:CONTAINS_THEATRE (Story) → (Theatre)
  properties: importanceLevel (integer)

:CONTAINS_CHARACTER (Story) → (Character)
  properties: importanceLevel (integer), role (string)

:CONTAINS_SCENE (Story) → (Scene)
  properties: sequence (integer)

:CONTAINS_ACT (Story) → (Act)
  properties: actNumber (integer)

// Character Monomythic Progression
:EXPERIENCES (Character) → (MonomythStage)
  properties: enteredAt (datetime), completedAt (datetime), notes (string)

:EXPERIENCES (Character) → (Transformation)
  properties: timestamp (datetime), context (string)

// Character Actantial Relationships
:PURSUES (Character with actant SUBJECT) → (Character with actant OBJECT)
  properties: intensity (integer), reason (string)

:HELPS (Character with actant HELPER) → (Character with actant SUBJECT)
  properties: helpType (string), strength (integer)

:OPPOSES (Character with actant OPPONENT) → (Character with actant SUBJECT)
  properties: conflictType (string), intensity (integer)

:SENDS_QUEST (Character with actant SENDER) → (Character with actant SUBJECT)
  properties: questDescription (string), reward (string)

:RECEIVES_BENEFIT (Character with actant RECEIVER) ← (Character with actant SUBJECT)
  properties: benefitType (string), gratitude (integer)

// Character Relationships
:FORMS_BOND (Character) → (Character)
  properties: bondId (reference to Bond node), since (datetime)

:MEETS_IN (Character) → (Theatre)
  properties: context (string), timestamp (datetime)

:PARTICIPATES_IN (Character) → (Scene)
  properties: role (enum: PROTAGONIST, ANTAGONIST, WITNESS, VICTIM, RESCUER)

:RECEIVES (Character) → (Gift)
  properties: timestamp (datetime), context (string)

:FACES (Character) → (Challenge)
  properties: outcome (enum: SUCCESS, FAILURE, PARTIAL), timestamp (datetime)

// Scene and Theatre
:OCCURS_IN (Scene) → (Theatre)

:FEATURES (Scene) → (Character)
  properties: screenTime (enum: MAJOR, SUPPORTING, CAMEO)

:REVEALS_ABOUT (Scene) → (Character)
  properties: disclosureType (enum: WEAKNESS, STRENGTH, SECRET, DESIRE, FEAR)

:OCCURS_IN_ACT (Scene) → (Act)

// Challenge and Obstacle
:PRESENTS (Challenge) → (Character with actant OPPONENT)

:HINDERS (Challenge) → (Character with actant SUBJECT)

// Transformation
:RESULTS_IN (Scene) → (Transformation)

:AFFECTS (Transformation) → (Character)

:AFFECTS (Transformation) → (Theatre)

// Knowledge and Succession
:MENTORS (Character with archetype MENTOR) → (Character with archetype HERO)
  properties: lessonTaught (list[string])

:TEACHES (Character) → (Character)
  properties: subject (string), effectiveness (integer)

// Thematic and Symbolic
:SYMBOLIZES (Gift) → (Transformation)

:EMBODIES (Character) → (Theme)
  properties: aspectOfTheme (string)
```

---

### 2.3 LLM-Friendly Design Decisions

#### Design Principle 1: Semantic Richness in Node Properties

Each node includes semantic descriptions alongside structural data. This allows LLMs to:
- Understand *why* relationships exist, not just *that* they exist
- Generate contextually appropriate narrative content
- Maintain thematic coherence across generative content

**Example:**
```cypher
Character {
  name: "Kael the Wanderer",
  essence: "A displaced warrior seeking redemption through impossible battles",
  archetypeRole: "HERO",
  currentMonomythStage: 6,  // Road of Trials
  primaryActantFunction: "SUBJECT",
  desire: "Redemption - to prove their worth despite past failures",
  power: "Combat mastery and an ancient sword of binding",
  obstacle: "The belief that redemption is impossible; their own self-doubt"
}
```

This single node contains enough context for an LLM to generate dialogue, internal monologue, or decision-making that feels consistent with the character.

#### Design Principle 2: Relationship Semantics Over Implicit Meaning

Instead of relying on relationship types alone, we include properties that explain the *nature* of the connection:

```cypher
MATCH (character:Character)-[r:OPPOSES {conflictType: "Internal", intensity: 8}]->(protagonist:Character)
// This tells an LLM: this isn't external combat, it's internal struggle, and it's severe
```

#### Design Principle 3: State vs. Schema

Rather than creating nodes for every possible state, we use properties maps:

```cypher
Theatre {
  state: {
    currentPopulation: ["Kael", "Merchant-Guild-Representative"],
    resources: ["Food-Cache", "Ancient-Library", "Healing-Springs"],
    threats: ["Shadow-Plague", "Political-Unrest"]
  }
}
```

This allows the story state to evolve without schema changes, and LLMs can reason about the dynamic state.

#### Design Principle 4: Bidirectional Traversal for Context

Relationships should be queryable in both directions for narrative generation:

```cypher
// Forward: What does this scene reveal about the character?
MATCH (scene:Scene)-[:REVEALS_ABOUT {disclosureType: "SECRET"}]->(character:Character)

// Backward: What scenes have revealed secrets about this character?
MATCH (character:Character)<-[:REVEALS_ABOUT {disclosureType: "SECRET"}]-(scene:Scene)
```

#### Design Principle 5: Embedding-Friendly Narrative Fields

Fields like `essence`, `desire`, `power`, `obstacle`, and `dramaticTension` are specifically designed to be vectorized and semantically searched:

```
Character.essence: "A displaced warrior seeking redemption through impossible battles"
→ Vector embedding captures: displacement, redemption, struggle, warrior identity

Scene.dramaticTension: "The kingdom's last hope arrives broken and doubting"
→ Vector embedding captures: stakes, hope, vulnerability, transformation
```

An LLM + vector DB can find thematically similar scenes, characters, and moments across the story.

---

## Part 3: Domain Language Specification

### 3.1 Core Vocabulary

#### Narrative Operators

| Operator | Semantics | Example |
|----------|-----------|---------|
| **Calls** | Initiates a quest or inciting incident | "The Prophet calls the Wanderer to seek the Lost Crown" |
| **Tests** | Presents a trial or challenge | "The Guardian tests the Hero's resolve with a riddle" |
| **Grants** | Provides aid, gift, or knowledge | "The Mentor grants the Novice access to forbidden archives" |
| **Transforms** | Changes state of being | "The ritual transforms the mortal into something other" |
| **Opposes** | Creates conflict or obstacle | "The Tyrant opposes the liberation movement" |
| **Reveals** | Discloses hidden information | "The discovery reveals that the mentor was a traitor" |
| **Binds** | Creates alliance or deepens relationship | "The shared trial binds the heroes in brotherhood" |
| **Severs** | Breaks relationship or creates rupture | "The betrayal severs all trust between them" |

#### Narrative States

| State | Meaning | Transition Triggers |
|-------|---------|-------------------|
| **Dormant** | Latent; waiting for activation | Inciting incident, character entry |
| **Ascendant** | Growing in power, influence, or narrative importance | Victories, ally gains, revelation |
| **Contested** | Under active challenge; uncertain outcome | Direct conflict, moral dilemma |
| **Transformed** | Fundamentally changed | Apotheosis, death and rebirth, integration |
| **Resolved** | Concluded; moves to final state | Climax achieved, transformation complete |

#### Dramatic Modalities

| Modality | Function | Triggers Generation |
|----------|----------|-------------------|
| **Exposition** | Information sharing; world-building | Scene setup, character introduction |
| **Confrontation** | Direct conflict; testing characters | Combat, debate, choice |
| **Revelation** | Disclosure of hidden truth | Mystery solving, secret discovery |
| **Pivot** | Turning point; change of trajectory | Major decision, betrayal, transformation |
| **Climax** | Ultimate test; highest stakes | Final confrontation with central conflict |
| **Resolution** | Integration; new equilibrium | Victory/defeat, transformation acceptance |

---

### 3.2 Semantic Relationships (In Natural Language)

#### Archetypal Relationships

```
Hero -[on journey with]→ Mentor
Hero -[tests themselves against]→ Shadow
Hero -[questions]→ Threshold Guardian (initially) -[trusts]→ Threshold Guardian (after passage)
Hero -[aided by]→ Ally
Shape-Shifter -[uncertain alignment with]→ Hero
Trickster -[disrupts]→ Hero's assumptions
```

#### Actantial Relationships (In Natural Language)

```
Subject -[desires]→ Object
Subject -[is blocked by]→ Opponent
Subject -[is enabled by]→ Helper
Subject -[is sent by]→ Sender
Subject's achievement -[benefits]→ Receiver

// Complex example:
The Knight (Subject, HERO) -[desires]→ Justice (Object)
  -[is blocked by]→ The Corrupt Judge (Opponent, SHADOW)
  -[is enabled by]→ The Forgotten Law (Helper, THRESHOLD_GUARDIAN knowledge)
  -[is sent by]→ The Dying Victim (Sender, HERALD)
  -[ultimately benefits]→ The Kingdom (Receiver)
```

#### Dynamic Narrative Relationships

```
Character1 -[was betrayed by]→ Character2 → creates OPPONENT relationship
Character1 -[forgives]→ Character2 → transforms OPPONENT to ALLY
Character1 -[is revealed as]→ (different identity) → SHAPE_SHIFTER function changes
Character1 -[mentors]→ Character2 → Character2 begins HERO's journey
```

---

### 3.3 Query Patterns for LLM Integration

These patterns help LLMs understand and generate consistent narrative content:

#### Pattern 1: Character Context Query
```cypher
MATCH (character:Character)
      -[:EXPERIENCES]->(stage:MonomythStage),
      (character)-[:PURSUES {intensity: intensity}]->(desire),
      (character)-[:FORMS_BOND]->(ally:Character),
      (character)-[opp:OPPOSES]->(opponent:Character),
      (character)-[:MEETS_IN {context: context}]->(location:Theatre)
RETURN character, stage, desire, ally, opponent, location
```
*Use case:* Generate dialogue, internal monologue, or decision-making for a character in context.

#### Pattern 2: Scene Consistency Query
```cypher
MATCH (scene:Scene)-[:OCCURS_IN]->(location:Theatre),
      (scene)-[:FEATURES]->(character:Character),
      (character)-[:FACES]->(challenge:Challenge),
      (challenge)-[:HINDERS]->(obstacle)
RETURN scene, location, character, challenge, obstacle
```
*Use case:* Ensure generated scene content respects location rules, character capabilities, and narrative logic.

#### Pattern 3: Thematic Alignment Query
```cypher
MATCH (story:Story {theme: $theme}),
      (story)-[:CONTAINS_CHARACTER]->(character:Character),
      (character)-[:EMBODIES]->(aspect)
WHERE aspect.aspectOfTheme CONTAINS $themeKeyword
RETURN character, aspect
```
*Use case:* Generate content that reinforces story themes through character actions and dialogue.

#### Pattern 4: Consequence Tracing Query
```cypher
MATCH (scene:Scene)-[:RESULTS_IN]->(transformation:Transformation),
      (transformation)-[:AFFECTS]->(character:Character),
      (character)-[:MEETS_IN]->(location:Theatre)
RETURN scene, transformation, character, location
ORDER BY scene.timestamp
```
*Use case:* Ensure generated consequences ripple through the narrative consistently.

#### Pattern 5: Relationship Evolution Query
```cypher
MATCH (char1:Character)-[bond:FORMS_BOND]->(char2:Character)
OPTIONAL MATCH (char1)-[:PARTICIPATES_IN]->(scene:Scene)<-[:PARTICIPATES_IN]-(char2)
RETURN char1, char2, bond, collect(scene)
ORDER BY scene.timestamp
```
*Use case:* Generate dialogue and interaction history between characters showing relationship progression.

---

## Part 4: Integrated Narrative Loop

### 4.1 The Generative Narrative Cycle

```
┌─────────────────────────────────────────────────────┐
│  1. SCENE INITIALIZATION                            │
│  ├─ DM specifies act, location, participants         │
│  ├─ Query graph for character contexts, theatre      │
│  └─ Pass to LLM with semantic constraints            │
└──────────────────────┬──────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  2. LLM NARRATIVE GENERATION                         │
│  ├─ Generate scene description                       │
│  ├─ Generate character dialogue/action               │
│  ├─ Suggest dramatic complications                   │
│  └─ Maintain semantic consistency via prompting      │
└──────────────────────┬──────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  3. PLAYER/DM INTERACTION                            │
│  ├─ Players make choices                             │
│  ├─ DM makes narrative adjustments                   │
│  └─ System records state changes                     │
└──────────────────────┬──────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  4. GRAPH UPDATE & CONSEQUENCE PROPAGATION           │
│  ├─ Create/update Scene node with outcomes           │
│  ├─ Update Character monomyth stage if appropriate   │
│  ├─ Update actantial functions based on changes      │
│  ├─ Update relationship bonds (strength, type)       │
│  ├─ Query for ripple effects                         │
│  └─ Flag future scenes for revision if needed        │
└──────────────────────┬──────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  5. REGENERATION/CONSISTENCY CHECK                   │
│  ├─ Query for narrative contradictions               │
│  ├─ Check that character actions align with values   │
│  ├─ Validate scene type matches dramatic arc         │
│  └─ Suggest generation of follow-up scenes           │
└──────────────────────┬──────────────────────────────┘
                       ↓
                  Back to 1
```

### 4.2 DM Tools for Generative Control

#### Tool 1: Character Briefing Generator
```cypher
// Generates rich context for an LLM before character dialogue generation
MATCH (character:Character)-[:EXPERIENCES]->(stage:MonomythStage),
      (character)-[helps:HELPS]->(subject),
      (character)-[opposes:OPPOSES]->(opponent),
      (character)-[bond:FORMS_BOND]->(ally)
RETURN {
  character: character,
  stage: stage,
  relationships: {helps, opposes, bond},
  recentScenes: ...,  // Last 3 scenes featuring character
  desireAndObstacle: {character.desire, character.obstacle}
}
```

#### Tool 2: Consequence Explorer
```cypher
// Shows a DM all the potential narrative consequences of a choice
MATCH (choice:Choice)-[:TRIGGERS_CONSEQUENCE]->(consequence),
      (consequence)-[:AFFECTS]->(character:Character),
      (consequence)-[:AFFECTS]->(location:Theatre)
RETURN consequence, character, location
```

#### Tool 3: Thematic Resonance Checker
```cypher
// Validates that a scene reinforces story theme
MATCH (story:Story {theme: $theme}),
      (story)-[:CONTAINS_SCENE]->(scene:Scene),
      (scene)-[:FEATURES]->(character:Character)
WHERE character.alignment.moral ALIGNS_WITH story.theme
RETURN scene, character, thematicResonance
```

#### Tool 4: Character Arc Progression
```cypher
// Visualizes where a character is in their hero's journey
MATCH (character:Character)-[:EXPERIENCES]->(stage:MonomythStage)
RETURN character, stage, nextExpectedStage
```

---

## Part 5: LLM Prompting Architecture

### 5.1 System Prompt Structure

The system prompt to your LLM should include:

1. **Role Definition**
   - "You are a narrative engine for collaborative TTRPGs"
   - "Your role is to generate consistent, immersive narrative content"

2. **Domain Constraints**
   - Character monomythic stage (what journey are they on?)
   - Actantial function (what role do they play in the broader plot?)
   - Desire, power, obstacle (what wants/enables/blocks them?)

3. **Scene Context**
   - Setting (theatre with its rules and atmosphere)
   - Participants (who's present and their relationship to each other)
   - Dramatic tension (what's at stake?)
   - Scene type (exposition vs. climax changes tone)

4. **Thematic Guidance**
   - Story theme (what is this story about?)
   - Character thematic alignment (what aspect do they embody?)
   - Recent plot turns (what changed the narrative trajectory?)

5. **Guardrails**
   - Character consistency (don't contradict established character)
   - Consequence fidelity (mentioned consequences must occur)
   - Relationship respect (bonds have history; honor it)
   - Magic system rules (if any; respect world-building)

### 5.2 Example Prompt Template

```
# NARRATIVE GENERATION REQUEST

## Scene Context
- **Title**: [Scene Title]
- **Setting**: [Theatre Name] - [Atmosphere]
  - Rules: [rulesOfEnchantment]
  - Current State: [population, resources, threats]
- **Act**: [Act Number] - [Overall Dramatic Phase]
- **Type**: [EXPOSITION/CONFRONTATION/REVELATION/etc.]

## Participating Characters
### Character 1: [Name]
- **Essence**: [One-sentence core nature]
- **Current Monomyth Stage**: [Stage Name] (Stage X/17)
- **Actantial Function**: [SUBJECT/OBJECT/HELPER/OPPONENT/SENDER/RECEIVER]
- **Desire**: [What they want]
- **Power**: [What enables them]
- **Obstacle**: [What blocks them]
- **Recent Events**: [Last scene they were in and outcome]

### Character 2: [Name]
[Same format]

## Dramatic Tension
[What needs to happen in this scene? What's at stake?]

## Story Theme
"[Theme]" - Characters should reinforce this through their choices and dialogue.

## Your Task
Generate:
1. A 200-300 word scene description with sensory detail
2. Natural dialogue between characters
3. At least one dramatic complication or choice point
4. Potential consequences or scene outcomes

Maintain consistency with character desires, powers, and obstacles. Respect relationships and bonds.
```

---

## Part 6: Example Domain Instance

### A Fragment: "The Wanderer's Redemption"

#### Story Node
```
Story {
  id: "story-001",
  title: "The Wanderer's Redemption",
  theme: "Transformation through confronting your past",
  currentAct: 2,
  currentScene: 6
}
```

#### Character: Kael the Wanderer
```
Character {
  id: "char-kael",
  name: "Kael the Wanderer",
  essence: "A displaced warrior haunted by betrayal, seeking redemption through impossible battles",
  archetypeRole: "HERO",
  currentMonomythStage: 6,  // Road of Trials
  completedStages: [1, 2, 3, 4, 5],
  primaryActantFunction: "SUBJECT",
  desire: "Redemption - to prove worth despite past failures",
  power: "Combat mastery, the Sword of Binding, a network of allies",
  obstacle: "Self-doubt and the belief that redemption is impossible"
}
```

#### Character: The Crimson Sage
```
Character {
  id: "char-sage",
  name: "The Crimson Sage",
  essence: "An immortal mentor bearing knowledge of worlds beyond, guiding heroes toward their destiny",
  archetypeRole: "MENTOR",
  primaryActantFunction: "HELPER",
  desire: "To guide worthy heroes to their transformation",
  power: "Ancient wisdom, magical sight, connection to mystical forces",
  obstacle: "Cannot directly solve problems; can only guide"
}
```

#### Relationship: Kael and Sage
```
FORMS_BOND {
  type: "MENTOR",
  strength: 8,
  history: "Met at the Threshold of the Ruined City; Sage recognized Kael's potential",
  sharedGoals: ["Defeat the Betrayer", "Restore the Old Order"],
  conflicts: ["Sage believes in patience; Kael wants immediate action"]
}

HELPS {
  helpType: "Guidance and magical aid",
  strength: 9
}
```

#### Theatre: The Citadel of Echoes
```
Theatre {
  id: "theatre-citadel",
  name: "The Citadel of Echoes",
  type: "LOCATION",
  atmosphere: "Ancient, haunted, filled with memories of past glory",
  rulesOfEnchantment: [
    "Spirits of the dead can be glimpsed in mirrors",
    "Speaking names of the dead aloud summons their attention",
    "The inner sanctum is only accessible to those who've faced their shadow"
  ],
  state: {
    population: ["Kael", "The Crimson Sage", "Three shadow spirits"],
    resources: ["Ancient library", "Sacred flame", "Armory of ancient weapons"],
    threats: ["The Betrayer's minions patrol the lower levels", "Temporal decay eating at the structure"]
  }
}
```

#### Scene: Confrontation in the Hall of Mirrors
```
Scene {
  id: "scene-006",
  title: "The Hall of Mirrors",
  act: 2,
  sequence: 6,
  type: "CONFRONTATION",
  setting: "theatre-citadel",
  participants: ["char-kael", "char-sage", "shadow-spirit-self"],
  dramaticTension: "Kael must confront the mirror-image of themselves—the self they've rejected—to progress deeper into the citadel",
  objectives: [
    "Kael must acknowledge their past mistake",
    "Kael must choose: integrate the shadow-self or continue fleeing"
  ],
  complications: [
    "The shadow-self has Kael's strength but is driven by rage and despair",
    "The Sage remains silent—Kael must find their own answer",
    "Engaging violently with the shadow damages the hall; time is limited"
  ],
  discoveries: [
    "Kael realizes the 'betrayer' was partly their own blindness",
    "The Sword of Binding glows when Kael accepts both light and dark aspects of self"
  ]
}
```

#### Challenge Node
```
Challenge {
  id: "challenge-shadow-self",
  name: "The Shadow Self in the Mirror",
  type: "MORAL",
  difficulty: "MAJOR",
  consequences: {
    success: "Kael integrates self-knowledge; transforms toward apotheosis",
    failure: "Kael flees further; delayed monomyth progression, relationship strain with Sage"
  }
}
```

#### Transformation Node
```
Transformation {
  id: "transform-shadow-integration",
  name: "Integration of the Shadow",
  scope: "PERSONAL",
  before: "Kael the Wanderer - fleeing their past, defined by negation",
  after: "Kael the Integrated - whole, owning both light and dark, moving toward wholeness",
  trigger: "Confrontation in the Hall of Mirrors; acceptance of past mistakes"
}
```

#### Relationships in this Fragment
```
kael -[:EXPERIENCES {enteredAt: ..., notes: "Midpoint of Road of Trials"}]-> stage-6-road-of-trials
kael -[:FACES {outcome: "PARTIAL"}]-> challenge-shadow-self
kael -[:FORMS_BOND {type: "MENTOR", strength: 8, since: ...}]-> sage
sage -[:HELPS {helpType: "Guidance", strength: 9}]-> kael
kael -[:PURSUES {intensity: 9, reason: "Must prove worth and attain redemption"}]-> redemption-object
scene-006 -[:OCCURS_IN]-> theatre-citadel
scene-006 -[:RESULTS_IN]-> transform-shadow-integration
transform-shadow-integration -[:AFFECTS]-> kael
kael -[:PARTICIPATES_IN {role: "PROTAGONIST"}]-> scene-006
sage -[:PARTICIPATES_IN {role: "WITNESS"}]-> scene-006
```

---

## Part 7: Implementation Roadmap

### Phase 1: Schema & Core Infrastructure
- [ ] Define Neo4j schema (nodes, relationships, constraints)
- [ ] Create indexes for fast querying (character by archetype, scene by type, etc.)
- [ ] Build basic CRUD operations for domain entities
- [ ] Implement monomyth stage progression logic

### Phase 2: LLM Integration Layer
- [ ] Design system prompts and retrieval patterns
- [ ] Implement context aggregation queries
- [ ] Build prompt builder that translates graph data to narrative briefs
- [ ] Test LLM consistency and constraint adherence

### Phase 3: DM Tools & UI
- [ ] Scene briefing generator
- [ ] Character context viewer
- [ ] Consequence explorer
- [ ] Thematic resonance checker
- [ ] Character arc progression visualizer

### Phase 4: Player Experience
- [ ] Choice point generation and presentation
- [ ] Consequence tracking and feedback
- [ ] Immersive scene presentation
- [ ] Character relationship visualization for players

### Phase 5: Advanced Features
- [ ] Automated consequence propagation
- [ ] Narrative contradiction detection
- [ ] Long-form arc planning and suggestion
- [ ] Multi-party story synchronization
- [ ] Vector embeddings for thematic search

---

## Conclusion

This domain model positions the TTRPG system as a **structured narrative engine** where:

- **Campbell's Monomyth** provides the *archetypal shape* of character journeys
- **Greimas' Actantial Model** provides the *relational structure* of desires and obstacles
- **Neo4j's Graph Structure** enables *rich, queryable narrative context* for LLM generation
- **LLM Integration** allows *generative content* that's grounded in consistent, semantically rich narrative data

The key innovation is treating the database not just as data storage, but as a **living, queryable narrative knowledge base** that informs and constrains generative content. This allows for simultaneous player immersion and DM control—the narrative can feel spontaneous while remaining architecturally coherent.

The semantic richness of the model (essence, desire, power, obstacle, dramatic tension) makes the graph "legible" to LLMs in a way that enables creative, contextually appropriate generation while maintaining character consistency and thematic alignment.
