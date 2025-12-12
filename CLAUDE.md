# Engine - Claude Code Instructions

This is the **Engine** component of WrldBldr - the backend server written in Rust that provides the API and WebSocket services.

## Environment

This project runs on **NixOS**. Use `nix-shell` for development dependencies:

```bash
nix-shell -p rustc cargo gcc pkg-config openssl.dev --run "cargo check"
```

## Architecture

The Engine follows **hexagonal architecture** (ports and adapters) with **domain-driven design** principles.

```
src/
├── domain/              # Core business logic (INNERMOST - no external deps)
│   ├── entities/        # Domain entities (World, Character, Scene, etc.)
│   ├── value_objects/   # IDs, types, small immutable types
│   ├── services/        # Pure domain logic services
│   ├── aggregates/      # Aggregate roots (WorldAggregate)
│   └── events/          # Domain events
├── application/         # Use cases and orchestration
│   ├── services/        # Application services (orchestrate domain + ports)
│   ├── ports/
│   │   ├── inbound/     # Use case interfaces (traits)
│   │   └── outbound/    # Repository/external service interfaces (traits)
│   └── dto/             # Data transfer objects
├── infrastructure/      # External adapters (OUTERMOST)
│   ├── http/            # REST API routes (Axum) - implements inbound ports
│   ├── websocket.rs     # WebSocket server
│   ├── persistence/     # Neo4j repositories - implements outbound ports
│   ├── llm/             # Ollama client - implements LlmPort
│   └── export/          # World snapshot builder
└── main.rs
```

## CRITICAL: Hexagonal Architecture Rules

### Dependency Direction (STRICTLY ENFORCED)

```
Domain ← Application ← Infrastructure
         ↑
    Presentation/HTTP
```

**NEVER violate these rules:**

1. **Domain layer has NO external dependencies**
   - No `use crate::infrastructure::*`
   - No `use crate::application::*`
   - No framework types (Axum, Neo4j, serde on domain types)
   - Domain types are pure Rust structs/enums

2. **Application layer depends ONLY on Domain**
   - `use crate::domain::*` ✓
   - `use crate::infrastructure::*` ✗ FORBIDDEN
   - Services use TRAIT BOUNDS, not concrete types

3. **Infrastructure implements ports**
   - Repositories implement `WorldRepository`, `CharacterRepository` traits
   - LLM client implements `LlmPort` trait
   - HTTP handlers call application services, NOT repositories directly

### Port Pattern (REQUIRED)

```rust
// application/ports/outbound/repository_port.rs
#[async_trait]
pub trait WorldRepository: Send + Sync {
    async fn save(&self, world: &World) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: WorldId) -> Result<Option<World>, RepositoryError>;
    async fn list(&self) -> Result<Vec<World>, RepositoryError>;
}

// application/services/world_service.rs
pub struct WorldServiceImpl<R: WorldRepository> {
    repository: R,  // Trait bound, NOT concrete type
}

// infrastructure/persistence/world_repository.rs
impl WorldRepository for Neo4jWorldRepository {
    // ... implementation
}
```

### HTTP Routes Pattern (REQUIRED)

HTTP handlers MUST go through application services:

```rust
// CORRECT
pub async fn list_worlds(State(state): State<Arc<AppState>>) -> Result<Json<Vec<WorldResponse>>, ...> {
    let worlds = state.world_service.list_worlds().await?;  // Through service
    Ok(Json(worlds.into_iter().map(WorldResponse::from).collect()))
}

// WRONG - Direct repository access
pub async fn list_worlds(State(state): State<Arc<AppState>>) -> Result<Json<Vec<WorldResponse>>, ...> {
    let worlds = state.repository.worlds().list().await?;  // VIOLATION!
    // ...
}
```

### Service Dependency Pattern (REQUIRED)

```rust
// CORRECT - Trait bound
pub struct WorldServiceImpl<R: WorldRepository> {
    repository: R,
}

// CORRECT - Trait object
pub struct WorldServiceImpl {
    repository: Arc<dyn WorldRepository>,
}

// WRONG - Concrete infrastructure type
pub struct WorldServiceImpl {
    repository: Neo4jRepository,  // VIOLATION!
}
```

## Key Conventions

### REST API

- Routes are defined in `src/infrastructure/http/mod.rs`
- Each entity has its own routes file (e.g., `character_routes.rs`)
- Use Axum extractors: `Path`, `Query`, `State`, `Json`
- Return `Result<Json<T>, (StatusCode, String)>` for handlers
- **Routes call services, services call repositories via ports**

### Domain Entities

- Each entity has an ID value object (e.g., `CharacterId`, `WorldId`)
- IDs are UUIDs wrapped in newtype structs
- Domain types have NO serde attributes (use DTOs for serialization)
- Use builder pattern for complex entity construction

### Database (Neo4j)

- Repositories are in `src/infrastructure/persistence/`
- Repositories MUST implement port traits from `application/ports/outbound/`
- Use Cypher queries with parameterized values
- Map Neo4j results to domain types at repository boundary

### LLM Integration

- Ollama client in `src/infrastructure/llm/`
- Client implements `LlmPort` trait
- Application services depend on `LlmPort`, not `OllamaClient`

### DTOs

- Define in `application/dto/` for cross-layer data transfer
- Infrastructure layer uses DTOs for API responses/requests
- Map domain types ↔ DTOs at boundaries

## File Placement Rules

| If you're creating... | Put it in... |
|-----------------------|--------------|
| Business entity (World, Character) | `domain/entities/` |
| ID type, value object | `domain/value_objects/` |
| Pure business logic | `domain/services/` |
| Use case trait | `application/ports/inbound/` |
| Repository trait | `application/ports/outbound/` |
| Business orchestration | `application/services/` |
| API request/response types | `application/dto/` |
| Axum route handlers | `infrastructure/http/` |
| Neo4j implementation | `infrastructure/persistence/` |
| External API client | `infrastructure/` (with port trait) |

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

## Architecture Violations to Avoid

1. **Importing infrastructure in application layer**
   ```rust
   // WRONG in application/services/*.rs
   use crate::infrastructure::persistence::Neo4jRepository;
   ```

2. **HTTP handlers calling repositories directly**
   ```rust
   // WRONG in infrastructure/http/*.rs
   state.repository.worlds().list().await
   ```

3. **Serde on domain types**
   ```rust
   // WRONG in domain/entities/*.rs
   #[derive(Serialize, Deserialize)]
   pub struct Character { ... }
   ```

4. **Concrete types instead of traits**
   ```rust
   // WRONG
   pub fn new(repo: Neo4jRepository) -> Self
   // CORRECT
   pub fn new(repo: impl WorldRepository) -> Self
   ```

## See Also

- `/home/otto/repos/WrldBldr/plans/Hexagonal_refactor.md` - Detailed refactoring plan
- `/home/otto/repos/WrldBldr/plans/CLAUDE.md` - Project planning conventions
