//! Neo4j connection management

use anyhow::Result;
use neo4rs::{ConfigBuilder, Graph};

/// Shared Neo4j connection
#[derive(Clone)]
pub struct Neo4jConnection {
    graph: Graph,
}

impl Neo4jConnection {
    pub async fn new(uri: &str, user: &str, password: &str, database: &str) -> Result<Self> {
        let config = ConfigBuilder::default()
            .uri(uri)
            .user(user)
            .password(password)
            .db(database)
            .build()?;

        let graph = Graph::connect(config).await?;
        tracing::info!("Connected to Neo4j at {}", uri);

        Ok(Self { graph })
    }

    /// Get a reference to the graph connection
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Initialize the database schema (create constraints and indexes)
    pub async fn initialize_schema(&self) -> Result<()> {
        // Create uniqueness constraints
        let constraints = [
            "CREATE CONSTRAINT world_id IF NOT EXISTS FOR (w:World) REQUIRE w.id IS UNIQUE",
            "CREATE CONSTRAINT location_id IF NOT EXISTS FOR (l:Location) REQUIRE l.id IS UNIQUE",
            "CREATE CONSTRAINT character_id IF NOT EXISTS FOR (c:Character) REQUIRE c.id IS UNIQUE",
            "CREATE CONSTRAINT scene_id IF NOT EXISTS FOR (s:Scene) REQUIRE s.id IS UNIQUE",
            "CREATE CONSTRAINT act_id IF NOT EXISTS FOR (a:Act) REQUIRE a.id IS UNIQUE",
            "CREATE CONSTRAINT item_id IF NOT EXISTS FOR (i:Item) REQUIRE i.id IS UNIQUE",
            "CREATE CONSTRAINT grid_map_id IF NOT EXISTS FOR (g:GridMap) REQUIRE g.id IS UNIQUE",
        ];

        for constraint in constraints {
            if let Err(e) = self.graph.run(neo4rs::query(constraint)).await {
                tracing::warn!("Constraint creation warning: {}", e);
            }
        }

        // Create indexes for common queries
        let indexes = [
            "CREATE INDEX world_name IF NOT EXISTS FOR (w:World) ON (w.name)",
            "CREATE INDEX character_name IF NOT EXISTS FOR (c:Character) ON (c.name)",
            "CREATE INDEX location_name IF NOT EXISTS FOR (l:Location) ON (l.name)",
            "CREATE INDEX character_world IF NOT EXISTS FOR (c:Character) ON (c.world_id)",
            "CREATE INDEX location_world IF NOT EXISTS FOR (l:Location) ON (l.world_id)",
            "CREATE INDEX scene_act IF NOT EXISTS FOR (s:Scene) ON (s.act_id)",
        ];

        for index in indexes {
            if let Err(e) = self.graph.run(neo4rs::query(index)).await {
                tracing::warn!("Index creation warning: {}", e);
            }
        }

        tracing::info!("Database schema initialized");
        Ok(())
    }
}
