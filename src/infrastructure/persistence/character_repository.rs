//! Character repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::CharacterRepositoryPort;
use crate::domain::entities::Character;
use crate::domain::entities::StatBlock;
use crate::domain::value_objects::{
    ActantTarget, ArchetypeChange, CampbellArchetype, CharacterId, ItemId, SceneId, Want, WantId,
    WorldId,
};

/// Repository for Character operations
pub struct Neo4jCharacterRepository {
    connection: Neo4jConnection,
}

impl Neo4jCharacterRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new character
    pub async fn create(&self, character: &Character) -> Result<()> {
        let wants_json = serde_json::to_string(
            &character
                .wants
                .iter()
                .cloned()
                .map(WantStored::try_from)
                .collect::<Result<Vec<_>>>()?,
        )?;
        let stats_json = serde_json::to_string(&StatBlockStored::from(character.stats.clone()))?;
        let archetype_history_json = serde_json::to_string(
            &character
                .archetype_history
                .iter()
                .cloned()
                .map(ArchetypeChangeStored::from)
                .collect::<Vec<_>>(),
        )?;
        let inventory_json = serde_json::to_string(
            &character
                .inventory
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (c:Character {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                sprite_asset: $sprite_asset,
                portrait_asset: $portrait_asset,
                base_archetype: $base_archetype,
                current_archetype: $current_archetype,
                archetype_history: $archetype_history,
                wants: $wants,
                stats: $stats,
                inventory: $inventory,
                is_alive: $is_alive,
                is_active: $is_active
            })
            CREATE (w)-[:CONTAINS_CHARACTER]->(c)
            RETURN c.id as id",
        )
        .param("id", character.id.to_string())
        .param("world_id", character.world_id.to_string())
        .param("name", character.name.clone())
        .param("description", character.description.clone())
        .param(
            "sprite_asset",
            character.sprite_asset.clone().unwrap_or_default(),
        )
        .param(
            "portrait_asset",
            character.portrait_asset.clone().unwrap_or_default(),
        )
        .param("base_archetype", format!("{:?}", character.base_archetype))
        .param(
            "current_archetype",
            format!("{:?}", character.current_archetype),
        )
        .param("archetype_history", archetype_history_json)
        .param("wants", wants_json)
        .param("stats", stats_json)
        .param("inventory", inventory_json)
        .param("is_alive", character.is_alive)
        .param("is_active", character.is_active);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created character: {}", character.name);
        Ok(())
    }

    /// Get a character by ID
    pub async fn get(&self, id: CharacterId) -> Result<Option<Character>> {
        let q = query(
            "MATCH (c:Character {id: $id})
            RETURN c",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_character(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all characters in a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Character>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHARACTER]->(c:Character)
            RETURN c
            ORDER BY c.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            characters.push(row_to_character(row)?);
        }

        Ok(characters)
    }

    /// Update a character
    pub async fn update(&self, character: &Character) -> Result<()> {
        let wants_json = serde_json::to_string(
            &character
                .wants
                .iter()
                .cloned()
                .map(WantStored::try_from)
                .collect::<Result<Vec<_>>>()?,
        )?;
        let stats_json = serde_json::to_string(&StatBlockStored::from(character.stats.clone()))?;
        let archetype_history_json = serde_json::to_string(
            &character
                .archetype_history
                .iter()
                .cloned()
                .map(ArchetypeChangeStored::from)
                .collect::<Vec<_>>(),
        )?;
        let inventory_json = serde_json::to_string(
            &character
                .inventory
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH (c:Character {id: $id})
            SET c.name = $name,
                c.description = $description,
                c.sprite_asset = $sprite_asset,
                c.portrait_asset = $portrait_asset,
                c.base_archetype = $base_archetype,
                c.current_archetype = $current_archetype,
                c.archetype_history = $archetype_history,
                c.wants = $wants,
                c.stats = $stats,
                c.inventory = $inventory,
                c.is_alive = $is_alive,
                c.is_active = $is_active
            RETURN c.id as id",
        )
        .param("id", character.id.to_string())
        .param("name", character.name.clone())
        .param("description", character.description.clone())
        .param(
            "sprite_asset",
            character.sprite_asset.clone().unwrap_or_default(),
        )
        .param(
            "portrait_asset",
            character.portrait_asset.clone().unwrap_or_default(),
        )
        .param("base_archetype", format!("{:?}", character.base_archetype))
        .param(
            "current_archetype",
            format!("{:?}", character.current_archetype),
        )
        .param("archetype_history", archetype_history_json)
        .param("wants", wants_json)
        .param("stats", stats_json)
        .param("inventory", inventory_json)
        .param("is_alive", character.is_alive)
        .param("is_active", character.is_active);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated character: {}", character.name);
        Ok(())
    }

    /// Delete a character
    pub async fn delete(&self, id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})
            DETACH DELETE c",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted character: {}", id);
        Ok(())
    }

    /// Change a character's archetype
    pub async fn change_archetype(
        &self,
        id: CharacterId,
        new_archetype: CampbellArchetype,
        reason: &str,
    ) -> Result<()> {
        // First get current character to build history
        if let Some(mut character) = self.get(id).await? {
            character.change_archetype(new_archetype, reason);
            self.update(&character).await?;
        }
        Ok(())
    }

    /// Add a want to a character
    pub async fn add_want(&self, id: CharacterId, want: Want) -> Result<()> {
        if let Some(mut character) = self.get(id).await? {
            character.wants.push(want);
            self.update(&character).await?;
        }
        Ok(())
    }
}

