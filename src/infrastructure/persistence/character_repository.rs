//! Character repository implementation for Neo4j
//!
//! # Graph-First Design (Phase 0.C)
//!
//! This repository uses Neo4j edges for all relationships:
//! - Wants: `(Character)-[:HAS_WANT]->(Want)` + `(Want)-[:TARGETS]->(target)`
//! - Inventory: `(Character)-[:POSSESSES]->(Item)`
//! - Location: `HOME_LOCATION`, `WORKS_AT`, `FREQUENTS`, `AVOIDS`
//! - Actantial: `VIEWS_AS_HELPER`, `VIEWS_AS_OPPONENT`, etc.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};

use super::connection::Neo4jConnection;
use crate::application::dto::parse_archetype;
use crate::application::ports::outbound::CharacterRepositoryPort;
use crate::domain::entities::{
    ActantialRole, ActantialView, AcquisitionMethod, Character, CharacterWant, FrequencyLevel,
    InventoryItem, Item, StatBlock, Want,
};
use crate::domain::value_objects::{
    ArchetypeChange, CampbellArchetype, CharacterId, ItemId, LocationId, RegionFrequency, RegionId,
    RegionRelationship, RegionRelationshipType, RegionShift, SceneId, WantId, WorldId,
};

/// Repository for Character operations
pub struct Neo4jCharacterRepository {
    connection: Neo4jConnection,
}

