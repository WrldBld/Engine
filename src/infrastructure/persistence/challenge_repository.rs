//! Challenge repository implementation for Neo4j

use anyhow::Result;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::domain::entities::{
    Challenge, ChallengeOutcomes, ChallengeType, Difficulty, TriggerCondition,
};
use crate::domain::value_objects::{ChallengeId, SceneId, SkillId, WorldId};

/// Repository for Challenge operations
pub struct Neo4jChallengeRepository {
    connection: Neo4jConnection,
}

impl Neo4jChallengeRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new challenge
    pub async fn create(&self, challenge: &Challenge) -> Result<()> {
        // Serialize complex fields as JSON
        let outcomes_json = serde_json::to_string(&challenge.outcomes)?;
        let triggers_json = serde_json::to_string(&challenge.trigger_conditions)?;
        let prerequisites_json: Vec<String> = challenge
            .prerequisite_challenges
            .iter()
            .map(|id| id.to_string())
            .collect();
        let tags_json = serde_json::to_string(&challenge.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (c:Challenge {
                id: $id,
                world_id: $world_id,
                scene_id: $scene_id,
                name: $name,
                description: $description,
                challenge_type: $challenge_type,
                skill_id: $skill_id,
                difficulty_json: $difficulty_json,
                outcomes_json: $outcomes_json,
                triggers_json: $triggers_json,
                prerequisites: $prerequisites,
                active: $active,
                challenge_order: $challenge_order,
                is_favorite: $is_favorite,
                tags_json: $tags_json
            })
            CREATE (w)-[:HAS_CHALLENGE]->(c)
            RETURN c.id as id",
        )
        .param("id", challenge.id.to_string())
        .param("world_id", challenge.world_id.to_string())
        .param(
            "scene_id",
            challenge.scene_id.map(|s| s.to_string()).unwrap_or_default(),
        )
        .param("name", challenge.name.clone())
        .param("description", challenge.description.clone())
        .param("challenge_type", format!("{:?}", challenge.challenge_type))
        .param("skill_id", challenge.skill_id.to_string())
        .param("difficulty_json", serde_json::to_string(&challenge.difficulty)?)
        .param("outcomes_json", outcomes_json)
        .param("triggers_json", triggers_json)
        .param("prerequisites", prerequisites_json)
        .param("active", challenge.active)
        .param("challenge_order", challenge.order as i64)
        .param("is_favorite", challenge.is_favorite)
        .param("tags_json", tags_json);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created challenge: {}", challenge.name);

        // Create relationship to scene if specified
        if let Some(scene_id) = challenge.scene_id {
            let scene_q = query(
                "MATCH (c:Challenge {id: $challenge_id}), (s:Scene {id: $scene_id})
                MERGE (s)-[:HAS_CHALLENGE]->(c)",
            )
            .param("challenge_id", challenge.id.to_string())
            .param("scene_id", scene_id.to_string());

            self.connection.graph().run(scene_q).await?;
        }

        Ok(())
    }

    /// Get a challenge by ID
    pub async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            RETURN c",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_challenge(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all challenges for a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_CHALLENGE]->(c:Challenge)
            RETURN c
            ORDER BY c.is_favorite DESC, c.challenge_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    /// List challenges for a specific scene
    pub async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[:HAS_CHALLENGE]->(c:Challenge)
            RETURN c
            ORDER BY c.is_favorite DESC, c.challenge_order",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    /// List active challenges for a world (for LLM context)
    pub async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_CHALLENGE]->(c:Challenge {active: true})
            RETURN c
            ORDER BY c.challenge_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    /// List favorite challenges for quick access
    pub async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_CHALLENGE]->(c:Challenge {is_favorite: true})
            RETURN c
            ORDER BY c.challenge_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    /// Update a challenge
    pub async fn update(&self, challenge: &Challenge) -> Result<()> {
        let outcomes_json = serde_json::to_string(&challenge.outcomes)?;
        let triggers_json = serde_json::to_string(&challenge.trigger_conditions)?;
        let prerequisites_json: Vec<String> = challenge
            .prerequisite_challenges
            .iter()
            .map(|id| id.to_string())
            .collect();
        let tags_json = serde_json::to_string(&challenge.tags)?;

        let q = query(
            "MATCH (c:Challenge {id: $id})
            SET c.name = $name,
                c.description = $description,
                c.scene_id = $scene_id,
                c.challenge_type = $challenge_type,
                c.skill_id = $skill_id,
                c.difficulty_json = $difficulty_json,
                c.outcomes_json = $outcomes_json,
                c.triggers_json = $triggers_json,
                c.prerequisites = $prerequisites,
                c.active = $active,
                c.challenge_order = $challenge_order,
                c.is_favorite = $is_favorite,
                c.tags_json = $tags_json
            RETURN c.id as id",
        )
        .param("id", challenge.id.to_string())
        .param("name", challenge.name.clone())
        .param("description", challenge.description.clone())
        .param(
            "scene_id",
            challenge.scene_id.map(|s| s.to_string()).unwrap_or_default(),
        )
        .param("challenge_type", format!("{:?}", challenge.challenge_type))
        .param("skill_id", challenge.skill_id.to_string())
        .param("difficulty_json", serde_json::to_string(&challenge.difficulty)?)
        .param("outcomes_json", outcomes_json)
        .param("triggers_json", triggers_json)
        .param("prerequisites", prerequisites_json)
        .param("active", challenge.active)
        .param("challenge_order", challenge.order as i64)
        .param("is_favorite", challenge.is_favorite)
        .param("tags_json", tags_json);

        self.connection.graph().run(q).await?;

        // Update scene relationship if changed
        // First remove existing scene relationship
        let remove_q = query(
            "MATCH (s:Scene)-[r:HAS_CHALLENGE]->(c:Challenge {id: $challenge_id})
            DELETE r",
        )
        .param("challenge_id", challenge.id.to_string());
        self.connection.graph().run(remove_q).await?;

        // Then add new one if specified
        if let Some(scene_id) = challenge.scene_id {
            let add_q = query(
                "MATCH (c:Challenge {id: $challenge_id}), (s:Scene {id: $scene_id})
                MERGE (s)-[:HAS_CHALLENGE]->(c)",
            )
            .param("challenge_id", challenge.id.to_string())
            .param("scene_id", scene_id.to_string());
            self.connection.graph().run(add_q).await?;
        }

        tracing::debug!("Updated challenge: {}", challenge.name);
        Ok(())
    }

    /// Delete a challenge
    pub async fn delete(&self, id: ChallengeId) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            DETACH DELETE c",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted challenge: {}", id);
        Ok(())
    }

    /// Set active status for a challenge
    pub async fn set_active(&self, id: ChallengeId, active: bool) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            SET c.active = $active
            RETURN c.id as id",
        )
        .param("id", id.to_string())
        .param("active", active);

        self.connection.graph().run(q).await?;
        tracing::debug!("Set challenge {} active: {}", id, active);
        Ok(())
    }

    /// Toggle favorite status
    pub async fn toggle_favorite(&self, id: ChallengeId) -> Result<bool> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            SET c.is_favorite = NOT coalesce(c.is_favorite, false)
            RETURN c.is_favorite as is_favorite",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let is_favorite: bool = row.get("is_favorite")?;
            Ok(is_favorite)
        } else {
            Ok(false)
        }
    }

    /// Delete all challenges for a world
    pub async fn delete_all_for_world(&self, world_id: WorldId) -> Result<()> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_CHALLENGE]->(c:Challenge)
            DETACH DELETE c",
        )
        .param("world_id", world_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted all challenges for world: {}", world_id);
        Ok(())
    }
}