fn row_to_character(row: Row) -> Result<Character> {
    let node: neo4rs::Node = row.get("c")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description")?;
    let sprite_asset: String = node.get("sprite_asset")?;
    let portrait_asset: String = node.get("portrait_asset")?;
    let base_archetype_str: String = node.get("base_archetype")?;
    let current_archetype_str: String = node.get("current_archetype")?;
    let archetype_history_json: String = node.get("archetype_history")?;
    let wants_json: String = node.get("wants")?;
    let stats_json: String = node.get("stats")?;
    let inventory_json: String = node.get("inventory")?;
    let is_alive: bool = node.get("is_alive")?;
    let is_active: bool = node.get("is_active")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = uuid::Uuid::parse_str(&world_id_str)?;
    let base_archetype = parse_archetype(&base_archetype_str);
    let current_archetype = parse_archetype(&current_archetype_str);
    let archetype_history: Vec<ArchetypeChange> =
        serde_json::from_str::<Vec<ArchetypeChangeStored>>(&archetype_history_json)?
            .into_iter()
            .map(Into::into)
            .collect();
    let wants: Vec<Want> = serde_json::from_str::<Vec<WantStored>>(&wants_json)?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>>>()?;
    let stats: StatBlock = serde_json::from_str::<StatBlockStored>(&stats_json)?.into();
    let inventory: Vec<ItemId> = serde_json::from_str::<Vec<String>>(&inventory_json)?
        .into_iter()
        .filter_map(|s| uuid::Uuid::parse_str(&s).ok().map(ItemId::from_uuid))
        .collect();

    Ok(Character {
        id: CharacterId::from_uuid(id),
        world_id: WorldId::from_uuid(world_id),
        name,
        description,
        sprite_asset: if sprite_asset.is_empty() {
            None
        } else {
            Some(sprite_asset)
        },
        portrait_asset: if portrait_asset.is_empty() {
            None
        } else {
            Some(portrait_asset)
        },
        base_archetype,
        current_archetype,
        archetype_history,
        wants,
        stats,
        inventory,
        is_alive,
        is_active,
    })
}

// ============================================================================
// Persistence serde models (so domain doesn't require serde)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatBlockStored {
    pub stats: std::collections::HashMap<String, i32>,
    pub current_hp: Option<i32>,
    pub max_hp: Option<i32>,
}

impl From<StatBlock> for StatBlockStored {
    fn from(value: StatBlock) -> Self {
        Self {
            stats: value.stats,
            current_hp: value.current_hp,
            max_hp: value.max_hp,
        }
    }
}