impl Neo4jCharacterRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    // =========================================================================
    // Core CRUD
    // =========================================================================

    /// Create a new character
    pub async fn create(&self, character: &Character) -> Result<()> {
        let stats_json = serde_json::to_string(&StatBlockStored::from(character.stats.clone()))?;
        let archetype_history_json = serde_json::to_string(
            &character
                .archetype_history
                .iter()
                .cloned()
                .map(ArchetypeChangeStored::from)
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
                stats: $stats,
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
        .param("stats", stats_json)
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

    /// Get all characters featured in a scene (via FEATURES_CHARACTER edge)
    pub async fn get_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[:FEATURES_CHARACTER]->(c:Character)
            RETURN c
            ORDER BY c.name",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            characters.push(row_to_character(row)?);
        }

        Ok(characters)
    }

    /// Update a character
    pub async fn update(&self, character: &Character) -> Result<()> {
        let stats_json = serde_json::to_string(&StatBlockStored::from(character.stats.clone()))?;
        let archetype_history_json = serde_json::to_string(
            &character
                .archetype_history
                .iter()
                .cloned()
                .map(ArchetypeChangeStored::from)
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
                c.stats = $stats,
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
        .param("stats", stats_json)
        .param("is_alive", character.is_alive)
        .param("is_active", character.is_active);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated character: {}", character.name);
        Ok(())
    }

    /// Delete a character (cascading deletes wants, actantial views, inventory edges)
    pub async fn delete(&self, id: CharacterId) -> Result<()> {
        // First delete all Want nodes connected to this character
        let delete_wants = query(
            "MATCH (c:Character {id: $id})-[:HAS_WANT]->(w:Want)
            DETACH DELETE w",
        )
        .param("id", id.to_string());
        self.connection.graph().run(delete_wants).await?;

        // Then delete the character itself
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
        if let Some(mut character) = self.get(id).await? {
            character.change_archetype(new_archetype, reason);
            self.update(&character).await?;
        }
        Ok(())
    }

    // =========================================================================
    // Wants
    // =========================================================================

    /// Create a want and attach it to a character
    pub async fn create_want(
        &self,
        character_id: CharacterId,
        want: &Want,
        priority: u32,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})
            CREATE (w:Want {
                id: $id,
                description: $description,
                intensity: $intensity,
                known_to_player: $known_to_player,
                created_at: $created_at
            })
            CREATE (c)-[:HAS_WANT {
                priority: $priority,
                acquired_at: $acquired_at
            }]->(w)
            RETURN w.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("id", want.id.to_string())
        .param("description", want.description.clone())
        .param("intensity", want.intensity as f64)
        .param("known_to_player", want.known_to_player)
        .param("created_at", want.created_at.to_rfc3339())
        .param("priority", priority as i64)
        .param("acquired_at", Utc::now().to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Created want for character {}: {}", character_id, want.description);
        Ok(())
    }

    /// Get all wants for a character
    pub async fn get_wants(&self, character_id: CharacterId) -> Result<Vec<CharacterWant>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:HAS_WANT]->(w:Want)
            RETURN w, r.priority as priority, r.acquired_at as acquired_at
            ORDER BY r.priority",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut wants = Vec::new();

        while let Some(row) = result.next().await? {
            let want = row_to_want(&row)?;
            let priority: i64 = row.get("priority")?;
            let acquired_at_str: String = row.get("acquired_at")?;
            let acquired_at = DateTime::parse_from_rfc3339(&acquired_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            wants.push(CharacterWant {
                want,
                priority: priority as u32,
                acquired_at,
            });
        }

        Ok(wants)
    }

    /// Update a want
    pub async fn update_want(&self, want: &Want) -> Result<()> {
        let q = query(
            "MATCH (w:Want {id: $id})
            SET w.description = $description,
                w.intensity = $intensity,
                w.known_to_player = $known_to_player
            RETURN w.id as id",
        )
        .param("id", want.id.to_string())
        .param("description", want.description.clone())
        .param("intensity", want.intensity as f64)
        .param("known_to_player", want.known_to_player);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Delete a want
    pub async fn delete_want(&self, want_id: WantId) -> Result<()> {
        let q = query(
            "MATCH (w:Want {id: $id})
            DETACH DELETE w",
        )
        .param("id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set a want's target (creates TARGETS edge)
    pub async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: &str,
        target_type: &str,
    ) -> Result<()> {
        // First remove any existing target
        self.remove_want_target(want_id).await?;

        // Create the new TARGETS edge based on target type
        let cypher = match target_type {
            "Character" => {
                "MATCH (w:Want {id: $want_id}), (t:Character {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            "Item" => {
                "MATCH (w:Want {id: $want_id}), (t:Item {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            "Goal" => {
                "MATCH (w:Want {id: $want_id}), (t:Goal {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            _ => return Err(anyhow::anyhow!("Invalid target type: {}", target_type)),
        };

        let q = query(cypher)
            .param("want_id", want_id.to_string())
            .param("target_id", target_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove a want's target
    pub async fn remove_want_target(&self, want_id: WantId) -> Result<()> {
        let q = query(
            "MATCH (w:Want {id: $id})-[r:TARGETS]->()
            DELETE r",
        )
        .param("id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    // =========================================================================
    // Actantial Views
    // =========================================================================

    /// Add an actantial view
    pub async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}}), (t:Character {{id: $target_id}})
            CREATE (s)-[:{} {{
                want_id: $want_id,
                reason: $reason,
                assigned_at: $assigned_at
            }}]->(t)",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", view.want_id.to_string())
            .param("reason", view.reason.clone())
            .param("assigned_at", view.assigned_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get all actantial views for a character
    pub async fn get_actantial_views(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<(ActantialRole, CharacterId, ActantialView)>> {
        let q = query(
            "MATCH (s:Character {id: $id})-[r]->(t:Character)
            WHERE type(r) IN ['VIEWS_AS_HELPER', 'VIEWS_AS_OPPONENT', 'VIEWS_AS_SENDER', 'VIEWS_AS_RECEIVER']
            RETURN type(r) as role_type, t.id as target_id, r.want_id as want_id, r.reason as reason, r.assigned_at as assigned_at",
        )
        .param("id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut views = Vec::new();

        while let Some(row) = result.next().await? {
            let role_type: String = row.get("role_type")?;
            let target_id_str: String = row.get("target_id")?;
            let want_id_str: String = row.get("want_id")?;
            let reason: String = row.get("reason")?;
            let assigned_at_str: String = row.get("assigned_at")?;

            let role = match role_type.as_str() {
                "VIEWS_AS_HELPER" => ActantialRole::Helper,
                "VIEWS_AS_OPPONENT" => ActantialRole::Opponent,
                "VIEWS_AS_SENDER" => ActantialRole::Sender,
                "VIEWS_AS_RECEIVER" => ActantialRole::Receiver,
                _ => continue,
            };

            let target_id =
                CharacterId::from_uuid(uuid::Uuid::parse_str(&target_id_str)?);
            let want_id = WantId::from_uuid(uuid::Uuid::parse_str(&want_id_str)?);
            let assigned_at = DateTime::parse_from_rfc3339(&assigned_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            views.push((
                role,
                target_id,
                ActantialView {
                    want_id,
                    reason,
                    assigned_at,
                },
            ));
        }

        Ok(views)
    }

    /// Remove an actantial view
    pub async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        want_id: WantId,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}})-[r:{} {{want_id: $want_id}}]->(t:Character {{id: $target_id}})
            DELETE r",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    // =========================================================================
    // Inventory
    // =========================================================================

    /// Add an item to character's inventory
    pub async fn add_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()> {
        let method_str = acquisition_method
            .map(|m| m.to_string())
            .unwrap_or_default();

        let q = query(
            "MATCH (c:Character {id: $character_id}), (i:Item {id: $item_id})
            CREATE (c)-[:POSSESSES {
                quantity: $quantity,
                equipped: $equipped,
                acquired_at: $acquired_at,
                acquisition_method: $acquisition_method
            }]->(i)
            RETURN i.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string())
        .param("quantity", quantity as i64)
        .param("equipped", equipped)
        .param("acquired_at", Utc::now().to_rfc3339())
        .param("acquisition_method", method_str);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get character's inventory
    pub async fn get_inventory(&self, character_id: CharacterId) -> Result<Vec<InventoryItem>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item)
            RETURN i, r.quantity as quantity, r.equipped as equipped, 
                   r.acquired_at as acquired_at, r.acquisition_method as acquisition_method",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut inventory = Vec::new();

        while let Some(row) = result.next().await? {
            let item = row_to_item(&row)?;
            let quantity: i64 = row.get("quantity")?;
            let equipped: bool = row.get("equipped")?;
            let acquired_at_str: String = row.get("acquired_at")?;
            let acquisition_method_str: String = row.get("acquisition_method").unwrap_or_default();

            let acquired_at = DateTime::parse_from_rfc3339(&acquired_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let acquisition_method = if acquisition_method_str.is_empty() {
                None
            } else {
                acquisition_method_str.parse().ok()
            };

            inventory.push(InventoryItem {
                item,
                quantity: quantity as u32,
                equipped,
                acquired_at,
                acquisition_method,
            });
        }

        Ok(inventory)
    }

    /// Update inventory item
    pub async fn update_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            SET r.quantity = $quantity, r.equipped = $equipped
            RETURN i.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string())
        .param("quantity", quantity as i64)
        .param("equipped", equipped);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove an item from inventory
    pub async fn remove_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    // =========================================================================
    // Character-Location Relationships
    // =========================================================================

    /// Set character's home location
    pub async fn set_home_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        description: Option<String>,
    ) -> Result<()> {
        // Remove existing home first
        self.remove_home_location(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:HOME_LOCATION {description: $description}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("description", description.unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove character's home location
    pub async fn remove_home_location(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:HOME_LOCATION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set character's work location
    pub async fn set_work_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        role: String,
        schedule: Option<String>,
    ) -> Result<()> {
        // Remove existing work first
        self.remove_work_location(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:WORKS_AT {role: $role, schedule: $schedule}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("role", role)
        .param("schedule", schedule.unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove character's work location
    pub async fn remove_work_location(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:WORKS_AT]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add a frequented location
    pub async fn add_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        frequency: FrequencyLevel,
        time_of_day: String,
        day_of_week: Option<String>,
        reason: Option<String>,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:FREQUENTS {
                frequency: $frequency,
                time_of_day: $time_of_day,
                day_of_week: $day_of_week,
                reason: $reason,
                since: $since
            }]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("frequency", frequency.to_string())
        .param("time_of_day", time_of_day)
        .param("day_of_week", day_of_week.unwrap_or_default())
        .param("reason", reason.unwrap_or_default())
        .param("since", Utc::now().to_rfc3339());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove a frequented location
    pub async fn remove_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:FREQUENTS]->(l:Location {id: $location_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add an avoided location
    pub async fn add_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        reason: String,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:AVOIDS {reason: $reason}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("reason", reason);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove an avoided location
    pub async fn remove_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:AVOIDS]->(l:Location {id: $location_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get NPCs who might be at a location
    pub async fn get_npcs_at_location(
        &self,
        location_id: LocationId,
        time_of_day: Option<&str>,
    ) -> Result<Vec<Character>> {
        // Build query based on whether time_of_day filter is provided
        let cypher = if time_of_day.is_some() {
            "MATCH (c:Character)-[r]->(l:Location {id: $location_id})
            WHERE (type(r) = 'HOME_LOCATION')
               OR (type(r) = 'WORKS_AT' AND (r.schedule IS NULL OR r.schedule = '' OR r.schedule = $time_of_day))
               OR (type(r) = 'FREQUENTS' AND (r.time_of_day = 'Any' OR r.time_of_day = $time_of_day))
            RETURN DISTINCT c"
        } else {
            "MATCH (c:Character)-[r]->(l:Location {id: $location_id})
            WHERE type(r) IN ['HOME_LOCATION', 'WORKS_AT', 'FREQUENTS']
            RETURN DISTINCT c"
        };

        let q = query(cypher)
            .param("location_id", location_id.to_string())
            .param("time_of_day", time_of_day.unwrap_or(""));

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            characters.push(row_to_character(row)?);
        }

        Ok(characters)
    }

    // =========================================================================
    // Character-Region Relationships (Phase 23C)
    // =========================================================================

    /// Set character's home region
    pub async fn set_home_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        // Remove existing home region first
        self.remove_home_region(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:HOME_REGION]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set home region for character {}: {}",
            character_id,
            region_id
        );
        Ok(())
    }

    /// Remove character's home region
    pub async fn remove_home_region(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:HOME_REGION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set character's work region with shift (day, night, always)
    pub async fn set_work_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        shift: RegionShift,
    ) -> Result<()> {
        // Remove existing work region first
        self.remove_work_region(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:WORKS_AT_REGION {shift: $shift}]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("shift", shift.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set work region for character {}: {} ({:?})",
            character_id,
            region_id,
            shift
        );
        Ok(())
    }

    /// Remove character's work region
    pub async fn remove_work_region(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:WORKS_AT_REGION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add a frequented region
    pub async fn add_frequented_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        frequency: RegionFrequency,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:FREQUENTS_REGION {
                frequency: $frequency,
                since: $since
            }]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("frequency", frequency.to_string())
        .param("since", Utc::now().to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added frequented region for character {}: {} ({:?})",
            character_id,
            region_id,
            frequency
        );
        Ok(())
    }

    /// Remove a frequented region
    pub async fn remove_frequented_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:FREQUENTS_REGION]->(reg:Region {id: $region_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add an avoided region
    pub async fn add_avoided_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        reason: String,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:AVOIDS_REGION {reason: $reason}]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("reason", reason);

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added avoided region for character {}: {}",
            character_id,
            region_id
        );
        Ok(())
    }

    /// Remove an avoided region
    pub async fn remove_avoided_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:AVOIDS_REGION]->(reg:Region {id: $region_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// List all region relationships for a character
    pub async fn list_region_relationships(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<RegionRelationship>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r]->(reg:Region)
            WHERE type(r) IN ['HOME_REGION', 'WORKS_AT_REGION', 'FREQUENTS_REGION', 'AVOIDS_REGION']
            RETURN type(r) as rel_type, reg.id as region_id, reg.name as region_name,
                   r.shift as shift, r.frequency as frequency, r.reason as reason",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut relationships = Vec::new();

        while let Some(row) = result.next().await? {
            let rel_type: String = row.get("rel_type")?;
            let region_id_str: String = row.get("region_id")?;
            let region_name: String = row.get("region_name")?;

            let region_id = RegionId::from_uuid(uuid::Uuid::parse_str(&region_id_str)?);

            let relationship_type = match rel_type.as_str() {
                "HOME_REGION" => RegionRelationshipType::Home,
                "WORKS_AT_REGION" => {
                    let shift_str: String = row.get("shift").unwrap_or_default();
                    let shift = shift_str.parse().unwrap_or(RegionShift::Always);
                    RegionRelationshipType::WorksAt { shift }
                }
                "FREQUENTS_REGION" => {
                    let freq_str: String = row.get("frequency").unwrap_or_default();
                    let frequency = freq_str.parse().unwrap_or(RegionFrequency::Sometimes);
                    RegionRelationshipType::Frequents { frequency }
                }
                "AVOIDS_REGION" => {
                    let reason: String = row.get("reason").unwrap_or_default();
                    RegionRelationshipType::Avoids { reason }
                }
                _ => continue,
            };

            relationships.push(RegionRelationship {
                region_id,
                region_name,
                relationship_type,
            });
        }

        Ok(relationships)
    }

    /// Get all NPCs with any relationship to a region
    pub async fn get_npcs_related_to_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>> {
        let q = query(
            "MATCH (c:Character)-[r]->(reg:Region {id: $region_id})
            WHERE type(r) IN ['HOME_REGION', 'WORKS_AT_REGION', 'FREQUENTS_REGION', 'AVOIDS_REGION']
            RETURN c, type(r) as rel_type, r.shift as shift, r.frequency as frequency, r.reason as reason",
        )
        .param("region_id", region_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut npcs = Vec::new();

        while let Some(row) = result.next().await? {
            // Extract relationship data first (before consuming row for character)
            let rel_type: String = row.get("rel_type")?;
            let shift_str: String = row.get("shift").unwrap_or_default();
            let freq_str: String = row.get("frequency").unwrap_or_default();
            let reason: String = row.get("reason").unwrap_or_default();

            let relationship_type = match rel_type.as_str() {
                "HOME_REGION" => RegionRelationshipType::Home,
                "WORKS_AT_REGION" => {
                    let shift = shift_str.parse().unwrap_or(RegionShift::Always);
                    RegionRelationshipType::WorksAt { shift }
                }
                "FREQUENTS_REGION" => {
                    let frequency = freq_str.parse().unwrap_or(RegionFrequency::Sometimes);
                    RegionRelationshipType::Frequents { frequency }
                }
                "AVOIDS_REGION" => RegionRelationshipType::Avoids { reason },
                _ => continue,
            };

            let character = row_to_character(row)?;
            npcs.push((character, relationship_type));
        }

        Ok(npcs)
    }
}

// =============================================================================
// Row Conversion Helpers
// =============================================================================

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
    let stats_json: String = node.get("stats")?;
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
    let stats: StatBlock = serde_json::from_str::<StatBlockStored>(&stats_json)?.into();

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
        stats,
        is_alive,
        is_active,
    })
}

