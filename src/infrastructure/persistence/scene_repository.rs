//! Scene repository implementation for Neo4j
//!
//! # Graph-First Design (Phase 0.D)
//!
//! This repository uses Neo4j edges for all relationships:
//! - Location: `(Scene)-[:AT_LOCATION]->(Location)`
//! - Featured characters: `(Scene)-[:FEATURES_CHARACTER {role, entrance_cue}]->(Character)`

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::SceneRepositoryPort;
use crate::domain::entities::{Scene, SceneCharacter, SceneCharacterRole, SceneCondition, TimeContext};
use crate::domain::value_objects::{ActId, CharacterId, ItemId, LocationId, SceneId};

/// Repository for Scene operations
pub struct Neo4jSceneRepository {
    connection: Neo4jConnection,
}

impl Neo4jSceneRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    // =========================================================================
    // Core CRUD
    // =========================================================================

    /// Create a new scene
    pub async fn create(&self, scene: &Scene) -> Result<()> {
        let time_context_json =
            serde_json::to_string(&TimeContextStored::from(scene.time_context.clone()))?;
        let entry_conditions_json = serde_json::to_string(
            &scene
                .entry_conditions
                .iter()
                .cloned()
                .map(SceneConditionStored::try_from)
                .collect::<Result<Vec<_>>>()?,
        )?;
        let featured_characters_json = serde_json::to_string(
            &scene
                .featured_characters
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH (a:Act {id: $act_id})
            MATCH (l:Location {id: $location_id})
            CREATE (s:Scene {
                id: $id,
                act_id: $act_id,
                name: $name,
                location_id: $location_id,
                time_context: $time_context,
                backdrop_override: $backdrop_override,
                entry_conditions: $entry_conditions,
                featured_characters: $featured_characters,
                directorial_notes: $directorial_notes,
                order_num: $order_num
            })
            CREATE (a)-[:CONTAINS_SCENE]->(s)
            CREATE (s)-[:AT_LOCATION]->(l)
            RETURN s.id as id",
        )
        .param("id", scene.id.to_string())
        .param("act_id", scene.act_id.to_string())
        .param("name", scene.name.clone())
        .param("location_id", scene.location_id.to_string())
        .param("time_context", time_context_json)
        .param(
            "backdrop_override",
            scene.backdrop_override.clone().unwrap_or_default(),
        )
        .param("entry_conditions", entry_conditions_json)
        .param("featured_characters", featured_characters_json)
        .param("directorial_notes", scene.directorial_notes.clone())
        .param("order_num", scene.order as i64);

        self.connection.graph().run(q).await?;

        // Create FEATURES_CHARACTER edges for featured characters
        for char_id in &scene.featured_characters {
            let char_q = query(
                "MATCH (s:Scene {id: $scene_id})
                MATCH (c:Character {id: $char_id})
                CREATE (s)-[:FEATURES_CHARACTER {role: 'Secondary', entrance_cue: ''}]->(c)",
            )
            .param("scene_id", scene.id.to_string())
            .param("char_id", char_id.to_string());

            self.connection.graph().run(char_q).await?;
        }

        tracing::debug!("Created scene: {}", scene.name);
        Ok(())
    }

    /// Get a scene by ID
    pub async fn get(&self, id: SceneId) -> Result<Option<Scene>> {
        let q = query(
            "MATCH (s:Scene {id: $id})
            RETURN s",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_scene(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all scenes in an act
    pub async fn list_by_act(&self, act_id: ActId) -> Result<Vec<Scene>> {
        let q = query(
            "MATCH (a:Act {id: $act_id})-[:CONTAINS_SCENE]->(s:Scene)
            RETURN s
            ORDER BY s.order_num",
        )
        .param("act_id", act_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut scenes = Vec::new();

        while let Some(row) = result.next().await? {
            scenes.push(row_to_scene(row)?);
        }

        Ok(scenes)
    }

    /// List all scenes at a location (via AT_LOCATION edge)
    pub async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>> {
        let q = query(
            "MATCH (s:Scene)-[:AT_LOCATION]->(l:Location {id: $location_id})
            RETURN s
            ORDER BY s.order_num",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut scenes = Vec::new();

        while let Some(row) = result.next().await? {
            scenes.push(row_to_scene(row)?);
        }

        Ok(scenes)
    }

    /// Update a scene
    pub async fn update(&self, scene: &Scene) -> Result<()> {
        let time_context_json =
            serde_json::to_string(&TimeContextStored::from(scene.time_context.clone()))?;
        let entry_conditions_json = serde_json::to_string(
            &scene
                .entry_conditions
                .iter()
                .cloned()
                .map(SceneConditionStored::try_from)
                .collect::<Result<Vec<_>>>()?,
        )?;
        let featured_characters_json = serde_json::to_string(
            &scene
                .featured_characters
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH (s:Scene {id: $id})
            SET s.name = $name,
                s.location_id = $location_id,
                s.time_context = $time_context,
                s.backdrop_override = $backdrop_override,
                s.entry_conditions = $entry_conditions,
                s.featured_characters = $featured_characters,
                s.directorial_notes = $directorial_notes,
                s.order_num = $order_num
            RETURN s.id as id",
        )
        .param("id", scene.id.to_string())
        .param("name", scene.name.clone())
        .param("location_id", scene.location_id.to_string())
        .param("time_context", time_context_json)
        .param(
            "backdrop_override",
            scene.backdrop_override.clone().unwrap_or_default(),
        )
        .param("entry_conditions", entry_conditions_json)
        .param("featured_characters", featured_characters_json)
        .param("directorial_notes", scene.directorial_notes.clone())
        .param("order_num", scene.order as i64);

        self.connection.graph().run(q).await?;

        // Update AT_LOCATION edge
        self.set_location(scene.id, scene.location_id).await?;

        // Update FEATURES_CHARACTER edges
        // First remove existing
        let remove_q = query(
            "MATCH (s:Scene {id: $id})-[f:FEATURES_CHARACTER]->()
            DELETE f",
        )
        .param("id", scene.id.to_string());
        self.connection.graph().run(remove_q).await?;

        // Then add new ones
        for char_id in &scene.featured_characters {
            let char_q = query(
                "MATCH (s:Scene {id: $scene_id})
                MATCH (c:Character {id: $char_id})
                CREATE (s)-[:FEATURES_CHARACTER {role: 'Secondary', entrance_cue: ''}]->(c)",
            )
            .param("scene_id", scene.id.to_string())
            .param("char_id", char_id.to_string());

            self.connection.graph().run(char_q).await?;
        }

        tracing::debug!("Updated scene: {}", scene.name);
        Ok(())
    }

    /// Delete a scene
    pub async fn delete(&self, id: SceneId) -> Result<()> {
        let q = query(
            "MATCH (s:Scene {id: $id})
            DETACH DELETE s",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted scene: {}", id);
        Ok(())
    }

    /// Update directorial notes for a scene
    pub async fn update_directorial_notes(&self, id: SceneId, notes: &str) -> Result<()> {
        let q = query(
            "MATCH (s:Scene {id: $id})
            SET s.directorial_notes = $notes
            RETURN s.id as id",
        )
        .param("id", id.to_string())
        .param("notes", notes);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    // =========================================================================
    // Location (AT_LOCATION edge)
    // =========================================================================

    /// Set scene's location
    pub async fn set_location(&self, scene_id: SceneId, location_id: LocationId) -> Result<()> {
        // First remove any existing AT_LOCATION edge
        let remove_q = query(
            "MATCH (s:Scene {id: $scene_id})-[r:AT_LOCATION]->()
            DELETE r",
        )
        .param("scene_id", scene_id.to_string());
        self.connection.graph().run(remove_q).await?;

        // Create new AT_LOCATION edge
        let q = query(
            "MATCH (s:Scene {id: $scene_id}), (l:Location {id: $location_id})
            CREATE (s)-[:AT_LOCATION]->(l)",
        )
        .param("scene_id", scene_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get scene's location
    pub async fn get_location(&self, scene_id: SceneId) -> Result<Option<LocationId>> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[:AT_LOCATION]->(l:Location)
            RETURN l.id as location_id",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let location_id_str: String = row.get("location_id")?;
            let location_id = uuid::Uuid::parse_str(&location_id_str)?;
            Ok(Some(LocationId::from_uuid(location_id)))
        } else {
            Ok(None)
        }
    }

    // =========================================================================
    // Featured Characters (FEATURES_CHARACTER edges)
    // =========================================================================

    /// Add a featured character to the scene
    pub async fn add_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id}), (c:Character {id: $character_id})
            CREATE (s)-[:FEATURES_CHARACTER {
                role: $role,
                entrance_cue: $entrance_cue
            }]->(c)",
        )
        .param("scene_id", scene_id.to_string())
        .param("character_id", character_id.to_string())
        .param("role", scene_char.role.to_string())
        .param("entrance_cue", scene_char.entrance_cue.clone().unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get all featured characters for a scene
    pub async fn get_featured_characters(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<(CharacterId, SceneCharacter)>> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[r:FEATURES_CHARACTER]->(c:Character)
            RETURN c.id as character_id, r.role as role, r.entrance_cue as entrance_cue",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            let char_id_str: String = row.get("character_id")?;
            let role_str: String = row.get("role")?;
            let entrance_cue: String = row.get("entrance_cue").unwrap_or_default();

            let char_id = CharacterId::from_uuid(uuid::Uuid::parse_str(&char_id_str)?);
            let role = role_str.parse().unwrap_or(SceneCharacterRole::Secondary);

            let scene_char = SceneCharacter {
                role,
                entrance_cue: if entrance_cue.is_empty() {
                    None
                } else {
                    Some(entrance_cue)
                },
            };

            characters.push((char_id, scene_char));
        }

        Ok(characters)
    }

    /// Update a featured character's role/cue
    pub async fn update_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[r:FEATURES_CHARACTER]->(c:Character {id: $character_id})
            SET r.role = $role, r.entrance_cue = $entrance_cue",
        )
        .param("scene_id", scene_id.to_string())
        .param("character_id", character_id.to_string())
        .param("role", scene_char.role.to_string())
        .param("entrance_cue", scene_char.entrance_cue.clone().unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove a featured character from the scene
    pub async fn remove_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<()> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[r:FEATURES_CHARACTER]->(c:Character {id: $character_id})
            DELETE r",
        )
        .param("scene_id", scene_id.to_string())
        .param("character_id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get scenes featuring a specific character
    pub async fn get_scenes_for_character(&self, character_id: CharacterId) -> Result<Vec<Scene>> {
        let q = query(
            "MATCH (s:Scene)-[:FEATURES_CHARACTER]->(c:Character {id: $character_id})
            RETURN s
            ORDER BY s.order_num",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut scenes = Vec::new();

        while let Some(row) = result.next().await? {
            scenes.push(row_to_scene(row)?);
        }

        Ok(scenes)
    }
}

// =============================================================================
// Row Conversion Helpers
// =============================================================================

fn row_to_scene(row: Row) -> Result<Scene> {
    let node: neo4rs::Node = row.get("s")?;

    let id_str: String = node.get("id")?;
    let act_id_str: String = node.get("act_id")?;
    let name: String = node.get("name")?;
    // location_id may not exist in newer schemas - default to a placeholder
    let location_id_str: String = node.get("location_id").unwrap_or_default();
    let time_context_json: String = node.get("time_context")?;
    let backdrop_override: String = node.get("backdrop_override")?;
    let entry_conditions_json: String = node.get("entry_conditions")?;
    // featured_characters may not exist in newer schemas
    let featured_characters_json: String = node.get("featured_characters").unwrap_or_else(|_| "[]".to_string());
    let directorial_notes: String = node.get("directorial_notes")?;
    let order_num: i64 = node.get("order_num")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let act_id = uuid::Uuid::parse_str(&act_id_str)?;
    let location_id = if location_id_str.is_empty() {
        LocationId::new() // Placeholder - should be fetched via AT_LOCATION edge
    } else {
        LocationId::from_uuid(uuid::Uuid::parse_str(&location_id_str)?)
    };
    let time_context: TimeContext =
        serde_json::from_str::<TimeContextStored>(&time_context_json)?.into();
    let entry_conditions: Vec<SceneCondition> =
        serde_json::from_str::<Vec<SceneConditionStored>>(&entry_conditions_json)?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;
    let featured_characters: Vec<CharacterId> =
        serde_json::from_str::<Vec<String>>(&featured_characters_json)?
            .into_iter()
            .filter_map(|s| uuid::Uuid::parse_str(&s).ok().map(CharacterId::from_uuid))
            .collect();

    Ok(Scene {
        id: SceneId::from_uuid(id),
        act_id: ActId::from_uuid(act_id),
        name,
        location_id,
        time_context,
        backdrop_override: if backdrop_override.is_empty() {
            None
        } else {
            Some(backdrop_override)
        },
        entry_conditions,
        featured_characters,
        directorial_notes,
        order: order_num as u32,
    })
}

// =============================================================================
// Persistence serde models
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TimeContextStored {
    Unspecified,
    TimeOfDay(TimeOfDayStored),
    During(String),
    Custom(String),
}

impl From<TimeContext> for TimeContextStored {
    fn from(value: TimeContext) -> Self {
        match value {
            TimeContext::Unspecified => Self::Unspecified,
            TimeContext::TimeOfDay(t) => Self::TimeOfDay(t.into()),
            TimeContext::During(s) => Self::During(s),
            TimeContext::Custom(s) => Self::Custom(s),
        }
    }
}

impl From<TimeContextStored> for TimeContext {
    fn from(value: TimeContextStored) -> Self {
        match value {
            TimeContextStored::Unspecified => Self::Unspecified,
            TimeContextStored::TimeOfDay(t) => Self::TimeOfDay(t.into()),
            TimeContextStored::During(s) => Self::During(s),
            TimeContextStored::Custom(s) => Self::Custom(s),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum TimeOfDayStored {
    Dawn,
    Morning,
    Midday,
    Afternoon,
    Evening,
    Dusk,
    Night,
    Midnight,
}

impl From<crate::domain::entities::TimeOfDay> for TimeOfDayStored {
    fn from(value: crate::domain::entities::TimeOfDay) -> Self {
        use crate::domain::entities::TimeOfDay as T;
        match value {
            T::Dawn => Self::Dawn,
            T::Morning => Self::Morning,
            T::Midday => Self::Midday,
            T::Afternoon => Self::Afternoon,
            T::Evening => Self::Evening,
            T::Dusk => Self::Dusk,
            T::Night => Self::Night,
            T::Midnight => Self::Midnight,
        }
    }
}

impl From<TimeOfDayStored> for crate::domain::entities::TimeOfDay {
    fn from(value: TimeOfDayStored) -> Self {
        use crate::domain::entities::TimeOfDay as T;
        match value {
            TimeOfDayStored::Dawn => T::Dawn,
            TimeOfDayStored::Morning => T::Morning,
            TimeOfDayStored::Midday => T::Midday,
            TimeOfDayStored::Afternoon => T::Afternoon,
            TimeOfDayStored::Evening => T::Evening,
            TimeOfDayStored::Dusk => T::Dusk,
            TimeOfDayStored::Night => T::Night,
            TimeOfDayStored::Midnight => T::Midnight,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SceneConditionStored {
    CompletedScene(String),
    HasItem(String),
    KnowsCharacter(String),
    FlagSet(String),
    Custom(String),
}

impl TryFrom<SceneCondition> for SceneConditionStored {
    type Error = anyhow::Error;

    fn try_from(value: SceneCondition) -> Result<Self> {
        Ok(match value {
            SceneCondition::CompletedScene(id) => Self::CompletedScene(id.to_string()),
            SceneCondition::HasItem(id) => Self::HasItem(id.to_string()),
            SceneCondition::KnowsCharacter(id) => Self::KnowsCharacter(id.to_string()),
            SceneCondition::FlagSet(s) => Self::FlagSet(s),
            SceneCondition::Custom(s) => Self::Custom(s),
        })
    }
}

impl TryFrom<SceneConditionStored> for SceneCondition {
    type Error = anyhow::Error;

    fn try_from(value: SceneConditionStored) -> Result<Self> {
        Ok(match value {
            SceneConditionStored::CompletedScene(id) => SceneCondition::CompletedScene(
                SceneId::from_uuid(uuid::Uuid::parse_str(&id)?),
            ),
            SceneConditionStored::HasItem(id) => {
                SceneCondition::HasItem(ItemId::from_uuid(uuid::Uuid::parse_str(&id)?))
            }
            SceneConditionStored::KnowsCharacter(id) => SceneCondition::KnowsCharacter(
                CharacterId::from_uuid(uuid::Uuid::parse_str(&id)?),
            ),
            SceneConditionStored::FlagSet(s) => SceneCondition::FlagSet(s),
            SceneConditionStored::Custom(s) => SceneCondition::Custom(s),
        })
    }
}

// =============================================================================
// SceneRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl SceneRepositoryPort for Neo4jSceneRepository {
    async fn create(&self, scene: &Scene) -> Result<()> {
        Neo4jSceneRepository::create(self, scene).await
    }

    async fn get(&self, id: SceneId) -> Result<Option<Scene>> {
        Neo4jSceneRepository::get(self, id).await
    }

    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<Scene>> {
        Neo4jSceneRepository::list_by_act(self, act_id).await
    }

    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>> {
        Neo4jSceneRepository::list_by_location(self, location_id).await
    }

    async fn update(&self, scene: &Scene) -> Result<()> {
        Neo4jSceneRepository::update(self, scene).await
    }

    async fn delete(&self, id: SceneId) -> Result<()> {
        Neo4jSceneRepository::delete(self, id).await
    }

    async fn update_directorial_notes(&self, id: SceneId, notes: &str) -> Result<()> {
        Neo4jSceneRepository::update_directorial_notes(self, id, notes).await
    }

    async fn set_location(&self, scene_id: SceneId, location_id: LocationId) -> Result<()> {
        Neo4jSceneRepository::set_location(self, scene_id, location_id).await
    }

    async fn get_location(&self, scene_id: SceneId) -> Result<Option<LocationId>> {
        Neo4jSceneRepository::get_location(self, scene_id).await
    }

    async fn add_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()> {
        Neo4jSceneRepository::add_featured_character(self, scene_id, character_id, scene_char).await
    }

    async fn get_featured_characters(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<(CharacterId, SceneCharacter)>> {
        Neo4jSceneRepository::get_featured_characters(self, scene_id).await
    }

    async fn update_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()> {
        Neo4jSceneRepository::update_featured_character(self, scene_id, character_id, scene_char)
            .await
    }

    async fn remove_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<()> {
        Neo4jSceneRepository::remove_featured_character(self, scene_id, character_id).await
    }

    async fn get_scenes_for_character(&self, character_id: CharacterId) -> Result<Vec<Scene>> {
        Neo4jSceneRepository::get_scenes_for_character(self, character_id).await
    }
}
