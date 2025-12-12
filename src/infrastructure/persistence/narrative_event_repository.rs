//! NarrativeEvent repository implementation for Neo4j

use anyhow::Result;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};
use uuid::Uuid;

use super::connection::Neo4jConnection;
use crate::domain::entities::{
    EventOutcome, NarrativeEvent, NarrativeTrigger, TriggerLogic,
};
use crate::domain::value_objects::{
    ActId, CharacterId, EventChainId, LocationId, NarrativeEventId, SceneId, WorldId,
};

/// Repository for NarrativeEvent operations
pub struct Neo4jNarrativeEventRepository {
    connection: Neo4jConnection,
}

impl Neo4jNarrativeEventRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new narrative event
    pub async fn create(&self, event: &NarrativeEvent) -> Result<()> {
        let triggers_json = serde_json::to_string(&event.trigger_conditions)?;
        let outcomes_json = serde_json::to_string(&event.outcomes)?;
        let featured_npcs: Vec<String> = event
            .featured_npcs
            .iter()
            .map(|id| id.to_string())
            .collect();
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (e:NarrativeEvent {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                tags_json: $tags_json,
                triggers_json: $triggers_json,
                trigger_logic: $trigger_logic,
                scene_direction: $scene_direction,
                suggested_opening: $suggested_opening,
                featured_npcs: $featured_npcs,
                outcomes_json: $outcomes_json,
                default_outcome: $default_outcome,
                is_active: $is_active,
                is_triggered: $is_triggered,
                triggered_at: $triggered_at,
                selected_outcome: $selected_outcome,
                is_repeatable: $is_repeatable,
                trigger_count: $trigger_count,
                delay_turns: $delay_turns,
                expires_after_turns: $expires_after_turns,
                scene_id: $scene_id,
                location_id: $location_id,
                act_id: $act_id,
                priority: $priority,
                is_favorite: $is_favorite,
                chain_id: $chain_id,
                chain_position: $chain_position,
                created_at: $created_at,
                updated_at: $updated_at
            })
            CREATE (w)-[:HAS_NARRATIVE_EVENT]->(e)
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("world_id", event.world_id.to_string())
        .param("name", event.name.clone())
        .param("description", event.description.clone())
        .param("tags_json", tags_json)
        .param("triggers_json", triggers_json)
        .param("trigger_logic", format!("{:?}", event.trigger_logic))
        .param("scene_direction", event.scene_direction.clone())
        .param(
            "suggested_opening",
            event.suggested_opening.clone().unwrap_or_default(),
        )
        .param("featured_npcs", featured_npcs)
        .param("outcomes_json", outcomes_json)
        .param(
            "default_outcome",
            event.default_outcome.clone().unwrap_or_default(),
        )
        .param("is_active", event.is_active)
        .param("is_triggered", event.is_triggered)
        .param(
            "triggered_at",
            event
                .triggered_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        )
        .param(
            "selected_outcome",
            event.selected_outcome.clone().unwrap_or_default(),
        )
        .param("is_repeatable", event.is_repeatable)
        .param("trigger_count", event.trigger_count as i64)
        .param("delay_turns", event.delay_turns as i64)
        .param(
            "expires_after_turns",
            event.expires_after_turns.map(|t| t as i64).unwrap_or(-1),
        )
        .param(
            "scene_id",
            event.scene_id.map(|s| s.to_string()).unwrap_or_default(),
        )
        .param(
            "location_id",
            event.location_id.map(|l| l.to_string()).unwrap_or_default(),
        )
        .param(
            "act_id",
            event.act_id.map(|a| a.to_string()).unwrap_or_default(),
        )
        .param("priority", event.priority as i64)
        .param("is_favorite", event.is_favorite)
        .param(
            "chain_id",
            event.chain_id.map(|c| c.to_string()).unwrap_or_default(),
        )
        .param(
            "chain_position",
            event.chain_position.map(|p| p as i64).unwrap_or(-1),
        )
        .param("created_at", event.created_at.to_rfc3339())
        .param("updated_at", event.updated_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Created narrative event: {}", event.name);

        Ok(())
    }

    /// Get a narrative event by ID
    pub async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            RETURN e",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_narrative_event(row)?))
        } else {
            Ok(None)
        }
    }

    /// Update a narrative event
    pub async fn update(&self, event: &NarrativeEvent) -> Result<bool> {
        let triggers_json = serde_json::to_string(&event.trigger_conditions)?;
        let outcomes_json = serde_json::to_string(&event.outcomes)?;
        let featured_npcs: Vec<String> = event
            .featured_npcs
            .iter()
            .map(|id| id.to_string())
            .collect();
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.name = $name,
                e.description = $description,
                e.tags_json = $tags_json,
                e.triggers_json = $triggers_json,
                e.trigger_logic = $trigger_logic,
                e.scene_direction = $scene_direction,
                e.suggested_opening = $suggested_opening,
                e.featured_npcs = $featured_npcs,
                e.outcomes_json = $outcomes_json,
                e.default_outcome = $default_outcome,
                e.is_active = $is_active,
                e.is_triggered = $is_triggered,
                e.triggered_at = $triggered_at,
                e.selected_outcome = $selected_outcome,
                e.is_repeatable = $is_repeatable,
                e.trigger_count = $trigger_count,
                e.delay_turns = $delay_turns,
                e.expires_after_turns = $expires_after_turns,
                e.scene_id = $scene_id,
                e.location_id = $location_id,
                e.act_id = $act_id,
                e.priority = $priority,
                e.is_favorite = $is_favorite,
                e.chain_id = $chain_id,
                e.chain_position = $chain_position,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("name", event.name.clone())
        .param("description", event.description.clone())
        .param("tags_json", tags_json)
        .param("triggers_json", triggers_json)
        .param("trigger_logic", format!("{:?}", event.trigger_logic))
        .param("scene_direction", event.scene_direction.clone())
        .param(
            "suggested_opening",
            event.suggested_opening.clone().unwrap_or_default(),
        )
        .param("featured_npcs", featured_npcs)
        .param("outcomes_json", outcomes_json)
        .param(
            "default_outcome",
            event.default_outcome.clone().unwrap_or_default(),
        )
        .param("is_active", event.is_active)
        .param("is_triggered", event.is_triggered)
        .param(
            "triggered_at",
            event
                .triggered_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        )
        .param(
            "selected_outcome",
            event.selected_outcome.clone().unwrap_or_default(),
        )
        .param("is_repeatable", event.is_repeatable)
        .param("trigger_count", event.trigger_count as i64)
        .param("delay_turns", event.delay_turns as i64)
        .param(
            "expires_after_turns",
            event.expires_after_turns.map(|t| t as i64).unwrap_or(-1),
        )
        .param(
            "scene_id",
            event.scene_id.map(|s| s.to_string()).unwrap_or_default(),
        )
        .param(
            "location_id",
            event.location_id.map(|l| l.to_string()).unwrap_or_default(),
        )
        .param(
            "act_id",
            event.act_id.map(|a| a.to_string()).unwrap_or_default(),
        )
        .param("priority", event.priority as i64)
        .param("is_favorite", event.is_favorite)
        .param(
            "chain_id",
            event.chain_id.map(|c| c.to_string()).unwrap_or_default(),
        )
        .param(
            "chain_position",
            event.chain_position.map(|p| p as i64).unwrap_or(-1),
        )
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// List all narrative events for a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            RETURN e
            ORDER BY e.is_favorite DESC, e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List active narrative events for a world
    pub async fn list_active(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_active = true
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List favorite narrative events for a world
    pub async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_favorite = true
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List untriggered active events (for LLM context)
    pub async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_active = true AND e.is_triggered = false
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// Toggle favorite status
    pub async fn toggle_favorite(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_favorite = NOT e.is_favorite,
                e.updated_at = $updated_at
            RETURN e.is_favorite as is_favorite",
        )
        .param("id", id.to_string())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let is_favorite: bool = row.get("is_favorite")?;
            Ok(is_favorite)
        } else {
            Ok(false)
        }
    }

    /// Set active status
    pub async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_active = $is_active,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("is_active", is_active)
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Mark event as triggered
    pub async fn mark_triggered(
        &self,
        id: NarrativeEventId,
        outcome_name: Option<String>,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_triggered = true,
                e.triggered_at = $triggered_at,
                e.selected_outcome = $selected_outcome,
                e.trigger_count = e.trigger_count + 1,
                e.is_active = CASE WHEN e.is_repeatable THEN e.is_active ELSE false END,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("triggered_at", Utc::now().to_rfc3339())
        .param("selected_outcome", outcome_name.unwrap_or_default())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Reset triggered status (for repeatable events)
    pub async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_triggered = false,
                e.triggered_at = null,
                e.selected_outcome = null,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Delete a narrative event
    pub async fn delete(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            DETACH DELETE e
            RETURN count(*) as deleted",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }
}

/// Convert a Neo4j row to a NarrativeEvent
fn row_to_narrative_event(row: Row) -> Result<NarrativeEvent> {
    let node: neo4rs::Node = row.get("e")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());
    let triggers_json: String = node.get("triggers_json").unwrap_or_else(|_| "[]".to_string());
    let trigger_logic_str: String = node.get("trigger_logic").unwrap_or_else(|_| "All".to_string());
    let scene_direction: String = node.get("scene_direction").unwrap_or_default();
    let suggested_opening: String = node.get("suggested_opening").unwrap_or_default();
    let featured_npcs: Vec<String> = node.get("featured_npcs").unwrap_or_default();
    let outcomes_json: String = node.get("outcomes_json").unwrap_or_else(|_| "[]".to_string());
    let default_outcome: String = node.get("default_outcome").unwrap_or_default();
    let is_active: bool = node.get("is_active").unwrap_or(true);
    let is_triggered: bool = node.get("is_triggered").unwrap_or(false);
    let triggered_at_str: String = node.get("triggered_at").unwrap_or_default();
    let selected_outcome: String = node.get("selected_outcome").unwrap_or_default();
    let is_repeatable: bool = node.get("is_repeatable").unwrap_or(false);
    let trigger_count: i64 = node.get("trigger_count").unwrap_or(0);
    let delay_turns: i64 = node.get("delay_turns").unwrap_or(0);
    let expires_after_turns: i64 = node.get("expires_after_turns").unwrap_or(-1);
    let scene_id_str: String = node.get("scene_id").unwrap_or_default();
    let location_id_str: String = node.get("location_id").unwrap_or_default();
    let act_id_str: String = node.get("act_id").unwrap_or_default();
    let priority: i64 = node.get("priority").unwrap_or(0);
    let is_favorite: bool = node.get("is_favorite").unwrap_or(false);
    let chain_id_str: String = node.get("chain_id").unwrap_or_default();
    let chain_position: i64 = node.get("chain_position").unwrap_or(-1);
    let created_at_str: String = node.get("created_at")?;
    let updated_at_str: String = node.get("updated_at")?;

    let tags: Vec<String> = serde_json::from_str(&tags_json)?;
    let trigger_conditions: Vec<NarrativeTrigger> = serde_json::from_str(&triggers_json)?;
    let outcomes: Vec<EventOutcome> = serde_json::from_str(&outcomes_json)?;

    let trigger_logic = match trigger_logic_str.as_str() {
        "Any" => TriggerLogic::Any,
        s if s.starts_with("AtLeast(") => {
            let n: u32 = s
                .trim_start_matches("AtLeast(")
                .trim_end_matches(')')
                .parse()
                .unwrap_or(1);
            TriggerLogic::AtLeast(n)
        }
        _ => TriggerLogic::All,
    };

    let featured_npcs_ids: Vec<CharacterId> = featured_npcs
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok().map(CharacterId::from))
        .collect();

    Ok(NarrativeEvent {
        id: NarrativeEventId::from(Uuid::parse_str(&id_str)?),
        world_id: WorldId::from(Uuid::parse_str(&world_id_str)?),
        name,
        description,
        tags,
        trigger_conditions,
        trigger_logic,
        scene_direction,
        suggested_opening: if suggested_opening.is_empty() {
            None
        } else {
            Some(suggested_opening)
        },
        featured_npcs: featured_npcs_ids,
        outcomes,
        default_outcome: if default_outcome.is_empty() {
            None
        } else {
            Some(default_outcome)
        },
        is_active,
        is_triggered,
        triggered_at: if triggered_at_str.is_empty() {
            None
        } else {
            DateTime::parse_from_rfc3339(&triggered_at_str)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        },
        selected_outcome: if selected_outcome.is_empty() {
            None
        } else {
            Some(selected_outcome)
        },
        is_repeatable,
        trigger_count: trigger_count as u32,
        delay_turns: delay_turns as u32,
        expires_after_turns: if expires_after_turns < 0 {
            None
        } else {
            Some(expires_after_turns as u32)
        },
        scene_id: if scene_id_str.is_empty() {
            None
        } else {
            Uuid::parse_str(&scene_id_str).ok().map(SceneId::from)
        },
        location_id: if location_id_str.is_empty() {
            None
        } else {
            Uuid::parse_str(&location_id_str).ok().map(LocationId::from)
        },
        act_id: if act_id_str.is_empty() {
            None
        } else {
            Uuid::parse_str(&act_id_str).ok().map(ActId::from)
        },
        priority: priority as i32,
        is_favorite,
        chain_id: if chain_id_str.is_empty() {
            None
        } else {
            Uuid::parse_str(&chain_id_str).ok().map(EventChainId::from)
        },
        chain_position: if chain_position < 0 {
            None
        } else {
            Some(chain_position as u32)
        },
        created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc),
    })
}
