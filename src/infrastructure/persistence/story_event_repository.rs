//! StoryEvent repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};
use uuid::Uuid;

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::StoryEventRepositoryPort;
use crate::domain::entities::{StoryEvent, StoryEventType};
use crate::domain::value_objects::{
    CharacterId, LocationId, NarrativeEventId, SceneId, SessionId, StoryEventId, WorldId,
};

/// Repository for StoryEvent operations
pub struct Neo4jStoryEventRepository {
    connection: Neo4jConnection,
}

impl Neo4jStoryEventRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl StoryEventRepositoryPort for Neo4jStoryEventRepository {
    /// Create a new story event
    async fn create(&self, event: &StoryEvent) -> Result<()> {
        let event_type_json = serde_json::to_string(&event.event_type)?;
        let involved_chars: Vec<String> = event
            .involved_characters
            .iter()
            .map(|id| id.to_string())
            .collect();
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (e:StoryEvent {
                id: $id,
                world_id: $world_id,
                session_id: $session_id,
                scene_id: $scene_id,
                location_id: $location_id,
                event_type_json: $event_type_json,
                timestamp: $timestamp,
                game_time: $game_time,
                summary: $summary,
                involved_characters: $involved_characters,
                is_hidden: $is_hidden,
                tags_json: $tags_json,
                triggered_by: $triggered_by
            })
            CREATE (w)-[:HAS_STORY_EVENT]->(e)
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("world_id", event.world_id.to_string())
        .param("session_id", event.session_id.to_string())
        .param(
            "scene_id",
            event.scene_id.map(|s| s.to_string()).unwrap_or_default(),
        )
        .param(
            "location_id",
            event.location_id.map(|l| l.to_string()).unwrap_or_default(),
        )
        .param("event_type_json", event_type_json)
        .param("timestamp", event.timestamp.to_rfc3339())
        .param(
            "game_time",
            event.game_time.clone().unwrap_or_default(),
        )
        .param("summary", event.summary.clone())
        .param("involved_characters", involved_chars)
        .param("is_hidden", event.is_hidden)
        .param("tags_json", tags_json)
        .param(
            "triggered_by",
            event.triggered_by.map(|t| t.to_string()).unwrap_or_default(),
        );

        self.connection.graph().run(q).await?;
        tracing::debug!("Created story event: {}", event.id);

        // Create relationships to scene and location if specified
        if let Some(scene_id) = event.scene_id {
            let scene_q = query(
                "MATCH (e:StoryEvent {id: $event_id}), (s:Scene {id: $scene_id})
                MERGE (s)-[:HAS_EVENT]->(e)",
            )
            .param("event_id", event.id.to_string())
            .param("scene_id", scene_id.to_string());

            let _ = self.connection.graph().run(scene_q).await;
        }

        if let Some(location_id) = event.location_id {
            let loc_q = query(
                "MATCH (e:StoryEvent {id: $event_id}), (l:Location {id: $location_id})
                MERGE (l)-[:HAS_EVENT]->(e)",
            )
            .param("event_id", event.id.to_string())
            .param("location_id", location_id.to_string());

            let _ = self.connection.graph().run(loc_q).await;
        }

        // Create relationships to involved characters
        for char_id in &event.involved_characters {
            let char_q = query(
                "MATCH (e:StoryEvent {id: $event_id}), (c:Character {id: $char_id})
                MERGE (c)-[:INVOLVED_IN]->(e)",
            )
            .param("event_id", event.id.to_string())
            .param("char_id", char_id.to_string());

            let _ = self.connection.graph().run(char_q).await;
        }

        Ok(())
    }

