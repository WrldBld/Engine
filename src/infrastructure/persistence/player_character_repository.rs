//! Player Character repository implementation for Neo4j

use anyhow::{Context, Result};
use async_trait::async_trait;
use neo4rs::{query, Row};
use serde_json;

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::PlayerCharacterRepositoryPort;
use neo4rs::Node;
use crate::domain::entities::PlayerCharacter;
use crate::domain::entities::CharacterSheetData;
use crate::domain::value_objects::{
    LocationId, PlayerCharacterId, RegionId, SessionId,
};

/// Repository for PlayerCharacter operations
pub struct Neo4jPlayerCharacterRepository {
    connection: Neo4jConnection,
}

impl Neo4jPlayerCharacterRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl PlayerCharacterRepositoryPort for Neo4jPlayerCharacterRepository {
    async fn create(&self, pc: &PlayerCharacter) -> Result<()> {
        let sheet_data_json = if let Some(ref sheet) = pc.sheet_data {
            serde_json::to_string(sheet)?
        } else {
            "{}".to_string()
        };

        let session_id_str = pc.session_id.map(|s| s.to_string()).unwrap_or_default();
        let current_region_id_str = pc.current_region_id.map(|r| r.to_string()).unwrap_or_default();

        // Different query depending on whether we have a session
        if let Some(session_id) = pc.session_id {
            let q = query(
                "MATCH (s:Session {id: $session_id})
                MATCH (w:World {id: $world_id})
                MATCH (l:Location {id: $location_id})
                CREATE (pc:PlayerCharacter {
                    id: $id,
                    session_id: $session_id,
                    user_id: $user_id,
                    world_id: $world_id,
                    name: $name,
                    description: $description,
                    sheet_data: $sheet_data,
                    current_location_id: $current_location_id,
                    current_region_id: $current_region_id,
                    starting_location_id: $starting_location_id,
                    sprite_asset: $sprite_asset,
                    portrait_asset: $portrait_asset,
                    created_at: $created_at,
                    last_active_at: $last_active_at
                })
                CREATE (s)-[:HAS_PC]->(pc)
                CREATE (pc)-[:IN_WORLD]->(w)
                CREATE (pc)-[:AT_LOCATION]->(l)
                CREATE (pc)-[:STARTED_AT]->(l)
                RETURN pc.id as id",
            )
            .param("id", pc.id.to_string())
            .param("session_id", session_id.to_string())
            .param("user_id", pc.user_id.clone())
            .param("world_id", pc.world_id.to_string())
            .param("name", pc.name.clone())
            .param("description", pc.description.clone().unwrap_or_default())
            .param("sheet_data", sheet_data_json)
            .param("current_location_id", pc.current_location_id.to_string())
            .param("current_region_id", current_region_id_str)
            .param("starting_location_id", pc.starting_location_id.to_string())
            .param("sprite_asset", pc.sprite_asset.clone().unwrap_or_default())
            .param("portrait_asset", pc.portrait_asset.clone().unwrap_or_default())
            .param("created_at", pc.created_at.to_rfc3339())
            .param("last_active_at", pc.last_active_at.to_rfc3339());

            self.connection.graph().run(q).await?;
        } else {
            // Standalone PC (no session)
            let q = query(
                "MATCH (w:World {id: $world_id})
                MATCH (l:Location {id: $location_id})
                CREATE (pc:PlayerCharacter {
                    id: $id,
                    session_id: '',
                    user_id: $user_id,
                    world_id: $world_id,
                    name: $name,
                    description: $description,
                    sheet_data: $sheet_data,
                    current_location_id: $current_location_id,
                    current_region_id: $current_region_id,
                    starting_location_id: $starting_location_id,
                    sprite_asset: $sprite_asset,
                    portrait_asset: $portrait_asset,
                    created_at: $created_at,
                    last_active_at: $last_active_at
                })
                CREATE (pc)-[:IN_WORLD]->(w)
                CREATE (pc)-[:AT_LOCATION]->(l)
                CREATE (pc)-[:STARTED_AT]->(l)
                RETURN pc.id as id",
            )
            .param("id", pc.id.to_string())
            .param("user_id", pc.user_id.clone())
            .param("world_id", pc.world_id.to_string())
            .param("name", pc.name.clone())
            .param("description", pc.description.clone().unwrap_or_default())
            .param("sheet_data", sheet_data_json)
            .param("current_location_id", pc.current_location_id.to_string())
            .param("current_region_id", current_region_id_str)
            .param("starting_location_id", pc.starting_location_id.to_string())
            .param("sprite_asset", pc.sprite_asset.clone().unwrap_or_default())
            .param("portrait_asset", pc.portrait_asset.clone().unwrap_or_default())
            .param("created_at", pc.created_at.to_rfc3339())
            .param("last_active_at", pc.last_active_at.to_rfc3339());

            self.connection.graph().run(q).await?;
        }

        tracing::debug!("Created player character: {}", pc.name);
        Ok(())
    }