/// Convert a Neo4j row to a Challenge
fn row_to_challenge(row: Row) -> Result<Challenge> {
    let node: neo4rs::Node = row.get("c")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let scene_id_str: String = node.get("scene_id").unwrap_or_default();
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let challenge_type_str: String = node.get("challenge_type")?;
    let skill_id_str: String = node.get("skill_id")?;
    let difficulty_json: String = node.get("difficulty_json")?;
    let outcomes_json: String = node.get("outcomes_json")?;
    let triggers_json: String = node.get("triggers_json")?;
    let prerequisites: Vec<String> = node.get("prerequisites").unwrap_or_default();
    let active: bool = node.get("active").unwrap_or(true);
    let order: i64 = node.get("challenge_order").unwrap_or(0);
    let is_favorite: bool = node.get("is_favorite").unwrap_or(false);
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());

    // Parse scene_id if present
    let scene_id = if scene_id_str.is_empty() {
        None
    } else {
        Some(SceneId::from_uuid(uuid::Uuid::parse_str(&scene_id_str)?))
    };

    // Parse prerequisites
    let prerequisite_challenges: Vec<ChallengeId> = prerequisites
        .into_iter()
        .filter_map(|s| uuid::Uuid::parse_str(&s).ok().map(ChallengeId::from_uuid))
        .collect();

    Ok(Challenge {
        id: ChallengeId::from_uuid(uuid::Uuid::parse_str(&id_str)?),
        world_id: WorldId::from_uuid(uuid::Uuid::parse_str(&world_id_str)?),
        scene_id,
        name,
        description,
        challenge_type: parse_challenge_type(&challenge_type_str),
        skill_id: SkillId::from_uuid(uuid::Uuid::parse_str(&skill_id_str)?),
        difficulty: serde_json::from_str(&difficulty_json)?,
        outcomes: serde_json::from_str(&outcomes_json)?,
        trigger_conditions: serde_json::from_str(&triggers_json)?,
        active,
        prerequisite_challenges,
        order: order as u32,
        is_favorite,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
    })
}

/// Parse ChallengeType from string
fn parse_challenge_type(s: &str) -> ChallengeType {
    match s {
        "SkillCheck" => ChallengeType::SkillCheck,
        "AbilityCheck" => ChallengeType::AbilityCheck,
        "SavingThrow" => ChallengeType::SavingThrow,
        "OpposedCheck" => ChallengeType::OpposedCheck,
        "ComplexChallenge" => ChallengeType::ComplexChallenge,
        _ => ChallengeType::SkillCheck,
    }
}
