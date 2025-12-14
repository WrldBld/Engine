//! Relationship repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::{RelationshipRepositoryPort, SocialNetwork, CharacterNode, RelationshipEdge};
use crate::domain::value_objects::{
    CharacterId, FamilyRelation, Relationship, RelationshipEvent, RelationshipId, RelationshipType,
    WorldId,
};

/// Repository for Relationship (character social network) operations
pub struct Neo4jRelationshipRepository {
    connection: Neo4jConnection,
}

impl Neo4jRelationshipRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a relationship between two characters
    pub async fn create(&self, relationship: &Relationship) -> Result<()> {
        let type_json =
            serde_json::to_string(&RelationshipTypeStored::from(relationship.relationship_type.clone()))?;
        let history_json = serde_json::to_string(
            &relationship
                .history
                .iter()
                .cloned()
                .map(RelationshipEventStored::from)
                .collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH (from:Character {id: $from_id})
            MATCH (to:Character {id: $to_id})
            CREATE (from)-[r:RELATES_TO {
                id: $id,
                relationship_type: $rel_type,
                sentiment: $sentiment,
                history: $history,
                known_to_player: $known_to_player
            }]->(to)
            RETURN r.id as id",
        )
        .param("id", relationship.id.to_string())
        .param("from_id", relationship.from_character.to_string())
        .param("to_id", relationship.to_character.to_string())
        .param("rel_type", type_json)
        .param("sentiment", relationship.sentiment as f64)
        .param("history", history_json)
        .param("known_to_player", relationship.known_to_player);

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Created relationship: {} -> {}",
            relationship.from_character,
            relationship.to_character
        );
        Ok(())
    }

    /// Get a relationship by ID
    pub async fn get(&self, id: RelationshipId) -> Result<Option<Relationship>> {
        let q = query(
            "MATCH (from:Character)-[r:RELATES_TO {id: $id}]->(to:Character)
            RETURN r.id as id, from.id as from_id, to.id as to_id,
                   r.relationship_type as rel_type, r.sentiment as sentiment,
                   r.history as history, r.known_to_player as known_to_player",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_relationship(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get all relationships for a character
    pub async fn get_for_character(&self, character_id: CharacterId) -> Result<Vec<Relationship>> {
        let q = query(
            "MATCH (from:Character {id: $id})-[r:RELATES_TO]->(to:Character)
            RETURN r.id as id, from.id as from_id, to.id as to_id,
                   r.relationship_type as rel_type, r.sentiment as sentiment,
                   r.history as history, r.known_to_player as known_to_player",
        )
        .param("id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut relationships = Vec::new();

        while let Some(row) = result.next().await? {
            relationships.push(row_to_relationship(row)?);
        }

        Ok(relationships)
    }

    /// Get all relationships involving a character (both directions)
    pub async fn get_involving_character(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<Relationship>> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:RELATES_TO]-(other:Character)
            WITH r,
                 CASE WHEN startNode(r).id = $id THEN startNode(r) ELSE endNode(r) END as from,
                 CASE WHEN startNode(r).id = $id THEN endNode(r) ELSE startNode(r) END as to
            RETURN r.id as id, from.id as from_id, to.id as to_id,
                   r.relationship_type as rel_type, r.sentiment as sentiment,
                   r.history as history, r.known_to_player as known_to_player",
        )
        .param("id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut relationships = Vec::new();

        while let Some(row) = result.next().await? {
            relationships.push(row_to_relationship(row)?);
        }

        Ok(relationships)
    }

    /// Get the social network graph for a world
    pub async fn get_social_network(
        &self,
        world_id: WorldId,
    ) -> Result<SocialNetwork> {
        // Get all characters in the world
        let chars_q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHARACTER]->(c:Character)
            RETURN c.id as id, c.name as name, c.current_archetype as archetype",
        )
        .param("world_id", world_id.to_string());

        let mut chars_result = self.connection.graph().execute(chars_q).await?;
        let mut nodes = Vec::new();

        while let Some(row) = chars_result.next().await? {
            let id: String = row.get("id")?;
            let name: String = row.get("name")?;
            let archetype: String = row.get("archetype")?;
            nodes.push(CharacterNode {
                id,
                name,
                archetype,
            });
        }

        // Get all relationships between characters in the world
        let rels_q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHARACTER]->(from:Character)
            MATCH (from)-[r:RELATES_TO]->(to:Character)<-[:CONTAINS_CHARACTER]-(w)
            RETURN from.id as from_id, to.id as to_id,
                   r.relationship_type as rel_type, r.sentiment as sentiment",
        )
        .param("world_id", world_id.to_string());

        let mut rels_result = self.connection.graph().execute(rels_q).await?;
        let mut edges = Vec::new();

        while let Some(row) = rels_result.next().await? {
            let from_id: String = row.get("from_id")?;
            let to_id: String = row.get("to_id")?;
            let rel_type: String = row.get("rel_type")?;
            let sentiment: f64 = row.get("sentiment")?;
            edges.push(RelationshipEdge {
                from_id,
                to_id,
                relationship_type: rel_type,
                sentiment: sentiment as f32,
            });
        }

        Ok(SocialNetwork {
            characters: nodes,
            relationships: edges,
        })
    }

    /// Update a relationship
    pub async fn update(&self, relationship: &Relationship) -> Result<()> {
        let type_json =
            serde_json::to_string(&RelationshipTypeStored::from(relationship.relationship_type.clone()))?;
        let history_json = serde_json::to_string(
            &relationship
                .history
                .iter()
                .cloned()
                .map(RelationshipEventStored::from)
                .collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH ()-[r:RELATES_TO {id: $id}]->()
            SET r.relationship_type = $rel_type,
                r.sentiment = $sentiment,
                r.history = $history,
                r.known_to_player = $known_to_player
            RETURN r.id as id",
        )
        .param("id", relationship.id.to_string())
        .param("rel_type", type_json)
        .param("sentiment", relationship.sentiment as f64)
        .param("history", history_json)
        .param("known_to_player", relationship.known_to_player);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated relationship: {}", relationship.id);
        Ok(())
    }

    /// Add an event to a relationship's history
    pub async fn add_event(&self, id: RelationshipId, event: RelationshipEvent) -> Result<()> {
        if let Some(mut relationship) = self.get(id).await? {
            relationship.history.push(event);
            self.update(&relationship).await?;
        }
        Ok(())
    }

    /// Update sentiment on a relationship
    pub async fn update_sentiment(&self, id: RelationshipId, sentiment: f32) -> Result<()> {
        let q = query(
            "MATCH ()-[r:RELATES_TO {id: $id}]->()
            SET r.sentiment = $sentiment
            RETURN r.id as id",
        )
        .param("id", id.to_string())
        .param("sentiment", sentiment as f64);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Delete a relationship
    pub async fn delete(&self, id: RelationshipId) -> Result<()> {
        let q = query(
            "MATCH ()-[r:RELATES_TO {id: $id}]->()
            DELETE r",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted relationship: {}", id);
        Ok(())
    }

    // ========================================
    // Social Network Query Methods
    // ========================================

    /// Get all connections for a character (alias for get_involving_character)
    ///
    /// Returns all relationships where the character is either the source or target.
    pub async fn get_character_connections(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<Relationship>> {
        self.get_involving_character(character_id).await
    }

    /// Find the shortest path between two characters through their relationships
    ///
    /// Uses Neo4j's shortest path algorithm to find the path with fewest hops.
    /// Returns None if no path exists between the characters.
    pub async fn find_path_between(
        &self,
        char_a: CharacterId,
        char_b: CharacterId,
    ) -> Result<Option<RelationshipPath>> {
        // First query: get the character IDs in the path
        let path_q = query(
            "MATCH (a:Character {id: $char_a}), (b:Character {id: $char_b})
            MATCH path = shortestPath((a)-[:RELATES_TO*]-(b))
            WITH nodes(path) as ns, relationships(path) as rs
            RETURN [n IN ns | n.id] as char_ids,
                   [r IN rs | {
                       from_id: startNode(r).id,
                       to_id: endNode(r).id,
                       rel_type: r.relationship_type,
                       sentiment: r.sentiment
                   }] as rels_json,
                   size(rs) as path_length"
        )
        .param("char_a", char_a.to_string())
        .param("char_b", char_b.to_string());

        let mut result = self.connection.graph().execute(path_q).await?;

        if let Some(row) = result.next().await? {
            let char_ids: Vec<String> = row.get("char_ids")?;
            let path_length: i64 = row.get("path_length")?;

            // Get relationship details for each edge in the path
            // We need to query each relationship individually since the path returns minimal data
            let mut relationships = Vec::new();

            for i in 0..(char_ids.len().saturating_sub(1)) {
                let from_id = &char_ids[i];
                let to_id = &char_ids[i + 1];

                // Query for the relationship between consecutive characters
                let rel_q = query(
                    "MATCH (a:Character {id: $from_id})-[r:RELATES_TO]-(b:Character {id: $to_id})
                    RETURN startNode(r).id as from_id, endNode(r).id as to_id,
                           r.relationship_type as rel_type, r.sentiment as sentiment
                    LIMIT 1"
                )
                .param("from_id", from_id.clone())
                .param("to_id", to_id.clone());

                let mut rel_result = self.connection.graph().execute(rel_q).await?;
                if let Some(rel_row) = rel_result.next().await? {
                    let edge_from: String = rel_row.get("from_id")?;
                    let edge_to: String = rel_row.get("to_id")?;
                    let rel_type: String = rel_row.get("rel_type")?;
                    let sentiment: f64 = rel_row.get("sentiment")?;

                    relationships.push(RelationshipEdge {
                        from_id: edge_from,
                        to_id: edge_to,
                        relationship_type: rel_type,
                        sentiment: sentiment as f32,
                    });
                }
            }

            Ok(Some(RelationshipPath {
                characters: char_ids,
                relationships,
                length: path_length as usize,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find all mutual connections between two characters
    ///
    /// Returns characters that have relationships with both char_a and char_b.
    pub async fn get_mutual_connections(
        &self,
        char_a: CharacterId,
        char_b: CharacterId,
    ) -> Result<Vec<CharacterNode>> {
        let q = query(
            "MATCH (a:Character {id: $char_a})-[:RELATES_TO]-(mutual:Character)-[:RELATES_TO]-(b:Character {id: $char_b})
            WHERE a <> b AND mutual <> a AND mutual <> b
            RETURN DISTINCT mutual.id as id, mutual.name as name, mutual.current_archetype as archetype"
        )
        .param("char_a", char_a.to_string())
        .param("char_b", char_b.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut mutual_connections = Vec::new();

        while let Some(row) = result.next().await? {
            let id: String = row.get("id")?;
            let name: String = row.get("name")?;
            let archetype: String = row.get("archetype")?;
            mutual_connections.push(CharacterNode {
                id,
                name,
                archetype,
            });
        }

        Ok(mutual_connections)
    }

    /// Get characters connected to the given character with sentiment above or below a threshold
    ///
    /// If `above_threshold` is true, returns characters with sentiment >= threshold.
    /// If `above_threshold` is false, returns characters with sentiment <= threshold.
    ///
    /// # Arguments
    /// * `character_id` - The source character to check relationships from
    /// * `threshold` - The sentiment threshold (-1.0 to 1.0)
    /// * `above_threshold` - Whether to find characters above (true) or below (false) the threshold
    pub async fn get_characters_by_sentiment_threshold(
        &self,
        character_id: CharacterId,
        threshold: f32,
        above_threshold: bool,
    ) -> Result<Vec<CharacterWithSentiment>> {
        let comparison = if above_threshold { ">=" } else { "<=" };

        // Using string formatting for the comparison operator since it cannot be parameterized
        let cypher = format!(
            "MATCH (c:Character {{id: $char_id}})-[r:RELATES_TO]-(other:Character)
            WHERE r.sentiment {} $threshold
            RETURN other.id as id, other.name as name, other.current_archetype as archetype,
                   r.sentiment as sentiment, r.relationship_type as rel_type",
            comparison
        );

        let q = query(&cypher)
            .param("char_id", character_id.to_string())
            .param("threshold", threshold as f64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            let id: String = row.get("id")?;
            let name: String = row.get("name")?;
            let archetype: String = row.get("archetype")?;
            let sentiment: f64 = row.get("sentiment")?;
            let rel_type: String = row.get("rel_type")?;

            characters.push(CharacterWithSentiment {
                character: CharacterNode {
                    id,
                    name,
                    archetype,
                },
                sentiment: sentiment as f32,
                relationship_type: rel_type,
            });
        }

        Ok(characters)
    }

    /// Get characters with positive sentiment towards the given character
    ///
    /// Convenience method that returns characters with sentiment > 0.
    pub async fn get_allies(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<CharacterWithSentiment>> {
        self.get_characters_by_sentiment_threshold(character_id, 0.0, true)
            .await
    }

    /// Get characters with negative sentiment towards the given character
    ///
    /// Convenience method that returns characters with sentiment < 0.
    pub async fn get_enemies(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<CharacterWithSentiment>> {
        self.get_characters_by_sentiment_threshold(character_id, 0.0, false)
            .await
    }

    /// List all relationships for a character (outgoing only)
    ///
    /// This is an alias for `get_for_character` to match standard CRUD naming.
    pub async fn list(&self, character_id: CharacterId) -> Result<Vec<Relationship>> {
        self.get_for_character(character_id).await
    }
}

fn row_to_relationship(row: Row) -> Result<Relationship> {
    let id_str: String = row.get("id")?;
    let from_id_str: String = row.get("from_id")?;
    let to_id_str: String = row.get("to_id")?;
    let rel_type_json: String = row.get("rel_type")?;
    let sentiment: f64 = row.get("sentiment")?;
    let history_json: String = row.get("history")?;
    let known_to_player: bool = row.get("known_to_player")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let from_id = uuid::Uuid::parse_str(&from_id_str)?;
    let to_id = uuid::Uuid::parse_str(&to_id_str)?;
    let relationship_type: RelationshipType =
        serde_json::from_str::<RelationshipTypeStored>(&rel_type_json)?.into();
    let history: Vec<RelationshipEvent> =
        serde_json::from_str::<Vec<RelationshipEventStored>>(&history_json)?
            .into_iter()
            .map(Into::into)
            .collect();

    Ok(Relationship {
        id: RelationshipId::from_uuid(id),
        from_character: CharacterId::from_uuid(from_id),
        to_character: CharacterId::from_uuid(to_id),
        relationship_type,
        sentiment: sentiment as f32,
        history,
        known_to_player,
    })
}

// ============================================================================
// Persistence serde models (so domain doesn't require serde)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
enum RelationshipTypeStored {
    Family(FamilyRelationStored),
    Romantic,
    Professional,
    Rivalry,
    Friendship,
    Mentorship,
    Enmity,
    Custom(String),
}

impl From<RelationshipType> for RelationshipTypeStored {
    fn from(value: RelationshipType) -> Self {
        match value {
            RelationshipType::Family(fr) => Self::Family(fr.into()),
            RelationshipType::Romantic => Self::Romantic,
            RelationshipType::Professional => Self::Professional,
            RelationshipType::Rivalry => Self::Rivalry,
            RelationshipType::Friendship => Self::Friendship,
            RelationshipType::Mentorship => Self::Mentorship,
            RelationshipType::Enmity => Self::Enmity,
            RelationshipType::Custom(s) => Self::Custom(s),
        }
    }
}

impl From<RelationshipTypeStored> for RelationshipType {
    fn from(value: RelationshipTypeStored) -> Self {
        match value {
            RelationshipTypeStored::Family(fr) => Self::Family(fr.into()),
            RelationshipTypeStored::Romantic => Self::Romantic,
            RelationshipTypeStored::Professional => Self::Professional,
            RelationshipTypeStored::Rivalry => Self::Rivalry,
            RelationshipTypeStored::Friendship => Self::Friendship,
            RelationshipTypeStored::Mentorship => Self::Mentorship,
            RelationshipTypeStored::Enmity => Self::Enmity,
            RelationshipTypeStored::Custom(s) => Self::Custom(s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum FamilyRelationStored {
    Parent,
    Child,
    Sibling,
    Spouse,
    Grandparent,
    Grandchild,
    AuntUncle,
    NieceNephew,
    Cousin,
}

impl From<FamilyRelation> for FamilyRelationStored {
    fn from(value: FamilyRelation) -> Self {
        match value {
            FamilyRelation::Parent => Self::Parent,
            FamilyRelation::Child => Self::Child,
            FamilyRelation::Sibling => Self::Sibling,
            FamilyRelation::Spouse => Self::Spouse,
            FamilyRelation::Grandparent => Self::Grandparent,
            FamilyRelation::Grandchild => Self::Grandchild,
            FamilyRelation::AuntUncle => Self::AuntUncle,
            FamilyRelation::NieceNephew => Self::NieceNephew,
            FamilyRelation::Cousin => Self::Cousin,
        }
    }
}

impl From<FamilyRelationStored> for FamilyRelation {
    fn from(value: FamilyRelationStored) -> Self {
        match value {
            FamilyRelationStored::Parent => Self::Parent,
            FamilyRelationStored::Child => Self::Child,
            FamilyRelationStored::Sibling => Self::Sibling,
            FamilyRelationStored::Spouse => Self::Spouse,
            FamilyRelationStored::Grandparent => Self::Grandparent,
            FamilyRelationStored::Grandchild => Self::Grandchild,
            FamilyRelationStored::AuntUncle => Self::AuntUncle,
            FamilyRelationStored::NieceNephew => Self::NieceNephew,
            FamilyRelationStored::Cousin => Self::Cousin,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelationshipEventStored {
    pub description: String,
    pub sentiment_change: f32,
    pub timestamp: String,
}

impl From<RelationshipEvent> for RelationshipEventStored {
    fn from(value: RelationshipEvent) -> Self {
        Self {
            description: value.description,
            sentiment_change: value.sentiment_change,
            timestamp: value.timestamp.to_rfc3339(),
        }
    }
}

impl From<RelationshipEventStored> for RelationshipEvent {
    fn from(value: RelationshipEventStored) -> Self {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&value.timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Self {
            description: value.description,
            sentiment_change: value.sentiment_change,
            timestamp,
        }
    }
}

/// Path between two characters through their relationships
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationshipPath {
    /// Ordered list of character IDs in the path
    pub characters: Vec<String>,
    /// Relationships connecting the characters (length = characters.len() - 1)
    pub relationships: Vec<RelationshipEdge>,
    /// Total path length (number of hops)
    pub length: usize,
}

/// Character with sentiment information for threshold queries
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterWithSentiment {
    pub character: CharacterNode,
    pub sentiment: f32,
    pub relationship_type: String,
}

// =============================================================================
// RelationshipRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl RelationshipRepositoryPort for Neo4jRelationshipRepository {
    async fn create(&self, relationship: &Relationship) -> Result<()> {
        Neo4jRelationshipRepository::create(self, relationship).await
    }

    async fn get(&self, id: RelationshipId) -> Result<Option<Relationship>> {
        Neo4jRelationshipRepository::get(self, id).await
    }

    async fn get_for_character(&self, character_id: CharacterId) -> Result<Vec<Relationship>> {
        Neo4jRelationshipRepository::get_for_character(self, character_id).await
    }

    async fn update(&self, relationship: &Relationship) -> Result<()> {
        Neo4jRelationshipRepository::update(self, relationship).await
    }

    async fn delete(&self, id: RelationshipId) -> Result<()> {
        Neo4jRelationshipRepository::delete(self, id).await
    }

    async fn get_social_network(&self, world_id: WorldId) -> Result<SocialNetwork> {
        Neo4jRelationshipRepository::get_social_network(self, world_id).await
    }
}