    /// Get a story event by ID
    async fn get(&self, id: StoryEventId) -> Result<Option<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            RETURN e",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_story_event(row)?))
        } else {
            Ok(None)
        }
    }

    /// List story events for a session
    async fn list_by_session(&self, session_id: SessionId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent {session_id: $session_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("session_id", session_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List story events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List story events for a world with pagination
    async fn list_by_world_paginated(
        &self,
        world_id: WorldId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN e
            ORDER BY e.timestamp DESC
            SKIP $offset
            LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("offset", offset as i64)
        .param("limit", limit as i64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List visible (non-hidden) story events for a world
    async fn list_visible(&self, world_id: WorldId, limit: u32) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE e.is_hidden = false
            RETURN e
            ORDER BY e.timestamp DESC
            LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("limit", limit as i64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// Search story events by tags
    async fn search_by_tags(
        &self,
        world_id: WorldId,
        tags: Vec<String>,
    ) -> Result<Vec<StoryEvent>> {
        // Note: We store tags as JSON, so we search in the JSON string
        // A more efficient approach would be to store tags as separate nodes
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE ANY(tag IN $tags WHERE e.tags_json CONTAINS tag)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string())
        .param("tags", tags);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// Search story events by text in summary
    async fn search_by_text(
        &self,
        world_id: WorldId,
        search_text: &str,
    ) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE toLower(e.summary) CONTAINS toLower($search_text)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string())
        .param("search_text", search_text);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events involving a specific character
    async fn list_by_character(&self, character_id: CharacterId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (c:Character {id: $char_id})-[:INVOLVED_IN]->(e:StoryEvent)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("char_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events at a specific location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (l:Location {id: $location_id})-[:HAS_EVENT]->(e:StoryEvent)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// Update story event summary (DM editing)
    async fn update_summary(&self, id: StoryEventId, summary: &str) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.summary = $summary
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("summary", summary);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Update event visibility
    async fn set_hidden(&self, id: StoryEventId, is_hidden: bool) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.is_hidden = $is_hidden
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("is_hidden", is_hidden);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Update event tags
    async fn update_tags(&self, id: StoryEventId, tags: Vec<String>) -> Result<bool> {
        let tags_json = serde_json::to_string(&tags)?;
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.tags_json = $tags_json
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("tags_json", tags_json);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Delete a story event (rarely used - events are usually immutable)
    async fn delete(&self, id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
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

    /// Count events for a world
    async fn count_by_world(&self, world_id: WorldId) -> Result<u64> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN count(e) as count",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            Ok(count as u64)
        } else {
            Ok(0)
        }
    }
}

/// Convert a Neo4j row to a StoryEvent
fn row_to_story_event(row: Row) -> Result<StoryEvent> {
    let node: neo4rs::Node = row.get("e")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let session_id_str: String = node.get("session_id")?;
    let scene_id_str: String = node.get("scene_id").unwrap_or_default();
    let location_id_str: String = node.get("location_id").unwrap_or_default();
    let event_type_json: String = node.get("event_type_json")?;
    let timestamp_str: String = node.get("timestamp")?;
    let game_time: String = node.get("game_time").unwrap_or_default();
    let summary: String = node.get("summary")?;
    let involved_chars: Vec<String> = node.get("involved_characters").unwrap_or_default();
    let is_hidden: bool = node.get("is_hidden").unwrap_or(false);
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());
    let triggered_by_str: String = node.get("triggered_by").unwrap_or_default();

    let event_type: StoryEventType = serde_json::from_str(&event_type_json)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json)?;

    let involved_characters: Vec<CharacterId> = involved_chars
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok().map(CharacterId::from))
        .collect();

    Ok(StoryEvent {
        id: StoryEventId::from(Uuid::parse_str(&id_str)?),
        world_id: WorldId::from(Uuid::parse_str(&world_id_str)?),
        session_id: SessionId::from(Uuid::parse_str(&session_id_str)?),
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
        event_type,
        timestamp: DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Utc),
        game_time: if game_time.is_empty() {
            None
        } else {
            Some(game_time)
        },
        summary,
        involved_characters,
        is_hidden,
        tags,
        triggered_by: if triggered_by_str.is_empty() {
            None
        } else {
            Uuid::parse_str(&triggered_by_str)
                .ok()
                .map(NarrativeEventId::from)
        },
    })
}
