//! Player Character Service - Application service for player character management
//!
//! This service provides use case implementations for creating, updating,
//! and managing player characters (PCs) in sessions.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::{
    LocationRepositoryPort, PlayerCharacterRepositoryPort, WorldRepositoryPort,
};
use crate::domain::entities::PlayerCharacter;
use crate::domain::entities::CharacterSheetData;
use crate::domain::value_objects::{
    LocationId, PlayerCharacterId, SessionId, SkillId, WorldId,
};

/// Request to create a new player character
#[derive(Debug, Clone)]
pub struct CreatePlayerCharacterRequest {
    /// Session to bind the PC to (None = standalone/selectable PC)
    pub session_id: Option<SessionId>,
    pub user_id: String,
    pub world_id: WorldId,
    pub name: String,
    pub description: Option<String>,
    pub starting_location_id: LocationId,
    pub sheet_data: Option<CharacterSheetData>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Request to update an existing player character
#[derive(Debug, Clone)]
pub struct UpdatePlayerCharacterRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sheet_data: Option<CharacterSheetData>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Player character service trait defining the application use cases
#[async_trait]
pub trait PlayerCharacterService: Send + Sync {
    /// Create a new player character
    async fn create_pc(&self, request: CreatePlayerCharacterRequest) -> Result<PlayerCharacter>;