    async fn get(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            RETURN pc",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            Ok(Some(parse_player_character_row(row)?))
        } else {
            Ok(None)
        }
    }

    async fn get_by_session(&self, session_id: SessionId) -> Result<Vec<PlayerCharacter>> {
        let q = query(
            "MATCH (s:Session {id: $session_id})-[:HAS_PC]->(pc:PlayerCharacter)
            RETURN pc
            ORDER BY pc.created_at",
        )
        .param("session_id", session_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut pcs = Vec::new();
        while let Some(row) = result.next().await? {
            pcs.push(parse_player_character_row(row)?);
        }
        Ok(pcs)
    }

    async fn get_by_user_and_session(
        &self,
        user_id: &str,
        session_id: SessionId,
    ) -> Result<Option<PlayerCharacter>> {
        let q = query(
            "MATCH (s:Session {id: $session_id})-[:HAS_PC]->(pc:PlayerCharacter {user_id: $user_id})
            RETURN pc",
        )
        .param("session_id", session_id.to_string())
        .param("user_id", user_id);

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            Ok(Some(parse_player_character_row(row)?))
        } else {
            Ok(None)
        }
    }

    async fn get_by_location(&self, location_id: LocationId) -> Result<Vec<PlayerCharacter>> {
        let q = query(
            "MATCH (pc:PlayerCharacter)-[:AT_LOCATION]->(l:Location {id: $location_id})
            RETURN pc
            ORDER BY pc.last_active_at DESC",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut pcs = Vec::new();
        while let Some(row) = result.next().await? {
            pcs.push(parse_player_character_row(row)?);
        }
        Ok(pcs)
    }

    async fn update(&self, pc: &PlayerCharacter) -> Result<()> {
        let sheet_data_json = if let Some(ref sheet) = pc.sheet_data {
            serde_json::to_string(sheet)?
        } else {
            "{}".to_string()
        };

        let q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            SET pc.name = $name,
                pc.description = $description,
                pc.sheet_data = $sheet_data,
                pc.sprite_asset = $sprite_asset,
                pc.portrait_asset = $portrait_asset,
                pc.last_active_at = $last_active_at",
        )
        .param("id", pc.id.to_string())
        .param("name", pc.name.clone())
        .param("description", pc.description.clone().unwrap_or_default())
        .param("sheet_data", sheet_data_json)
        .param("sprite_asset", pc.sprite_asset.clone().unwrap_or_default())
        .param("portrait_asset", pc.portrait_asset.clone().unwrap_or_default())
        .param("last_active_at", pc.last_active_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated player character: {}", pc.name);
        Ok(())
    }

    async fn update_location(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        // Delete old AT_LOCATION relationship
        let delete_q = query(
            "MATCH (pc:PlayerCharacter {id: $id})-[r:AT_LOCATION]->()
            DELETE r",
        )
        .param("id", id.to_string());

        self.connection.graph().run(delete_q).await?;

        // Create new AT_LOCATION relationship and clear region
        let create_q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            MATCH (l:Location {id: $location_id})
            CREATE (pc)-[:AT_LOCATION]->(l)
            SET pc.current_location_id = $location_id,
                pc.current_region_id = '',
                pc.last_active_at = $last_active_at",
        )
        .param("id", id.to_string())
        .param("location_id", location_id.to_string())
        .param("last_active_at", chrono::Utc::now().to_rfc3339());

        self.connection.graph().run(create_q).await?;
        tracing::debug!("Updated player character location: {} -> {}", id, location_id);
        Ok(())
    }

    async fn update_region(
        &self,
        id: PlayerCharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            SET pc.current_region_id = $region_id,
                pc.last_active_at = $last_active_at",
        )
        .param("id", id.to_string())
        .param("region_id", region_id.to_string())
        .param("last_active_at", chrono::Utc::now().to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated player character region: {} -> {}", id, region_id);
        Ok(())
    }

    async fn update_position(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
        region_id: Option<RegionId>,
    ) -> Result<()> {
        // Delete old AT_LOCATION relationship
        let delete_q = query(
            "MATCH (pc:PlayerCharacter {id: $id})-[r:AT_LOCATION]->()
            DELETE r",
        )
        .param("id", id.to_string());

        self.connection.graph().run(delete_q).await?;

        let region_id_str = region_id.map(|r| r.to_string()).unwrap_or_default();

        // Create new AT_LOCATION relationship with region
        let create_q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            MATCH (l:Location {id: $location_id})
            CREATE (pc)-[:AT_LOCATION]->(l)
            SET pc.current_location_id = $location_id,
                pc.current_region_id = $region_id,
                pc.last_active_at = $last_active_at",
        )
        .param("id", id.to_string())
        .param("location_id", location_id.to_string())
        .param("region_id", region_id_str)
        .param("last_active_at", chrono::Utc::now().to_rfc3339());

        self.connection.graph().run(create_q).await?;
        tracing::debug!("Updated player character position: {} -> {:?}", id, (location_id, region_id));
        Ok(())
    }

    async fn bind_to_session(
        &self,
        id: PlayerCharacterId,
        session_id: SessionId,
    ) -> Result<()> {
        // First check if already bound to a session
        let delete_q = query(
            "MATCH (pc:PlayerCharacter {id: $id})<-[r:HAS_PC]-(s:Session)
            DELETE r",
        )
        .param("id", id.to_string());

        self.connection.graph().run(delete_q).await?;

        // Create new session binding
        let create_q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            MATCH (s:Session {id: $session_id})
            CREATE (s)-[:HAS_PC]->(pc)
            SET pc.session_id = $session_id,
                pc.last_active_at = $last_active_at",
        )
        .param("id", id.to_string())
        .param("session_id", session_id.to_string())
        .param("last_active_at", chrono::Utc::now().to_rfc3339());

        self.connection.graph().run(create_q).await?;
        tracing::debug!("Bound player character {} to session {}", id, session_id);
        Ok(())
    }

    async fn unbind_from_session(&self, id: PlayerCharacterId) -> Result<()> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $id})<-[r:HAS_PC]-(s:Session)
            DELETE r
            SET pc.session_id = ''",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Unbound player character {} from session", id);
        Ok(())
    }

    async fn get_by_user_and_world(
        &self,
        user_id: &str,
        world_id: crate::domain::value_objects::WorldId,
    ) -> Result<Vec<PlayerCharacter>> {
        let q = query(
            "MATCH (pc:PlayerCharacter {user_id: $user_id})-[:IN_WORLD]->(w:World {id: $world_id})
            RETURN pc
            ORDER BY pc.last_active_at DESC",
        )
        .param("user_id", user_id)
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut pcs = Vec::new();
        while let Some(row) = result.next().await? {
            pcs.push(parse_player_character_row(row)?);
        }
        Ok(pcs)
    }

    async fn get_unbound_by_user(&self, user_id: &str) -> Result<Vec<PlayerCharacter>> {
        let q = query(
            "MATCH (pc:PlayerCharacter {user_id: $user_id})
            WHERE pc.session_id = '' OR pc.session_id IS NULL
            RETURN pc
            ORDER BY pc.last_active_at DESC",
        )
        .param("user_id", user_id);

        let mut result = self.connection.graph().execute(q).await?;
        let mut pcs = Vec::new();
        while let Some(row) = result.next().await? {
            pcs.push(parse_player_character_row(row)?);
        }
        Ok(pcs)
    }

    async fn delete(&self, id: PlayerCharacterId) -> Result<()> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            DETACH DELETE pc",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted player character: {}", id);
        Ok(())
    }
}

