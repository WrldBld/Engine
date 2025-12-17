//! Skill repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::SkillRepositoryPort;
use crate::domain::entities::{Skill, SkillCategory};
use crate::domain::value_objects::{SkillId, WorldId};

/// Repository for Skill operations
pub struct Neo4jSkillRepository {
    connection: Neo4jConnection,
}

impl Neo4jSkillRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new skill
    pub async fn create(&self, skill: &Skill) -> Result<()> {
        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (s:Skill {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                category: $category,
                base_attribute: $base_attribute,
                is_custom: $is_custom,
                is_hidden: $is_hidden,
                skill_order: $skill_order
            })
            CREATE (w)-[:HAS_SKILL]->(s)
            RETURN s.id as id",
        )
        .param("id", skill.id.to_string())
        .param("world_id", skill.world_id.to_string())
        .param("name", skill.name.clone())
        .param("description", skill.description.clone())
        .param("category", format!("{:?}", skill.category))
        .param(
            "base_attribute",
            skill.base_attribute.clone().unwrap_or_default(),
        )
        .param("is_custom", skill.is_custom)
        .param("is_hidden", skill.is_hidden)
        .param("skill_order", skill.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created skill: {}", skill.name);
        Ok(())
    }

    /// Get a skill by ID
    pub async fn get(&self, id: SkillId) -> Result<Option<Skill>> {
        let q = query(
            "MATCH (s:Skill {id: $id})
            RETURN s",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_skill(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all skills for a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Skill>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_SKILL]->(s:Skill)
            RETURN s
            ORDER BY s.skill_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut skills = Vec::new();

        while let Some(row) = result.next().await? {
            skills.push(row_to_skill(row)?);
        }

        Ok(skills)
    }

    /// Update a skill
    pub async fn update(&self, skill: &Skill) -> Result<()> {
        let q = query(
            "MATCH (s:Skill {id: $id})
            SET s.name = $name,
                s.description = $description,
                s.category = $category,
                s.base_attribute = $base_attribute,
                s.is_hidden = $is_hidden,
                s.skill_order = $skill_order
            RETURN s.id as id",
        )
        .param("id", skill.id.to_string())
        .param("name", skill.name.clone())
        .param("description", skill.description.clone())
        .param("category", format!("{:?}", skill.category))
        .param(
            "base_attribute",
            skill.base_attribute.clone().unwrap_or_default(),
        )
        .param("is_hidden", skill.is_hidden)
        .param("skill_order", skill.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated skill: {}", skill.name);
        Ok(())
    }

    /// Delete a skill
    pub async fn delete(&self, id: SkillId) -> Result<()> {
        let q = query(
            "MATCH (s:Skill {id: $id})
            DETACH DELETE s",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted skill: {}", id);
        Ok(())
    }
}

/// Convert a Neo4j row to a Skill
fn row_to_skill(row: Row) -> Result<Skill> {
    let node: neo4rs::Node = row.get("s")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let category_str: String = node.get("category")?;
    let base_attribute: String = node.get("base_attribute").unwrap_or_default();
    let is_custom: bool = node.get("is_custom").unwrap_or(false);
    let is_hidden: bool = node.get("is_hidden").unwrap_or(false);
    let order: i64 = node.get("skill_order").unwrap_or(0);

    Ok(Skill {
        id: SkillId::from_uuid(uuid::Uuid::parse_str(&id_str)?),
        world_id: WorldId::from_uuid(uuid::Uuid::parse_str(&world_id_str)?),
        name,
        description,
        category: parse_skill_category(&category_str),
        base_attribute: if base_attribute.is_empty() {
            None
        } else {
            Some(base_attribute)
        },
        is_custom,
        is_hidden,
        order: order as u32,
    })
}

/// Parse a SkillCategory from string
fn parse_skill_category(s: &str) -> SkillCategory {
    match s {
        "Physical" => SkillCategory::Physical,
        "Mental" => SkillCategory::Mental,
        "Social" => SkillCategory::Social,
        "Interpersonal" => SkillCategory::Interpersonal,
        "Investigation" => SkillCategory::Investigation,
        "Academic" => SkillCategory::Academic,
        "Practical" => SkillCategory::Practical,
        "Combat" => SkillCategory::Combat,
        "Approach" => SkillCategory::Approach,
        "Aspect" => SkillCategory::Aspect,
        "Custom" => SkillCategory::Custom,
        _ => SkillCategory::Other,
    }
}

// =============================================================================
// SkillRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl SkillRepositoryPort for Neo4jSkillRepository {
    async fn create(&self, skill: &Skill) -> Result<()> {
        Neo4jSkillRepository::create(self, skill).await
    }

    async fn get(&self, id: SkillId) -> Result<Option<Skill>> {
        Neo4jSkillRepository::get(self, id).await
    }

    async fn list(&self, world_id: WorldId) -> Result<Vec<Skill>> {
        Neo4jSkillRepository::list_by_world(self, world_id).await
    }

    async fn update(&self, skill: &Skill) -> Result<()> {
        Neo4jSkillRepository::update(self, skill).await
    }

    async fn delete(&self, id: SkillId) -> Result<()> {
        Neo4jSkillRepository::delete(self, id).await
    }
}