    /// Get a player character by ID
    async fn get_pc(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>>;

    /// Get a player character by user ID and session ID
    async fn get_pc_by_user_and_session(
        &self,
        user_id: &str,
        session_id: SessionId,
    ) -> Result<Option<PlayerCharacter>>;

    /// Get all player characters in a session
    async fn get_pcs_by_session(&self, session_id: SessionId) -> Result<Vec<PlayerCharacter>>;

    /// Update a player character
    async fn update_pc(
        &self,
        id: PlayerCharacterId,
        request: UpdatePlayerCharacterRequest,
    ) -> Result<PlayerCharacter>;

    /// Update a player character's location
    async fn update_pc_location(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Delete a player character
    async fn delete_pc(&self, id: PlayerCharacterId) -> Result<()>;

    /// Get a player character's modifier for a specific skill.
    /// Returns 0 if the PC doesn't have the skill or doesn't have sheet data.
    async fn get_skill_modifier(&self, id: PlayerCharacterId, skill_id: SkillId) -> Result<i32>;
}

/// Default implementation of PlayerCharacterService using port abstractions
#[derive(Clone)]
pub struct PlayerCharacterServiceImpl {
    pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
    location_repository: Arc<dyn LocationRepositoryPort>,
    world_repository: Arc<dyn WorldRepositoryPort>,
}

impl PlayerCharacterServiceImpl {
    /// Create a new PlayerCharacterServiceImpl with the given repositories
    pub fn new(
        pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
        location_repository: Arc<dyn LocationRepositoryPort>,
        world_repository: Arc<dyn WorldRepositoryPort>,
    ) -> Self {
        Self {
            pc_repository,
            location_repository,
            world_repository,
        }
    }

    /// Validate create request
    fn validate_create_request(request: &CreatePlayerCharacterRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Character name cannot be empty"));
        }
        Ok(())
    }
}

#[async_trait]
impl PlayerCharacterService for PlayerCharacterServiceImpl {
    #[instrument(skip(self), fields(user_id = %request.user_id, name = %request.name))]
    async fn create_pc(&self, request: CreatePlayerCharacterRequest) -> Result<PlayerCharacter> {
        Self::validate_create_request(&request)?;

        // Verify the world exists
        let _ = self
            .world_repository
            .get(request.world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", request.world_id))?;

        // Verify the location exists
        let _ = self
            .location_repository
            .get(request.starting_location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", request.starting_location_id))?;

        // Check if user already has a PC in this session (only if session is specified)
        if let Some(session_id) = request.session_id {
            if let Some(existing) = self
                .pc_repository
                .get_by_user_and_session(&request.user_id, session_id)
                .await?
            {
                return Err(anyhow::anyhow!(
                    "User {} already has a player character in session {}",
                    request.user_id,
                    session_id
                ));
            }
        }

        let mut pc = if let Some(session_id) = request.session_id {
            PlayerCharacter::new_in_session(
                session_id,
                request.user_id.clone(),
                request.world_id,
                request.name.clone(),
                request.starting_location_id,
            )
        } else {
            PlayerCharacter::new(
                request.user_id.clone(),
                request.world_id,
                request.name.clone(),
                request.starting_location_id,
            )
        };

        if let Some(description) = request.description {
            pc = pc.with_description(description);
        }
        if let Some(sheet_data) = request.sheet_data {
            pc = pc.with_sheet_data(sheet_data);
        }
        if let Some(sprite) = request.sprite_asset {
            pc = pc.with_sprite(sprite);
        }
        if let Some(portrait) = request.portrait_asset {
            pc = pc.with_portrait(portrait);
        }

        // Validate the PC
        pc.validate().map_err(|e| anyhow::anyhow!(e))?;

        self.pc_repository
            .create(&pc)
            .await
            .context("Failed to create player character in repository")?;

        if let Some(session_id) = request.session_id {
            info!(
                pc_id = %pc.id,
                user_id = %pc.user_id,
                "Created player character: {} in session {}",
                pc.name,
                session_id
            );
        } else {
            info!(
                pc_id = %pc.id,
                user_id = %pc.user_id,
                "Created standalone player character: {}",
                pc.name
            );
        }
        Ok(pc)
    }

    #[instrument(skip(self), fields(pc_id = %id))]
    async fn get_pc(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>> {
        debug!(pc_id = %id, "Fetching player character");
        self.pc_repository
            .get(id)
            .await
            .context("Failed to get player character from repository")
    }

    #[instrument(skip(self), fields(user_id = %user_id, session_id = %session_id))]
    async fn get_pc_by_user_and_session(
        &self,
        user_id: &str,
        session_id: SessionId,
    ) -> Result<Option<PlayerCharacter>> {
        debug!(user_id = %user_id, session_id = %session_id, "Fetching player character by user and session");
        self.pc_repository
            .get_by_user_and_session(user_id, session_id)
            .await
            .context("Failed to get player character from repository")
    }

    #[instrument(skip(self), fields(session_id = %session_id))]
    async fn get_pcs_by_session(&self, session_id: SessionId) -> Result<Vec<PlayerCharacter>> {
        debug!(session_id = %session_id, "Fetching player characters for session");
        self.pc_repository
            .get_by_session(session_id)
            .await
            .context("Failed to get player characters from repository")
    }

    #[instrument(skip(self), fields(pc_id = %id))]
    async fn update_pc(
        &self,
        id: PlayerCharacterId,
        request: UpdatePlayerCharacterRequest,
    ) -> Result<PlayerCharacter> {
        let mut pc = self
            .pc_repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", id))?;

        if let Some(name) = request.name {
            if name.trim().is_empty() {
                return Err(anyhow::anyhow!("Character name cannot be empty"));
            }
            pc.name = name;
        }
        if let Some(description) = request.description {
            pc.description = if description.trim().is_empty() {
                None
            } else {
                Some(description)
            };
        }
        if let Some(sheet_data) = request.sheet_data {
            pc.sheet_data = Some(sheet_data);
        }
        if let Some(sprite) = request.sprite_asset {
            pc.sprite_asset = if sprite.trim().is_empty() {
                None
            } else {
                Some(sprite)
            };
        }
        if let Some(portrait) = request.portrait_asset {
            pc.portrait_asset = if portrait.trim().is_empty() {
                None
            } else {
                Some(portrait)
            };
        }

        pc.touch(); // Update last_active_at
        pc.validate().map_err(|e| anyhow::anyhow!(e))?;

        self.pc_repository
            .update(&pc)
            .await
            .context("Failed to update player character in repository")?;

        info!(pc_id = %pc.id, "Updated player character: {}", pc.name);
        Ok(pc)
    }

    #[instrument(skip(self), fields(pc_id = %id, location_id = %location_id))]
    async fn update_pc_location(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        // Verify the location exists
        let _ = self
            .location_repository
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        // Verify the PC exists
        let _ = self
            .pc_repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", id))?;

        self.pc_repository
            .update_location(id, location_id)
            .await
            .context("Failed to update player character location in repository")?;

        info!(pc_id = %id, location_id = %location_id, "Updated player character location");
        Ok(())
    }

    #[instrument(skip(self), fields(pc_id = %id))]
    async fn delete_pc(&self, id: PlayerCharacterId) -> Result<()> {
        let pc = self
            .pc_repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", id))?;

        self.pc_repository
            .delete(id)
            .await
            .context("Failed to delete player character from repository")?;

        info!(pc_id = %id, "Deleted player character: {}", pc.name);
        Ok(())
    }

    #[instrument(skip(self), fields(pc_id = %id, skill_id = %skill_id))]
    async fn get_skill_modifier(&self, id: PlayerCharacterId, skill_id: SkillId) -> Result<i32> {
        let pc = self
            .pc_repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", id))?;

        // If the PC has sheet data, look up the skill modifier
        if let Some(sheet_data) = &pc.sheet_data {
            let skill_id_str = skill_id.to_string();
            if let Some(modifier) = sheet_data.get_skill_modifier(&skill_id_str) {
                debug!(pc_id = %id, skill_id = %skill_id, modifier = modifier, "Found skill modifier");
                return Ok(modifier);
            }
        }

        // Default to 0 if no sheet data or skill not found
        debug!(pc_id = %id, skill_id = %skill_id, "No skill modifier found, defaulting to 0");
        Ok(0)
    }
}

