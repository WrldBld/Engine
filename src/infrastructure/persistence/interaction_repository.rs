//! Neo4j repository for InteractionTemplate entities

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::InteractionRepositoryPort;
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
        let type_json =
            serde_json::to_string(&InteractionTypeStored::from(interaction.interaction_type.clone()))?;
        let target_json =
            serde_json::to_string(&InteractionTargetStored::try_from(interaction.target.clone())?)?;
        let conditions_json = serde_json::to_string(
            &interaction
                .conditions
                .iter()
                .cloned()
                .map(InteractionConditionStored::try_from)
                .collect::<Result<Vec<_>>>()?,
        )?;
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
        let type_json =
            serde_json::to_string(&InteractionTypeStored::from(interaction.interaction_type.clone()))?;
        let target_json =
            serde_json::to_string(&InteractionTargetStored::try_from(interaction.target.clone())?)?;
        let conditions_json = serde_json::to_string(
            &interaction
                .conditions
                .iter()
                .cloned()
                .map(InteractionConditionStored::try_from)
                .collect::<Result<Vec<_>>>()?,
        )?;
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
    let interaction_type: InteractionType =
        serde_json::from_str::<InteractionTypeStored>(&type_json)?.into();
    let target: InteractionTarget =
        serde_json::from_str::<InteractionTargetStored>(&target_json)?.try_into()?;
    let allowed_tools: Vec<String> = serde_json::from_str(&allowed_tools_json)?;
    let conditions: Vec<InteractionCondition> =
        serde_json::from_str::<Vec<InteractionConditionStored>>(&conditions_json)?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;

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

// ============================================================================
// Persistence serde models (so domain doesn't require serde)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
enum InteractionTypeStored {
    Dialogue,
    Examine,
    UseItem,
    PickUp,
    GiveItem,
    Attack,
    Travel,
    Custom(String),
}

impl From<InteractionType> for InteractionTypeStored {
    fn from(value: InteractionType) -> Self {
        match value {
            InteractionType::Dialogue => Self::Dialogue,
            InteractionType::Examine => Self::Examine,
            InteractionType::UseItem => Self::UseItem,
            InteractionType::PickUp => Self::PickUp,
            InteractionType::GiveItem => Self::GiveItem,
            InteractionType::Attack => Self::Attack,
            InteractionType::Travel => Self::Travel,
            InteractionType::Custom(s) => Self::Custom(s),
        }
    }
}

