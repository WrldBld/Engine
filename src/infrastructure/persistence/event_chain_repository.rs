//! EventChain repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};
use uuid::Uuid;

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::EventChainRepositoryPort;
use crate::domain::entities::{ChainStatus, EventChain};
use crate::domain::value_objects::{ActId, EventChainId, NarrativeEventId, WorldId};

/// Repository for EventChain operations
pub struct Neo4jEventChainRepository {
    connection: Neo4jConnection,
}

impl Neo4jEventChainRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new event chain
    pub async fn create(&self, chain: &EventChain) -> Result<()> {
        let events_json: Vec<String> = chain.events.iter().map(|id| id.to_string()).collect();
        let completed_json: Vec<String> = chain
            .completed_events
            .iter()
            .map(|id| id.to_string())
            .collect();
        let tags_json = serde_json::to_string(&chain.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (c:EventChain {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                events: $events,
                is_active: $is_active,
                current_position: $current_position,
                completed_events: $completed_events,
                act_id: $act_id,
                tags_json: $tags_json,
                color: $color,
                is_favorite: $is_favorite,
                created_at: $created_at,
                updated_at: $updated_at
            })
            CREATE (w)-[:HAS_EVENT_CHAIN]->(c)
            RETURN c.id as id",
        )
        .param("id", chain.id.to_string())
        .param("world_id", chain.world_id.to_string())
        .param("name", chain.name.clone())
        .param("description", chain.description.clone())
        .param("events", events_json)
        .param("is_active", chain.is_active)
        .param("current_position", chain.current_position as i64)
        .param("completed_events", completed_json)
        .param(
            "act_id",
            chain.act_id.map(|a| a.to_string()).unwrap_or_default(),
        )
        .param("tags_json", tags_json)
        .param("color", chain.color.clone().unwrap_or_default())
        .param("is_favorite", chain.is_favorite)
        .param("created_at", chain.created_at.to_rfc3339())
        .param("updated_at", chain.updated_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Created event chain: {}", chain.id);

        Ok(())
    }

    /// Get an event chain by ID
    pub async fn get(&self, id: EventChainId) -> Result<Option<EventChain>> {
        let q = query(
            "MATCH (c:EventChain {id: $id})
            RETURN c",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_event_chain(row)?))
        } else {
            Ok(None)
        }
    }

    /// Update an event chain
    pub async fn update(&self, chain: &EventChain) -> Result<bool> {
        let events_json: Vec<String> = chain.events.iter().map(|id| id.to_string()).collect();
        let completed_json: Vec<String> = chain
            .completed_events
            .iter()
            .map(|id| id.to_string())
            .collect();
        let tags_json = serde_json::to_string(&chain.tags)?;

        let q = query(
            "MATCH (c:EventChain {id: $id})
            SET c.name = $name,
                c.description = $description,
                c.events = $events,
                c.is_active = $is_active,
                c.current_position = $current_position,
                c.completed_events = $completed_events,
                c.act_id = $act_id,
                c.tags_json = $tags_json,
                c.color = $color,
                c.is_favorite = $is_favorite,
                c.updated_at = $updated_at
            RETURN c.id as id",
        )
        .param("id", chain.id.to_string())
        .param("name", chain.name.clone())
        .param("description", chain.description.clone())
        .param("events", events_json)
        .param("is_active", chain.is_active)
        .param("current_position", chain.current_position as i64)
        .param("completed_events", completed_json)
        .param(
            "act_id",
            chain.act_id.map(|a| a.to_string()).unwrap_or_default(),
        )
        .param("tags_json", tags_json)
        .param("color", chain.color.clone().unwrap_or_default())
        .param("is_favorite", chain.is_favorite)
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// List all event chains for a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_EVENT_CHAIN]->(c:EventChain)
            RETURN c
            ORDER BY c.created_at DESC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut chains = Vec::new();

        while let Some(row) = result.next().await? {
            chains.push(row_to_event_chain(row)?);
        }

        Ok(chains)
    }

    /// List active event chains for a world
    pub async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_EVENT_CHAIN]->(c:EventChain)
            WHERE c.is_active = true
            RETURN c
            ORDER BY c.updated_at DESC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut chains = Vec::new();

        while let Some(row) = result.next().await? {
            chains.push(row_to_event_chain(row)?);
        }

        Ok(chains)
    }

    /// List favorite event chains for a world
    pub async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_EVENT_CHAIN]->(c:EventChain)
            WHERE c.is_favorite = true
            RETURN c
            ORDER BY c.name ASC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut chains = Vec::new();

        while let Some(row) = result.next().await? {
            chains.push(row_to_event_chain(row)?);
        }

        Ok(chains)
    }

    /// Get chains containing a specific narrative event
    pub async fn get_chains_for_event(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<Vec<EventChain>> {
        // Search chains where the events array contains this event ID
        let q = query(
            "MATCH (c:EventChain)
            WHERE $event_id IN c.events
            RETURN c
            ORDER BY c.name ASC",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut chains = Vec::new();

        while let Some(row) = result.next().await? {
            chains.push(row_to_event_chain(row)?);
        }

        Ok(chains)
    }

    /// Add an event to a chain
    pub async fn add_event_to_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool> {
        let q = query(
            "MATCH (c:EventChain {id: $chain_id})
            SET c.events = c.events + $event_id,
                c.updated_at = $updated_at
            RETURN c.id as id",
        )
        .param("chain_id", chain_id.to_string())
        .param("event_id", event_id.to_string())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Remove an event from a chain
    pub async fn remove_event_from_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool> {
        let q = query(
            "MATCH (c:EventChain {id: $chain_id})
            SET c.events = [e IN c.events WHERE e <> $event_id],
                c.completed_events = [e IN c.completed_events WHERE e <> $event_id],
                c.updated_at = $updated_at
            RETURN c.id as id",
        )
        .param("chain_id", chain_id.to_string())
        .param("event_id", event_id.to_string())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Mark an event as completed in a chain
    pub async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool> {
        // First check if event is already completed
        let check_q = query(
            "MATCH (c:EventChain {id: $chain_id})
            WHERE NOT $event_id IN c.completed_events
            RETURN c.id as id",
        )
        .param("chain_id", chain_id.to_string())
        .param("event_id", event_id.to_string());

        let mut check_result = self.connection.graph().execute(check_q).await?;
        if check_result.next().await?.is_none() {
            return Ok(false); // Already completed
        }

        // Add to completed and advance position if needed
        let q = query(
            "MATCH (c:EventChain {id: $chain_id})
            WITH c, [i IN range(0, size(c.events)-1) WHERE c.events[i] = $event_id][0] as event_pos
            SET c.completed_events = c.completed_events + $event_id,
                c.current_position = CASE
                    WHEN event_pos IS NOT NULL AND event_pos = c.current_position
                    THEN c.current_position + 1
                    ELSE c.current_position
                END,
                c.updated_at = $updated_at
            RETURN c.id as id",
        )
        .param("chain_id", chain_id.to_string())
        .param("event_id", event_id.to_string())
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Toggle favorite status
    pub async fn toggle_favorite(&self, id: EventChainId) -> Result<bool> {
        let q = query(
            "MATCH (c:EventChain {id: $id})
            SET c.is_favorite = NOT c.is_favorite,
                c.updated_at = $updated_at
            RETURN c.is_favorite as is_favorite",
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
    pub async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<bool> {
        let q = query(
            "MATCH (c:EventChain {id: $id})
            SET c.is_active = $is_active,
                c.updated_at = $updated_at
            RETURN c.id as id",
        )
        .param("id", id.to_string())
        .param("is_active", is_active)
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Reset chain progress
    pub async fn reset(&self, id: EventChainId) -> Result<bool> {
        let empty_vec: Vec<String> = Vec::new();
        let q = query(
            "MATCH (c:EventChain {id: $id})
            SET c.current_position = 0,
                c.completed_events = $empty,
                c.updated_at = $updated_at
            RETURN c.id as id",
        )
        .param("id", id.to_string())
        .param("empty", empty_vec)
        .param("updated_at", Utc::now().to_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Delete an event chain
    pub async fn delete(&self, id: EventChainId) -> Result<bool> {
        let q = query(
            "MATCH (c:EventChain {id: $id})
            DETACH DELETE c
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

    /// Get chain status summary
    pub async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>> {
        if let Some(chain) = self.get(id).await? {
            Ok(Some(ChainStatus::from(&chain)))
        } else {
            Ok(None)
        }
    }

    /// Get all chain statuses for a world
    pub async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>> {
        let chains = self.list_by_world(world_id).await?;
        Ok(chains.iter().map(ChainStatus::from).collect())
    }
}

/// Convert a Neo4j row to an EventChain
fn row_to_event_chain(row: Row) -> Result<EventChain> {
    let node: neo4rs::Node = row.get("c")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let events_strs: Vec<String> = node.get("events").unwrap_or_default();
    let is_active: bool = node.get("is_active").unwrap_or(true);
    let current_position: i64 = node.get("current_position").unwrap_or(0);
    let completed_strs: Vec<String> = node.get("completed_events").unwrap_or_default();
    let act_id_str: String = node.get("act_id").unwrap_or_default();
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());
    let color: String = node.get("color").unwrap_or_default();
    let is_favorite: bool = node.get("is_favorite").unwrap_or(false);
    let created_at_str: String = node.get("created_at")?;
    let updated_at_str: String = node.get("updated_at")?;

    let events: Vec<NarrativeEventId> = events_strs
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok().map(NarrativeEventId::from))
        .collect();

    let completed_events: Vec<NarrativeEventId> = completed_strs
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok().map(NarrativeEventId::from))
        .collect();

    let tags: Vec<String> = serde_json::from_str(&tags_json)?;

    Ok(EventChain {
        id: EventChainId::from(Uuid::parse_str(&id_str)?),
        world_id: WorldId::from(Uuid::parse_str(&world_id_str)?),
        name,
        description,
        events,
        is_active,
        current_position: current_position as u32,
        completed_events,
        act_id: if act_id_str.is_empty() {
            None
        } else {
            Uuid::parse_str(&act_id_str).ok().map(ActId::from)
        },
        tags,
        color: if color.is_empty() { None } else { Some(color) },
        is_favorite,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc),
    })
}

// =============================================================================
// EventChainRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl EventChainRepositoryPort for Neo4jEventChainRepository {
    async fn create(&self, chain: &EventChain) -> Result<()> {
        self.create(chain).await
    }

    async fn get(&self, id: EventChainId) -> Result<Option<EventChain>> {
        self.get(id).await
    }

    async fn update(&self, chain: &EventChain) -> Result<bool> {
        self.update(chain).await
    }

    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        self.list_by_world(world_id).await
    }

    async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        self.list_active(world_id).await
    }

    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        self.list_favorites(world_id).await
    }

    async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>> {
        self.get_chains_for_event(event_id).await
    }

    async fn add_event_to_chain(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<bool> {
        self.add_event_to_chain(chain_id, event_id).await
    }

    async fn remove_event_from_chain(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<bool> {
        self.remove_event_from_chain(chain_id, event_id).await
    }

    async fn complete_event(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<bool> {
        self.complete_event(chain_id, event_id).await
    }

    async fn toggle_favorite(&self, id: EventChainId) -> Result<bool> {
        self.toggle_favorite(id).await
    }

    async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<bool> {
        self.set_active(id, is_active).await
    }

    async fn reset(&self, id: EventChainId) -> Result<bool> {
        self.reset(id).await
    }

    async fn delete(&self, id: EventChainId) -> Result<bool> {
        self.delete(id).await
    }

    async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>> {
        self.get_status(id).await
    }

    async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>> {
        self.list_statuses(world_id).await
    }
}
