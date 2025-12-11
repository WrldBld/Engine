# WrldBldr Engine

The **Engine** is the backend server for WrldBldr, a TTRPG (Tabletop Role-Playing Game) world management system. It handles world data persistence, AI-assisted NPC responses, asset generation, and real-time multiplayer coordination.

## Goals

- **World Building**: Enable Dungeon Masters to create rich campaign settings with locations, characters, relationships, and story arcs
- **AI-Assisted Gameplay**: Use LLMs (Ollama/qwen3-vl) to generate contextual NPC dialogue and actions
- **Generative Assets**: Integrate with ComfyUI to create character portraits, sprites, backdrops, and tilesheets
- **Real-Time Multiplayer**: Support multiple players and a DM in synchronized sessions via WebSocket
- **Graph-Based Data**: Leverage Neo4j to model complex relationships between characters, locations, and narrative elements

## Architecture

The Engine follows **Hexagonal Architecture** (Ports and Adapters) with clear layer separation:

```
Engine/
├── src/
│   ├── main.rs                    # Axum server entry point
│   │
│   ├── domain/                    # Core business logic (no external dependencies)
│   │   ├── entities/              # Domain entities
│   │   │   ├── world.rs           # World - top-level campaign container
│   │   │   ├── act.rs             # Act - monomyth story stages
│   │   │   ├── scene.rs           # Scene - storytelling unit (location + time + events)
│   │   │   ├── character.rs       # Character - NPCs with archetypes and wants
│   │   │   ├── location.rs        # Location - hierarchical places with connections
│   │   │   ├── interaction.rs     # Interaction - available scene actions
│   │   │   ├── grid_map.rs        # GridMap - tactical combat grids
│   │   │   ├── gallery_asset.rs   # GalleryAsset - generated/uploaded assets
│   │   │   ├── generation_batch.rs# GenerationBatch - asset generation tracking
│   │   │   └── workflow_config.rs # WorkflowConfiguration - ComfyUI workflows
│   │   │
│   │   ├── value_objects/         # Immutable domain concepts
│   │   │   ├── ids.rs             # Strongly-typed IDs (WorldId, CharacterId, etc.)
│   │   │   ├── archetype.rs       # Campbell archetypes (Hero, Mentor, Shadow, etc.)
│   │   │   ├── want.rs            # Character desires (actantial model)
│   │   │   ├── relationship.rs    # Character relationships with sentiment
│   │   │   ├── rule_system.rs     # Game system configuration (D20, Fate, etc.)
│   │   │   └── directorial.rs     # LLM guidance (tone, motivations, forbidden topics)
│   │   │
│   │   ├── aggregates/            # Aggregate roots (planned)
│   │   ├── events/                # Domain events (planned)
│   │   └── services/              # Domain services (planned)
│   │
│   ├── application/               # Use cases and orchestration
│   │   ├── services/              # Application services
│   │   │   ├── world_service.rs   # World CRUD and export
│   │   │   ├── character_service.rs
│   │   │   ├── location_service.rs
│   │   │   ├── scene_service.rs
│   │   │   ├── generation_service.rs  # Asset generation queue
│   │   │   ├── workflow_service.rs    # ComfyUI workflow management
│   │   │   ├── llm_service.rs         # AI response orchestration
│   │   │   └── suggestion_service.rs  # AI-powered content suggestions
│   │   │
│   │   ├── ports/
│   │   │   ├── inbound/           # Request handling interfaces (planned)
│   │   │   └── outbound/
│   │   │       └── llm_port.rs    # LLM abstraction trait
│   │   │
│   │   └── dto/                   # Data transfer objects (planned)
│   │
│   └── infrastructure/            # External system adapters
│       ├── persistence/           # Neo4j repositories
│       │   ├── connection.rs      # Database connection and schema init
│       │   ├── world_repository.rs
│       │   ├── character_repository.rs
│       │   ├── location_repository.rs
│       │   ├── scene_repository.rs
│       │   ├── interaction_repository.rs
│       │   ├── relationship_repository.rs
│       │   ├── asset_repository.rs
│       │   └── workflow_repository.rs
│       │
│       ├── http/                  # REST API routes
│       │   ├── world_routes.rs    # /api/worlds/*
│       │   ├── character_routes.rs# /api/characters/*
│       │   ├── location_routes.rs # /api/locations/*
│       │   ├── scene_routes.rs    # /api/scenes/*
│       │   ├── interaction_routes.rs
│       │   ├── asset_routes.rs    # /api/assets/* (generation queue)
│       │   ├── suggestion_routes.rs # /api/suggest/*
│       │   ├── workflow_routes.rs # /api/workflows/*
│       │   └── export_routes.rs   # /api/worlds/{id}/export
│       │
│       ├── websocket.rs           # WebSocket handler for real-time play
│       ├── ollama.rs              # Ollama LLM client
│       ├── comfyui.rs             # ComfyUI asset generation client
│       ├── config.rs              # Environment configuration
│       ├── state.rs               # Shared application state
│       ├── session.rs             # Game session management
│       │
│       ├── export/                # World export functionality
│       │   ├── json_exporter.rs
│       │   └── world_snapshot.rs
│       │
│       └── asset_manager/         # Asset file storage
│
├── Cargo.toml                     # Dependencies
├── Dockerfile                     # Container build
├── shell.nix                      # NixOS development environment
└── Taskfile.yml                   # Task runner commands
```

## Domain Model

### Core Entities

