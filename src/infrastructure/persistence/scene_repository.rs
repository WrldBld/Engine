//! Scene repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::SceneRepositoryPort;
use crate::domain::entities::{Scene, SceneCondition, TimeContext};
use crate::domain::value_objects::{ActId, CharacterId, LocationId, SceneId};

/// Repository for Scene operations
pub struct Neo4jSceneRepository {
    connection: Neo4jConnection,
}

impl Neo4jSceneRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new scene
    pub async fn create(&self, scene: &Scene) -> Result<()> {
        let time_context_json = serde_json::to_string(&scene.time_context)?;
        let entry_conditions_json = serde_json::to_string(&scene.entry_conditions)?;
        let featured_characters_json = serde_json::to_string(&scene.featured_characters)?;

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
            CREATE (s)-[:TAKES_PLACE_AT]->(l)
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

        // Create relationships to featured characters
        for char_id in &scene.featured_characters {
            let char_q = query(
                "MATCH (s:Scene {id: $scene_id})
                MATCH (c:Character {id: $char_id})
                CREATE (s)-[:FEATURES]->(c)",
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

    /// List all scenes at a location
    pub async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>> {
        let q = query(
            "MATCH (s:Scene)-[:TAKES_PLACE_AT]->(l:Location {id: $location_id})
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
        let time_context_json = serde_json::to_string(&scene.time_context)?;
        let entry_conditions_json = serde_json::to_string(&scene.entry_conditions)?;
        let featured_characters_json = serde_json::to_string(&scene.featured_characters)?;

        let q = query(
            "MATCH (s:Scene {id: $id})
            SET s.name = $name,
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

        // Update featured character relationships
        // First remove existing
        let remove_q = query(
            "MATCH (s:Scene {id: $id})-[f:FEATURES]->()
            DELETE f",
        )
        .param("id", scene.id.to_string());
        self.connection.graph().run(remove_q).await?;

        // Then add new ones
        for char_id in &scene.featured_characters {
            let char_q = query(
                "MATCH (s:Scene {id: $scene_id})
                MATCH (c:Character {id: $char_id})
                CREATE (s)-[:FEATURES]->(c)",
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
}

fn row_to_scene(row: Row) -> Result<Scene> {
    let node: neo4rs::Node = row.get("s")?;

    let id_str: String = node.get("id")?;
    let act_id_str: String = node.get("act_id")?;
    let name: String = node.get("name")?;
    let location_id_str: String = node.get("location_id")?;
    let time_context_json: String = node.get("time_context")?;
    let backdrop_override: String = node.get("backdrop_override")?;
    let entry_conditions_json: String = node.get("entry_conditions")?;
    let featured_characters_json: String = node.get("featured_characters")?;
    let directorial_notes: String = node.get("directorial_notes")?;
    let order_num: i64 = node.get("order_num")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let act_id = uuid::Uuid::parse_str(&act_id_str)?;
    let location_id = uuid::Uuid::parse_str(&location_id_str)?;
    let time_context: TimeContext = serde_json::from_str(&time_context_json)?;
    let entry_conditions: Vec<SceneCondition> = serde_json::from_str(&entry_conditions_json)?;
    let featured_characters: Vec<CharacterId> = serde_json::from_str(&featured_characters_json)?;

    Ok(Scene {
        id: SceneId::from_uuid(id),
        act_id: ActId::from_uuid(act_id),
        name,
        location_id: LocationId::from_uuid(location_id),
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
}
