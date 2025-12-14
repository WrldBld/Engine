//! World repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::application::dto::RuleSystemConfigDto;
use crate::application::ports::outbound::WorldRepositoryPort;
use crate::domain::entities::{Act, MonomythStage, World};
use crate::domain::value_objects::{ActId, RuleSystemConfig, WorldId};

/// Repository for World aggregate operations
pub struct Neo4jWorldRepository {
    connection: Neo4jConnection,
}

impl Neo4jWorldRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new world
    pub async fn create(&self, world: &World) -> Result<()> {
        let rule_system_json =
            serde_json::to_string(&RuleSystemConfigDto::from(world.rule_system.clone()))?;

        let q = query(
            "CREATE (w:World {
                id: $id,
                name: $name,
                description: $description,
                rule_system: $rule_system,
                created_at: $created_at,
                updated_at: $updated_at
            })
            RETURN w.id as id",
        )
        .param("id", world.id.to_string())
        .param("name", world.name.clone())
        .param("description", world.description.clone())
        .param("rule_system", rule_system_json)
        .param("created_at", world.created_at.to_rfc3339())
        .param("updated_at", world.updated_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Created world: {}", world.name);
        Ok(())
    }

    /// Get a world by ID
    pub async fn get(&self, id: WorldId) -> Result<Option<World>> {
        let q = query(
            "MATCH (w:World {id: $id})
            RETURN w.id as id, w.name as name, w.description as description,
                   w.rule_system as rule_system, w.created_at as created_at,
                   w.updated_at as updated_at",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_world(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all worlds
    pub async fn list(&self) -> Result<Vec<World>> {
        let q = query(
            "MATCH (w:World)
            RETURN w.id as id, w.name as name, w.description as description,
                   w.rule_system as rule_system, w.created_at as created_at,
                   w.updated_at as updated_at
            ORDER BY w.name",
        );

        let mut result = self.connection.graph().execute(q).await?;
        let mut worlds = Vec::new();

        while let Some(row) = result.next().await? {
            worlds.push(row_to_world(row)?);
        }

        Ok(worlds)
    }

    /// Update a world
    pub async fn update(&self, world: &World) -> Result<()> {
        let rule_system_json =
            serde_json::to_string(&RuleSystemConfigDto::from(world.rule_system.clone()))?;

        let q = query(
            "MATCH (w:World {id: $id})
            SET w.name = $name,
                w.description = $description,
                w.rule_system = $rule_system,
                w.updated_at = $updated_at
            RETURN w.id as id",
        )
        .param("id", world.id.to_string())
        .param("name", world.name.clone())
        .param("description", world.description.clone())
        .param("rule_system", rule_system_json)
        .param("updated_at", world.updated_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated world: {}", world.name);
        Ok(())
    }

    /// Delete a world and all its contents
    pub async fn delete(&self, id: WorldId) -> Result<()> {
        // Delete all related entities first (cascading delete)
        let q = query(
            "MATCH (w:World {id: $id})
            OPTIONAL MATCH (w)-[*]->(related)
            DETACH DELETE related, w",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted world: {}", id);
        Ok(())
    }

    /// Create an act within a world
    pub async fn create_act(&self, act: &Act) -> Result<()> {
        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (a:Act {
                id: $id,
                world_id: $world_id,
                name: $name,
                stage: $stage,
                description: $description,
                order_num: $order_num
            })
            CREATE (w)-[:CONTAINS_ACT]->(a)
            RETURN a.id as id",
        )
        .param("id", act.id.to_string())
        .param("world_id", act.world_id.to_string())
        .param("name", act.name.clone())
        .param("stage", format!("{:?}", act.stage))
        .param("description", act.description.clone())
        .param("order_num", act.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created act: {}", act.name);
        Ok(())
    }

    /// Get acts for a world
    pub async fn get_acts(&self, world_id: WorldId) -> Result<Vec<Act>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_ACT]->(a:Act)
            RETURN a.id as id, a.world_id as world_id, a.name as name,
                   a.stage as stage, a.description as description, a.order_num as order_num
            ORDER BY a.order_num",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut acts = Vec::new();

        while let Some(row) = result.next().await? {
            acts.push(row_to_act(row)?);
        }

        Ok(acts)
    }
}

fn row_to_world(row: Row) -> Result<World> {
    let id_str: String = row.get("id")?;
    let name: String = row.get("name")?;
    let description: String = row.get("description")?;
    let rule_system_json: String = row.get("rule_system")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let rule_system: RuleSystemConfig =
        serde_json::from_str::<RuleSystemConfigDto>(&rule_system_json)?.into();
    let created_at =
        chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc);
    let updated_at =
        chrono::DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&chrono::Utc);

    Ok(World {
        id: WorldId::from_uuid(id),
        name,
        description,
        rule_system,
        created_at,
        updated_at,
    })
}

fn row_to_act(row: Row) -> Result<Act> {
    let id_str: String = row.get("id")?;
    let world_id_str: String = row.get("world_id")?;
    let name: String = row.get("name")?;
    let stage_str: String = row.get("stage")?;
    let description: String = row.get("description")?;
    let order_num: i64 = row.get("order_num")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = uuid::Uuid::parse_str(&world_id_str)?;

    let stage = match stage_str.as_str() {
        "OrdinaryWorld" => MonomythStage::OrdinaryWorld,
        "CallToAdventure" => MonomythStage::CallToAdventure,
        "RefusalOfTheCall" => MonomythStage::RefusalOfTheCall,
        "MeetingTheMentor" => MonomythStage::MeetingTheMentor,
        "CrossingTheThreshold" => MonomythStage::CrossingTheThreshold,
        "TestsAlliesEnemies" => MonomythStage::TestsAlliesEnemies,
        "ApproachToInnermostCave" => MonomythStage::ApproachToInnermostCave,
        "Ordeal" => MonomythStage::Ordeal,
        "Reward" => MonomythStage::Reward,
        "TheRoadBack" => MonomythStage::TheRoadBack,
        "Resurrection" => MonomythStage::Resurrection,
        "ReturnWithElixir" => MonomythStage::ReturnWithElixir,
        _ => MonomythStage::OrdinaryWorld,
    };

    Ok(Act {
        id: ActId::from_uuid(id),
        world_id: WorldId::from_uuid(world_id),
        name,
        stage,
        description,
        order: order_num as u32,
    })
}

// =============================================================================
// WorldRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl WorldRepositoryPort for Neo4jWorldRepository {
    async fn create(&self, world: &World) -> Result<()> {
        Neo4jWorldRepository::create(self, world).await
    }

    async fn get(&self, id: WorldId) -> Result<Option<World>> {
        Neo4jWorldRepository::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<World>> {
        Neo4jWorldRepository::list(self).await
    }

    async fn update(&self, world: &World) -> Result<()> {
        Neo4jWorldRepository::update(self, world).await
    }

    async fn delete(&self, id: WorldId) -> Result<()> {
        Neo4jWorldRepository::delete(self, id).await
    }

    async fn create_act(&self, act: &Act) -> Result<()> {
        Neo4jWorldRepository::create_act(self, act).await
    }

    async fn get_acts(&self, world_id: WorldId) -> Result<Vec<Act>> {
        Neo4jWorldRepository::get_acts(self, world_id).await
    }
}