| Entity | Description |
|--------|-------------|
| **World** | Top-level campaign container with rule system configuration |
| **Act** | Story arc using Campbell's monomyth (12 stages from "Ordinary World" to "Return with Elixir") |
| **Scene** | Storytelling unit combining location, time, characters, and available interactions |
| **Character** | NPCs with Campbell archetypes, actantial wants, and relationship networks |
| **Location** | Hierarchical places (Town contains Tavern contains Rooms) with spatial connections |
| **Interaction** | Available actions in a scene (Dialogue, Examine, UseItem, Travel, Attack) |
| **GridMap** | Tactical combat grid with terrain, elevation, and cover |

### Key Concepts

- **Campbell Archetypes**: Hero, Mentor, Threshold Guardian, Herald, Shapeshifter, Shadow, Trickster, Ally
- **Actantial Model**: Characters have "Wants" with targets and intensity
- **Relationships**: Character connections with sentiment (-1.0 hatred to +1.0 love)
- **Directorial Notes**: LLM guidance including tone, NPC motivations, and forbidden topics

## Running the Engine

### Prerequisites

- **Rust** (latest stable)
- **Neo4j** 5.x (graph database)
- **Ollama** with `qwen3-vl:30b` model (or compatible LLM)
- **ComfyUI** (optional, for asset generation)

### Using Nix (Recommended)

```bash
# Enter development environment
nix-shell

# Run the server
cargo run
```

### Using Docker Compose

From the repository root:

```bash
# Start Engine + Neo4j
docker-compose up

# Engine available at http://localhost:3000
# Neo4j browser at http://localhost:7474
```

### Manual Setup

```bash
# Set environment variables (or create .env file)
export NEO4J_URI="bolt://localhost:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="your_password"
export OLLAMA_BASE_URL="http://localhost:11434/v1"
export OLLAMA_MODEL="qwen3-vl:30b"
export COMFYUI_BASE_URL="http://localhost:8188"

# Build and run
cargo build --release
./target/release/engine
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `NEO4J_URI` | `bolt://localhost:7687` | Neo4j connection URI |
| `NEO4J_USER` | `neo4j` | Neo4j username |
| `NEO4J_PASSWORD` | (required) | Neo4j password |
| `NEO4J_DATABASE` | `neo4j` | Neo4j database name |
| `OLLAMA_BASE_URL` | `http://10.8.0.6:11434/v1` | Ollama API endpoint |
| `OLLAMA_MODEL` | `qwen3-vl:30b` | LLM model to use |
| `COMFYUI_BASE_URL` | `http://10.8.0.6:8188` | ComfyUI API endpoint |
| `SERVER_PORT` | `3000` | HTTP server port |

## API Overview

### REST Endpoints

```
# Worlds
GET    /api/worlds                    # List all worlds
POST   /api/worlds                    # Create world
GET    /api/worlds/{id}               # Get world
PUT    /api/worlds/{id}               # Update world
DELETE /api/worlds/{id}               # Delete world
GET    /api/worlds/{id}/export        # Export world as JSON snapshot

# Characters
GET    /api/worlds/{world_id}/characters
POST   /api/worlds/{world_id}/characters
GET    /api/characters/{id}
PUT    /api/characters/{id}
PUT    /api/characters/{id}/archetype  # Change archetype

# Locations
GET    /api/worlds/{world_id}/locations
POST   /api/worlds/{world_id}/locations
GET    /api/locations/{id}/connections
POST   /api/locations/connections

# Scenes & Interactions
GET    /api/acts/{act_id}/scenes
POST   /api/acts/{act_id}/scenes
GET    /api/scenes/{scene_id}/interactions
POST   /api/scenes/{scene_id}/interactions

# Asset Generation
POST   /api/assets/generate           # Queue generation
GET    /api/assets/queue              # List queue
GET    /api/assets/batch/{batch_id}   # Batch status
GET    /api/characters/{id}/gallery   # Character assets

# AI Suggestions
POST   /api/suggest/character-name
POST   /api/suggest/character-description
POST   /api/suggest/location-description

# Workflow Configuration
GET    /api/workflows                 # List configurations
POST   /api/workflows/{slot}          # Configure workflow slot
POST   /api/workflows/{slot}/test     # Test workflow
```

### WebSocket Protocol

Connect to `ws://localhost:3000/ws`

**Client → Server Messages:**
- `JoinSession` - Join a game session
- `PlayerAction` - PC performs action (talk, examine, travel)
- `DirectorialUpdate` - DM updates scene guidance
- `ApprovalDecision` - DM approves/rejects LLM response
- `RequestSceneChange` - Request scene transition

**Server → Client Messages:**
- `SessionJoined` - Confirmation with world snapshot
- `SceneUpdate` - Scene state changed
- `DialogueResponse` - NPC dialogue with choices
- `ApprovalRequired` - DM approval needed for LLM response
- `GenerationEvent` - Asset generation progress

## Development

### Task Commands

```bash
# Using Taskfile
task build      # Build the project
task run        # Run the server
task test       # Run tests
task check      # Run clippy and format check
```

### Code Style

- Follow Rust idioms and clippy recommendations
- Use `async/await` for all I/O operations
- Prefer explicit types over inference in function signatures
- Domain layer must have zero external dependencies

## Current Status

### Implemented
- Full CRUD for all domain entities
- REST API (80+ endpoints)
- WebSocket server with session management
- Neo4j persistence layer
- World export to JSON
- Asset gallery with generation batches
- ComfyUI workflow configuration
- Ollama client for LLM

### In Progress
- LLM response flow (TODOs in websocket.rs)
- DM approval workflow

### Planned
- Aggregate pattern implementation
- Domain events
- Authentication/authorization
- Comprehensive test suite

## Related

- [Player README](../Player/README.md) - Frontend client documentation
- [Master Plan](../plans/00-master-plan.md) - Full project specification