fn row_to_want(row: &Row) -> Result<Want> {
    let node: neo4rs::Node = row.get("w")?;

    let id_str: String = node.get("id")?;
    let description: String = node.get("description")?;
    let intensity: f64 = node.get("intensity")?;
    let known_to_player: bool = node.get("known_to_player")?;
    let created_at_str: String = node.get("created_at")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(Want {
        id: WantId::from_uuid(id),
        description,
        intensity: intensity as f32,
        known_to_player,
        created_at,
    })
}

fn row_to_item(row: &Row) -> Result<Item> {
    let node: neo4rs::Node = row.get("i")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let item_type: String = node.get("item_type").unwrap_or_default();
    let is_unique: bool = node.get("is_unique").unwrap_or(false);
    let properties: String = node.get("properties").unwrap_or_default();

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = uuid::Uuid::parse_str(&world_id_str)?;

    Ok(Item {
        id: ItemId::from_uuid(id),
        world_id: WorldId::from_uuid(world_id),
        name,
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
        item_type: if item_type.is_empty() {
            None
        } else {
            Some(item_type)
        },
        is_unique,
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
    })
}

// =============================================================================
// Persistence serde models
// =============================================================================

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

    async fn get_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>> {
        Neo4jCharacterRepository::get_by_scene(self, scene_id).await
    }

    // Wants
    async fn create_want(
        &self,
        character_id: CharacterId,
        want: &Want,
        priority: u32,
    ) -> Result<()> {
        Neo4jCharacterRepository::create_want(self, character_id, want, priority).await
    }

    async fn get_wants(&self, character_id: CharacterId) -> Result<Vec<CharacterWant>> {
        Neo4jCharacterRepository::get_wants(self, character_id).await
    }

    async fn update_want(&self, want: &Want) -> Result<()> {
        Neo4jCharacterRepository::update_want(self, want).await
    }

    async fn delete_want(&self, want_id: WantId) -> Result<()> {
        Neo4jCharacterRepository::delete_want(self, want_id).await
    }

    async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: &str,
        target_type: &str,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_want_target(self, want_id, target_id, target_type).await
    }

    async fn remove_want_target(&self, want_id: WantId) -> Result<()> {
        Neo4jCharacterRepository::remove_want_target(self, want_id).await
    }

    // Actantial Views
    async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_actantial_view(self, subject_id, role, target_id, view).await
    }

    async fn get_actantial_views(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<(ActantialRole, CharacterId, ActantialView)>> {
        Neo4jCharacterRepository::get_actantial_views(self, character_id).await
    }

    async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        want_id: WantId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_actantial_view(self, subject_id, role, target_id, want_id)
            .await
    }

    // Inventory
    async fn add_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_inventory_item(
            self,
            character_id,
            item_id,
            quantity,
            equipped,
            acquisition_method,
        )
        .await
    }

    async fn get_inventory(&self, character_id: CharacterId) -> Result<Vec<InventoryItem>> {
        Neo4jCharacterRepository::get_inventory(self, character_id).await
    }

    async fn update_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
    ) -> Result<()> {
        Neo4jCharacterRepository::update_inventory_item(self, character_id, item_id, quantity, equipped)
            .await
    }

    async fn remove_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_inventory_item(self, character_id, item_id).await
    }

    // Character-Location Relationships
    async fn set_home_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        description: Option<String>,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_home_location(self, character_id, location_id, description)
            .await
    }

    async fn remove_home_location(&self, character_id: CharacterId) -> Result<()> {
        Neo4jCharacterRepository::remove_home_location(self, character_id).await
    }

    async fn set_work_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        role: String,
        schedule: Option<String>,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_work_location(self, character_id, location_id, role, schedule)
            .await
    }

    async fn remove_work_location(&self, character_id: CharacterId) -> Result<()> {
        Neo4jCharacterRepository::remove_work_location(self, character_id).await
    }

    async fn add_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        frequency: FrequencyLevel,
        time_of_day: String,
        day_of_week: Option<String>,
        reason: Option<String>,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_frequented_location(
            self,
            character_id,
            location_id,
            frequency,
            time_of_day,
            day_of_week,
            reason,
        )
        .await
    }

    async fn remove_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_frequented_location(self, character_id, location_id).await
    }

    async fn add_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        reason: String,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_avoided_location(self, character_id, location_id, reason)
            .await
    }

    async fn remove_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_avoided_location(self, character_id, location_id).await
    }

    async fn get_npcs_at_location(
        &self,
        location_id: LocationId,
        time_of_day: Option<&str>,
    ) -> Result<Vec<Character>> {
        Neo4jCharacterRepository::get_npcs_at_location(self, location_id, time_of_day).await
    }
}