impl From<StatBlockStored> for StatBlock {
    fn from(value: StatBlockStored) -> Self {
        Self {
            stats: value.stats,
            current_hp: value.current_hp,
            max_hp: value.max_hp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchetypeChangeStored {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub timestamp: String,
}

impl From<ArchetypeChange> for ArchetypeChangeStored {
    fn from(value: ArchetypeChange) -> Self {
        Self {
            from: format!("{:?}", value.from),
            to: format!("{:?}", value.to),
            reason: value.reason,
            timestamp: value.timestamp.to_rfc3339(),
        }
    }
}

impl From<ArchetypeChangeStored> for ArchetypeChange {
    fn from(value: ArchetypeChangeStored) -> Self {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&value.timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Self {
            from: parse_archetype(&value.from),
            to: parse_archetype(&value.to),
            reason: value.reason,
            timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ActantTargetStored {
    Character(String),
    Item(String),
    Goal(String),
}

impl TryFrom<ActantTarget> for ActantTargetStored {
    type Error = anyhow::Error;

    fn try_from(value: ActantTarget) -> Result<Self> {
        Ok(match value {
            ActantTarget::Character(id) => Self::Character(id.to_string()),
            ActantTarget::Item(id) => Self::Item(id.to_string()),
            ActantTarget::Goal(s) => Self::Goal(s),
        })
    }
}

impl TryFrom<ActantTargetStored> for ActantTarget {
    type Error = anyhow::Error;

    fn try_from(value: ActantTargetStored) -> Result<Self> {
        Ok(match value {
            ActantTargetStored::Character(id) => {
                ActantTarget::Character(CharacterId::from_uuid(uuid::Uuid::parse_str(&id)?))
            }
            ActantTargetStored::Item(id) => {
                ActantTarget::Item(ItemId::from_uuid(uuid::Uuid::parse_str(&id)?))
            }
            ActantTargetStored::Goal(s) => ActantTarget::Goal(s),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WantStored {
    pub id: String,
    pub description: String,
    pub target: Option<ActantTargetStored>,
    pub intensity: f32,
    pub known_to_player: bool,
}

impl TryFrom<Want> for WantStored {
    type Error = anyhow::Error;

    fn try_from(value: Want) -> Result<Self> {
        Ok(Self {
            id: value.id.to_string(),
            description: value.description,
            target: match value.target {
                Some(t) => Some(ActantTargetStored::try_from(t)?),
                None => None,
            },
            intensity: value.intensity,
            known_to_player: value.known_to_player,
        })
    }
}

impl TryFrom<WantStored> for Want {
    type Error = anyhow::Error;

    fn try_from(value: WantStored) -> Result<Self> {
        let id = uuid::Uuid::parse_str(&value.id).map(WantId::from_uuid)?;
        Ok(Self {
            id,
            description: value.description,
            target: match value.target {
                Some(t) => Some(ActantTarget::try_from(t)?),
                None => None,
            },
            intensity: value.intensity,
            known_to_player: value.known_to_player,
        })
    }
}

fn parse_archetype(s: &str) -> CampbellArchetype {
    match s {
        "Hero" => CampbellArchetype::Hero,
        "Mentor" => CampbellArchetype::Mentor,
        "ThresholdGuardian" => CampbellArchetype::ThresholdGuardian,
        "Herald" => CampbellArchetype::Herald,
        "Shapeshifter" => CampbellArchetype::Shapeshifter,
        "Shadow" => CampbellArchetype::Shadow,
        "Trickster" => CampbellArchetype::Trickster,
        "Ally" => CampbellArchetype::Ally,
        _ => CampbellArchetype::Ally,
    }
}

// =============================================================================
// CharacterRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl CharacterRepositoryPort for Neo4jCharacterRepository {
    async fn create(&self, character: &Character) -> Result<()> {
        Neo4jCharacterRepository::create(self, character).await
    }

    async fn get(&self, id: CharacterId) -> Result<Option<Character>> {
        Neo4jCharacterRepository::get(self, id).await
    }

    async fn list(&self, world_id: WorldId) -> Result<Vec<Character>> {
        Neo4jCharacterRepository::list_by_world(self, world_id).await
    }

    async fn update(&self, character: &Character) -> Result<()> {
        Neo4jCharacterRepository::update(self, character).await
    }

    async fn delete(&self, id: CharacterId) -> Result<()> {
        Neo4jCharacterRepository::delete(self, id).await
    }

    async fn get_by_scene(&self, _scene_id: SceneId) -> Result<Vec<Character>> {
        // TODO: Implement scene-based character lookup
        // For now, return empty vec - scenes have featured_characters field
        Ok(vec![])
    }
}