/// Parse a PlayerCharacter from a Neo4j row
fn parse_player_character_row(row: Row) -> Result<PlayerCharacter> {
    use crate::domain::value_objects::{LocationId, PlayerCharacterId, RegionId, SessionId, WorldId};
    use chrono::DateTime;
    

    let node = row.get::<Node>("pc")
        .context("Expected 'pc' node in row")?;

    let id_str: String = node.get("id").context("Missing id")?;
    let id = PlayerCharacterId::from_uuid(
        uuid::Uuid::parse_str(&id_str)
            .context("Invalid UUID for player character id")?,
    );

    // session_id is optional - empty string means not bound
    let session_id_str: String = node.get("session_id").unwrap_or_default();
    let session_id = if session_id_str.is_empty() {
        None
    } else {
        Some(SessionId::from_uuid(
            uuid::Uuid::parse_str(&session_id_str)
                .context("Invalid UUID for session_id")?,
        ))
    };

    let user_id: String = node.get("user_id").context("Missing user_id")?;

    let world_id_str: String = node.get("world_id").context("Missing world_id")?;
    let world_id = WorldId::from_uuid(
        uuid::Uuid::parse_str(&world_id_str)
            .context("Invalid UUID for world_id")?,
    );

    let name: String = node.get("name").context("Missing name")?;
    let description: Option<String> = node.get("description").ok().flatten();
    let description = if description.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        None
    } else {
        description
    };

    let sheet_data_str: String = node.get("sheet_data").unwrap_or_default();
    let sheet_data = if sheet_data_str.is_empty() || sheet_data_str == "{}" {
        None
    } else {
        Some(
            serde_json::from_str::<CharacterSheetData>(&sheet_data_str)
                .context("Failed to parse sheet_data")?,
        )
    };

    let current_location_id_str: String = node.get("current_location_id").context("Missing current_location_id")?;
    let current_location_id = LocationId::from_uuid(
        uuid::Uuid::parse_str(&current_location_id_str)
            .context("Invalid UUID for current_location_id")?,
    );

    // current_region_id is optional
    let current_region_id_str: String = node.get("current_region_id").unwrap_or_default();
    let current_region_id = if current_region_id_str.is_empty() {
        None
    } else {
        Some(RegionId::from_uuid(
            uuid::Uuid::parse_str(&current_region_id_str)
                .context("Invalid UUID for current_region_id")?,
        ))
    };

    let starting_location_id_str: String = node.get("starting_location_id").context("Missing starting_location_id")?;
    let starting_location_id = LocationId::from_uuid(
        uuid::Uuid::parse_str(&starting_location_id_str)
            .context("Invalid UUID for starting_location_id")?,
    );

    let sprite_asset: Option<String> = node.get("sprite_asset").ok().flatten();
    let sprite_asset = if sprite_asset.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        None
    } else {
        sprite_asset
    };

    let portrait_asset: Option<String> = node.get("portrait_asset").ok().flatten();
    let portrait_asset = if portrait_asset.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        None
    } else {
        portrait_asset
    };

    let created_at_str: String = node.get("created_at").context("Missing created_at")?;
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .context("Invalid created_at timestamp")?
        .with_timezone(&chrono::Utc);

    let last_active_at_str: String = node.get("last_active_at").context("Missing last_active_at")?;
    let last_active_at = DateTime::parse_from_rfc3339(&last_active_at_str)
        .context("Invalid last_active_at timestamp")?
        .with_timezone(&chrono::Utc);

    Ok(PlayerCharacter {
        id,
        session_id,
        user_id,
        world_id,
        name,
        description,
        sheet_data,
        current_location_id,
        current_region_id,
        starting_location_id,
        sprite_asset,
        portrait_asset,
        created_at,
        last_active_at,
    })
}

