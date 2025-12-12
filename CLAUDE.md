# Engine - Claude Code Instructions

This is the **Engine** component of WrldBldr - the backend server written in Rust that provides the API and WebSocket services.

## Environment

This project runs on **NixOS**. Use `nix-shell` for development dependencies:

```bash
nix-shell -p rustc cargo gcc pkg-config openssl.dev --run "cargo check"
```

## Architecture

The Engine follows **hexagonal architecture** (ports and adapters):

```
src/
├── domain/           # Core business logic
│   ├── entities/     # Domain entities (World, Character, Scene, etc.)
│   └── value_objects/# IDs, types, small value types
├── application/      # Use cases and services
│   └── services/     # LLM service, game logic, story events
├── infrastructure/   # External adapters
│   ├── http/         # REST API routes (Axum)
│   ├── websocket.rs  # WebSocket server
│   ├── persistence/  # Neo4j repositories
│   └── llm/          # Ollama LLM client
└── main.rs
```

## Key Conventions

### REST API

- Routes are defined in `src/infrastructure/http/mod.rs`
- Each entity has its own routes file (e.g., `character_routes.rs`)
- Use Axum extractors: `Path`, `Query`, `State`, `Json`
- Return `Result<Json<T>, (StatusCode, String)>` for handlers

### Routing Rules

- **Always use proper routing** - UI tabs and sub-views should have URL routes
- Route pattern: `/api/{resource}` for REST, `/ws` for WebSocket
- Paginated responses should include `events`, `total`, `limit`, `offset` fields

### Domain Entities

- Each entity has an ID value object (e.g., `CharacterId`, `WorldId`)
- IDs are UUIDs wrapped in newtype structs
- Use builder pattern for complex entity construction

### Database

- Neo4j is the primary database
- Repositories are in `src/infrastructure/persistence/`
- Use Cypher queries with parameterized values

### LLM Integration

- Ollama client in `src/infrastructure/llm/`
- Game prompts include context about active narrative events
- Responses are parsed for structured output (dialogue, tools, suggestions)

## Running

```bash
# Development
cargo run

# Check compilation
cargo check

# Run tests
cargo test
```

The server runs on `http://localhost:3000` by default.