impl From<InteractionTypeStored> for InteractionType {
    fn from(value: InteractionTypeStored) -> Self {
        match value {
            InteractionTypeStored::Dialogue => Self::Dialogue,
            InteractionTypeStored::Examine => Self::Examine,
            InteractionTypeStored::UseItem => Self::UseItem,
            InteractionTypeStored::PickUp => Self::PickUp,
            InteractionTypeStored::GiveItem => Self::GiveItem,
            InteractionTypeStored::Attack => Self::Attack,
            InteractionTypeStored::Travel => Self::Travel,
            InteractionTypeStored::Custom(s) => Self::Custom(s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum InteractionTargetStored {
    Character(String),
    Item(String),
    Environment(String),
    None,
}

impl TryFrom<InteractionTarget> for InteractionTargetStored {
    type Error = anyhow::Error;

    fn try_from(value: InteractionTarget) -> Result<Self> {
        Ok(match value {
            InteractionTarget::Character(id) => Self::Character(id.to_string()),
            InteractionTarget::Item(id) => Self::Item(id.to_string()),
            InteractionTarget::Environment(s) => Self::Environment(s),
            InteractionTarget::None => Self::None,
        })
    }
}

impl TryFrom<InteractionTargetStored> for InteractionTarget {
    type Error = anyhow::Error;

    fn try_from(value: InteractionTargetStored) -> Result<Self> {
        Ok(match value {
            InteractionTargetStored::Character(id) => {
                InteractionTarget::Character(CharacterId::from_uuid(uuid::Uuid::parse_str(&id)?))
            }
            InteractionTargetStored::Item(id) => {
                InteractionTarget::Item(ItemId::from_uuid(uuid::Uuid::parse_str(&id)?))
            }
            InteractionTargetStored::Environment(s) => InteractionTarget::Environment(s),
            InteractionTargetStored::None => InteractionTarget::None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum InteractionConditionStored {
    HasItem(String),
    CharacterPresent(String),
    HasRelationship {
        with_character: String,
        relationship_type: Option<String>,
    },
    FlagSet(String),
    FlagNotSet(String),
    Custom(String),
}

impl TryFrom<InteractionCondition> for InteractionConditionStored {
    type Error = anyhow::Error;

    fn try_from(value: InteractionCondition) -> Result<Self> {
        Ok(match value {
            InteractionCondition::HasItem(item_id) => Self::HasItem(item_id.to_string()),
            InteractionCondition::CharacterPresent(char_id) => {
                Self::CharacterPresent(char_id.to_string())
            }
            InteractionCondition::HasRelationship {
                with_character,
                relationship_type,
            } => Self::HasRelationship {
                with_character: with_character.to_string(),
                relationship_type,
            },
            InteractionCondition::FlagSet(s) => Self::FlagSet(s),
            InteractionCondition::FlagNotSet(s) => Self::FlagNotSet(s),
            InteractionCondition::Custom(s) => Self::Custom(s),
        })
    }
}

impl TryFrom<InteractionConditionStored> for InteractionCondition {
    type Error = anyhow::Error;

    fn try_from(value: InteractionConditionStored) -> Result<Self> {
        Ok(match value {
            InteractionConditionStored::HasItem(id) => {
                InteractionCondition::HasItem(ItemId::from_uuid(uuid::Uuid::parse_str(&id)?))
            }
            InteractionConditionStored::CharacterPresent(id) => InteractionCondition::CharacterPresent(
                CharacterId::from_uuid(uuid::Uuid::parse_str(&id)?),
            ),
            InteractionConditionStored::HasRelationship {
                with_character,
                relationship_type,
            } => InteractionCondition::HasRelationship {
                with_character: CharacterId::from_uuid(uuid::Uuid::parse_str(&with_character)?),
                relationship_type,
            },
            InteractionConditionStored::FlagSet(s) => InteractionCondition::FlagSet(s),
            InteractionConditionStored::FlagNotSet(s) => InteractionCondition::FlagNotSet(s),
            InteractionConditionStored::Custom(s) => InteractionCondition::Custom(s),
        })
    }
}

// =============================================================================
// InteractionRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl InteractionRepositoryPort for Neo4jInteractionRepository {
    async fn create(&self, interaction: &InteractionTemplate) -> Result<()> {
        Neo4jInteractionRepository::create(self, interaction).await
    }

    async fn get(&self, id: InteractionId) -> Result<Option<InteractionTemplate>> {
        Neo4jInteractionRepository::get(self, id).await
    }

    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>> {
        Neo4jInteractionRepository::list_by_scene(self, scene_id).await
    }

    async fn update(&self, interaction: &InteractionTemplate) -> Result<()> {
        Neo4jInteractionRepository::update(self, interaction).await
    }

    async fn delete(&self, id: InteractionId) -> Result<()> {
        Neo4jInteractionRepository::delete(self, id).await
    }

    // Target edge methods - implemented as stubs for now
    // TODO: Implement full edge-based targeting in Phase 0.H

    async fn set_target_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()> {
        // Remove any existing target first
        self.remove_target(interaction_id).await?;

        let q = query(
            "MATCH (i:Interaction {id: $interaction_id}), (c:Character {id: $character_id})
            CREATE (i)-[:TARGETS_CHARACTER]->(c)",
        )
        .param("interaction_id", interaction_id.to_string())
        .param("character_id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn set_target_item(
        &self,
        interaction_id: InteractionId,
        item_id: ItemId,
    ) -> Result<()> {
        self.remove_target(interaction_id).await?;

        let q = query(
            "MATCH (i:Interaction {id: $interaction_id}), (t:Item {id: $item_id})
            CREATE (i)-[:TARGETS_ITEM]->(t)",
        )
        .param("interaction_id", interaction_id.to_string())
        .param("item_id", item_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn set_target_region(
        &self,
        interaction_id: InteractionId,
        region_id: crate::domain::value_objects::RegionId,
    ) -> Result<()> {
        self.remove_target(interaction_id).await?;

        let q = query(
            "MATCH (i:Interaction {id: $interaction_id}), (r:Region {id: $region_id})
            CREATE (i)-[:TARGETS_REGION]->(r)",
        )
        .param("interaction_id", interaction_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn remove_target(&self, interaction_id: InteractionId) -> Result<()> {
        let q = query(
            "MATCH (i:Interaction {id: $id})-[r]->()
            WHERE type(r) IN ['TARGETS_CHARACTER', 'TARGETS_ITEM', 'TARGETS_REGION']
            DELETE r",
        )
        .param("id", interaction_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn get_target(
        &self,
        interaction_id: InteractionId,
    ) -> Result<Option<(crate::domain::entities::InteractionTargetType, String)>> {
        let q = query(
            "MATCH (i:Interaction {id: $id})-[r]->(t)
            WHERE type(r) IN ['TARGETS_CHARACTER', 'TARGETS_ITEM', 'TARGETS_REGION']
            RETURN type(r) as edge_type, t.id as target_id",
        )
        .param("id", interaction_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let edge_type: String = row.get("edge_type")?;
            let target_id: String = row.get("target_id")?;

            let target_type = match edge_type.as_str() {
                "TARGETS_CHARACTER" => crate::domain::entities::InteractionTargetType::Character,
                "TARGETS_ITEM" => crate::domain::entities::InteractionTargetType::Item,
                "TARGETS_REGION" => crate::domain::entities::InteractionTargetType::Region,
                _ => return Ok(None),
            };

            Ok(Some((target_type, target_id)))
        } else {
            Ok(None)
        }
    }

    async fn add_required_item(
        &self,
        interaction_id: InteractionId,
        item_id: ItemId,
        requirement: &crate::domain::entities::InteractionRequirement,
    ) -> Result<()> {
        let q = query(
            "MATCH (i:Interaction {id: $interaction_id}), (t:Item {id: $item_id})
            CREATE (i)-[:REQUIRES_ITEM {consumed: $consumed}]->(t)",
        )
        .param("interaction_id", interaction_id.to_string())
        .param("item_id", item_id.to_string())
        .param("consumed", requirement.consumed);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn remove_required_item(
        &self,
        interaction_id: InteractionId,
        item_id: ItemId,
    ) -> Result<()> {
        let q = query(
            "MATCH (i:Interaction {id: $interaction_id})-[r:REQUIRES_ITEM]->(t:Item {id: $item_id})
            DELETE r",
        )
        .param("interaction_id", interaction_id.to_string())
        .param("item_id", item_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn add_required_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()> {
        let q = query(
            "MATCH (i:Interaction {id: $interaction_id}), (c:Character {id: $character_id})
            CREATE (i)-[:REQUIRES_CHARACTER_PRESENT]->(c)",
        )
        .param("interaction_id", interaction_id.to_string())
        .param("character_id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn remove_required_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()> {
        let q = query(
            "MATCH (i:Interaction {id: $interaction_id})-[r:REQUIRES_CHARACTER_PRESENT]->(c:Character {id: $character_id})
            DELETE r",
        )
        .param("interaction_id", interaction_id.to_string())
        .param("character_id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }
}
