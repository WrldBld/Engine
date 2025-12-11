//! Neo4j repository for InteractionTemplate entities

use anyhow::Result;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::domain::entities::{
    InteractionCondition, InteractionTarget, InteractionTemplate, InteractionType,
};
use crate::domain::value_objects::{CharacterId, InteractionId, ItemId, SceneId};

/// Repository for InteractionTemplate operations
pub struct Neo4jInteractionRepository {
    connection: Neo4jConnection,
}

impl Neo4jInteractionRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new interaction template
    pub async fn create(&self, interaction: &InteractionTemplate) -> Result<()> {
        let type_json = serde_json::to_string(&interaction.interaction_type)?;
        let target_json = serde_json::to_string(&interaction.target)?;
        let conditions_json = serde_json::to_string(&interaction.conditions)?;
        let allowed_tools_json = serde_json::to_string(&interaction.allowed_tools)?;

        let q = query(
            "MATCH (s:Scene {id: $scene_id})
            CREATE (i:Interaction {
                id: $id,
                scene_id: $scene_id,
                name: $name,
                interaction_type: $interaction_type,
                target: $target,
                prompt_hints: $prompt_hints,
                allowed_tools: $allowed_tools,
                conditions: $conditions,
                is_available: $is_available,
                order: $order
            })
            CREATE (s)-[:HAS_INTERACTION]->(i)
            RETURN i.id as id",
        )
        .param("id", interaction.id.to_string())
        .param("scene_id", interaction.scene_id.to_string())
        .param("name", interaction.name.clone())
        .param("interaction_type", type_json)
        .param("target", target_json)
        .param("prompt_hints", interaction.prompt_hints.clone())
        .param("allowed_tools", allowed_tools_json)
        .param("conditions", conditions_json)
        .param("is_available", interaction.is_available)
        .param("order", interaction.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created interaction: {}", interaction.id);
        Ok(())
    }

    /// Get an interaction by ID
    pub async fn get(&self, id: InteractionId) -> Result<Option<InteractionTemplate>> {
        let q = query(
            "MATCH (i:Interaction {id: $id})
            RETURN i.id as id,
                   i.scene_id as scene_id,
                   i.name as name,
                   i.interaction_type as interaction_type,
                   i.target as target,
                   i.prompt_hints as prompt_hints,
                   i.allowed_tools as allowed_tools,
                   i.conditions as conditions,
                   i.is_available as is_available,
                   i.order as order",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_interaction(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all interactions for a scene
    pub async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>> {
        let q = query(
            "MATCH (i:Interaction {scene_id: $scene_id})
            RETURN i.id as id,
                   i.scene_id as scene_id,
                   i.name as name,
                   i.interaction_type as interaction_type,
                   i.target as target,
                   i.prompt_hints as prompt_hints,
                   i.allowed_tools as allowed_tools,
                   i.conditions as conditions,
                   i.is_available as is_available,
                   i.order as order
            ORDER BY i.order",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut interactions = Vec::new();

        while let Some(row) = result.next().await? {
            interactions.push(row_to_interaction(row)?);
        }

        Ok(interactions)
    }

    /// Update an interaction
    pub async fn update(&self, interaction: &InteractionTemplate) -> Result<()> {
        let type_json = serde_json::to_string(&interaction.interaction_type)?;
        let target_json = serde_json::to_string(&interaction.target)?;
        let conditions_json = serde_json::to_string(&interaction.conditions)?;
        let allowed_tools_json = serde_json::to_string(&interaction.allowed_tools)?;

        let q = query(
            "MATCH (i:Interaction {id: $id})
            SET i.name = $name,
                i.interaction_type = $interaction_type,
                i.target = $target,
                i.prompt_hints = $prompt_hints,
                i.allowed_tools = $allowed_tools,
                i.conditions = $conditions,
                i.is_available = $is_available,
                i.order = $order
            RETURN i.id as id",
        )
        .param("id", interaction.id.to_string())
        .param("name", interaction.name.clone())
        .param("interaction_type", type_json)
        .param("target", target_json)
        .param("prompt_hints", interaction.prompt_hints.clone())
        .param("allowed_tools", allowed_tools_json)
        .param("conditions", conditions_json)
        .param("is_available", interaction.is_available)
        .param("order", interaction.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated interaction: {}", interaction.id);
        Ok(())
    }

    /// Delete an interaction
    pub async fn delete(&self, id: InteractionId) -> Result<()> {
        let q = query(
            "MATCH (i:Interaction {id: $id})
            DETACH DELETE i",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted interaction: {}", id);
        Ok(())
    }

    /// Toggle availability of an interaction
    pub async fn set_availability(&self, id: InteractionId, available: bool) -> Result<()> {
        let q = query(
            "MATCH (i:Interaction {id: $id})
            SET i.is_available = $available
            RETURN i.id as id",
        )
        .param("id", id.to_string())
        .param("available", available);

        self.connection.graph().run(q).await?;
        Ok(())
    }
}

fn row_to_interaction(row: Row) -> Result<InteractionTemplate> {
    let id_str: String = row.get("id")?;
    let scene_id_str: String = row.get("scene_id")?;
    let name: String = row.get("name")?;
    let type_json: String = row.get("interaction_type")?;
    let target_json: String = row.get("target")?;
    let prompt_hints: String = row.get("prompt_hints")?;
    let allowed_tools_json: String = row.get("allowed_tools")?;
    let conditions_json: String = row.get("conditions")?;
    let is_available: bool = row.get("is_available")?;
    let order: i64 = row.get("order")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let scene_id = uuid::Uuid::parse_str(&scene_id_str)?;
    let interaction_type: InteractionType = serde_json::from_str(&type_json)?;
    let target: InteractionTarget = serde_json::from_str(&target_json)?;
    let allowed_tools: Vec<String> = serde_json::from_str(&allowed_tools_json)?;
    let conditions: Vec<InteractionCondition> = serde_json::from_str(&conditions_json)?;

    Ok(InteractionTemplate {
        id: InteractionId::from_uuid(id),
        scene_id: SceneId::from_uuid(scene_id),
        name,
        interaction_type,
        target,
        prompt_hints,
        allowed_tools,
        conditions,
        is_available,
        order: order as u32,
    })
}
