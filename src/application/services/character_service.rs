//! Character Service - Application service for character management
//!
//! This service provides use case implementations for creating, updating,
//! and managing characters, including archetype changes and wants.

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, info, instrument};

use crate::domain::entities::{Character, StatBlock};
use crate::domain::value_objects::{
    CampbellArchetype, CharacterId, Relationship, Want, WantId, WorldId,
};
use crate::infrastructure::persistence::Neo4jRepository;

/// Request to create a new character
#[derive(Debug, Clone)]
pub struct CreateCharacterRequest {
    pub world_id: WorldId,
    pub name: String,
    pub description: Option<String>,
    pub archetype: CampbellArchetype,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub stats: Option<StatBlock>,
    pub wants: Vec<Want>,
}

/// Request to update an existing character
#[derive(Debug, Clone)]
pub struct UpdateCharacterRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub stats: Option<StatBlock>,
    pub is_alive: Option<bool>,
    pub is_active: Option<bool>,
}

/// Request to change a character's archetype
#[derive(Debug, Clone)]
pub struct ChangeArchetypeRequest {
    pub new_archetype: CampbellArchetype,
    pub reason: String,
}

/// Character with relationship information
#[derive(Debug, Clone)]
pub struct CharacterWithRelationships {
    pub character: Character,
    pub relationships: Vec<Relationship>,
}

/// Character service trait defining the application use cases
#[async_trait]
pub trait CharacterService: Send + Sync {
    /// Create a new character with archetype
    async fn create_character(&self, request: CreateCharacterRequest) -> Result<Character>;

    /// Get a character by ID
    async fn get_character(&self, id: CharacterId) -> Result<Option<Character>>;

    /// Get a character with all their relationships
    async fn get_character_with_relationships(
        &self,
        id: CharacterId,
    ) -> Result<Option<CharacterWithRelationships>>;

    /// List all characters in a world
    async fn list_characters(&self, world_id: WorldId) -> Result<Vec<Character>>;

    /// List active characters in a world
    async fn list_active_characters(&self, world_id: WorldId) -> Result<Vec<Character>>;

    /// Update a character
    async fn update_character(
        &self,
        id: CharacterId,
        request: UpdateCharacterRequest,
    ) -> Result<Character>;

    /// Delete a character
    async fn delete_character(&self, id: CharacterId) -> Result<()>;

    /// Change a character's archetype with history tracking
    async fn change_archetype(
        &self,
        id: CharacterId,
        request: ChangeArchetypeRequest,
    ) -> Result<Character>;

    /// Temporarily assume a different archetype (for a scene)
    async fn assume_archetype(
        &self,
        id: CharacterId,
        archetype: CampbellArchetype,
    ) -> Result<Character>;

    /// Revert character to their base archetype
    async fn revert_to_base_archetype(&self, id: CharacterId) -> Result<Character>;

    /// Add a want to a character
    async fn add_want(&self, id: CharacterId, want: Want) -> Result<Character>;

    /// Remove a want from a character
    async fn remove_want(&self, id: CharacterId, want_id: WantId) -> Result<Character>;

    /// Update wants for a character
    async fn update_wants(&self, id: CharacterId, wants: Vec<Want>) -> Result<Character>;

    /// Set character as dead
    async fn kill_character(&self, id: CharacterId) -> Result<Character>;

    /// Resurrect a dead character
    async fn resurrect_character(&self, id: CharacterId) -> Result<Character>;

    /// Activate or deactivate a character
    async fn set_active(&self, id: CharacterId, active: bool) -> Result<Character>;
}

/// Default implementation of CharacterService using Neo4j repository
pub struct CharacterServiceImpl {
    repository: Neo4jRepository,
}

impl CharacterServiceImpl {
    /// Create a new CharacterServiceImpl with the given repository
    pub fn new(repository: Neo4jRepository) -> Self {
        Self { repository }
    }

    /// Validate a character creation request
    fn validate_create_request(request: &CreateCharacterRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            anyhow::bail!("Character name cannot be empty");
        }
        if request.name.len() > 255 {
            anyhow::bail!("Character name cannot exceed 255 characters");
        }
        if let Some(ref description) = request.description {
            if description.len() > 10000 {
                anyhow::bail!("Character description cannot exceed 10000 characters");
            }
        }
        Ok(())
    }

    /// Validate a character update request
    fn validate_update_request(request: &UpdateCharacterRequest) -> Result<()> {
        if let Some(ref name) = request.name {
            if name.trim().is_empty() {
                anyhow::bail!("Character name cannot be empty");
            }
            if name.len() > 255 {
                anyhow::bail!("Character name cannot exceed 255 characters");
            }
        }
        if let Some(ref description) = request.description {
            if description.len() > 10000 {
                anyhow::bail!("Character description cannot exceed 10000 characters");
            }
        }
        Ok(())
    }
}

#[async_trait]
impl CharacterService for CharacterServiceImpl {
    #[instrument(skip(self), fields(world_id = %request.world_id, name = %request.name))]
    async fn create_character(&self, request: CreateCharacterRequest) -> Result<Character> {
        Self::validate_create_request(&request)?;

        // Verify the world exists
        let _ = self
            .repository
            .worlds()
            .get(request.world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", request.world_id))?;

        let mut character = Character::new(request.world_id, &request.name, request.archetype);

        if let Some(description) = request.description {
            character = character.with_description(description);
        }
        if let Some(sprite) = request.sprite_asset {
            character = character.with_sprite(sprite);
        }
        if let Some(portrait) = request.portrait_asset {
            character = character.with_portrait(portrait);
        }
        if let Some(stats) = request.stats {
            character.stats = stats;
        }
        for want in request.wants {
            character = character.with_want(want);
        }

        self.repository
            .characters()
            .create(&character)
            .await
            .context("Failed to create character in repository")?;

        info!(
            character_id = %character.id,
            archetype = %character.current_archetype,
            "Created character: {} in world {}",
            character.name,
            request.world_id
        );
        Ok(character)
    }

    #[instrument(skip(self))]
    async fn get_character(&self, id: CharacterId) -> Result<Option<Character>> {
        debug!(character_id = %id, "Fetching character");
        self.repository
            .characters()
            .get(id)
            .await
            .context("Failed to get character from repository")
    }

    #[instrument(skip(self))]
    async fn get_character_with_relationships(
        &self,
        id: CharacterId,
    ) -> Result<Option<CharacterWithRelationships>> {
        debug!(character_id = %id, "Fetching character with relationships");

        let character = match self.repository.characters().get(id).await? {
            Some(c) => c,
            None => return Ok(None),
        };

        let relationships = self
            .repository
            .relationships()
            .get_involving_character(id)
            .await
            .context("Failed to get relationships for character")?;

        Ok(Some(CharacterWithRelationships {
            character,
            relationships,
        }))
    }

    #[instrument(skip(self))]
    async fn list_characters(&self, world_id: WorldId) -> Result<Vec<Character>> {
        debug!(world_id = %world_id, "Listing characters in world");
        self.repository
            .characters()
            .list_by_world(world_id)
            .await
            .context("Failed to list characters from repository")
    }

    #[instrument(skip(self))]
    async fn list_active_characters(&self, world_id: WorldId) -> Result<Vec<Character>> {
        debug!(world_id = %world_id, "Listing active characters in world");
        let characters = self
            .repository
            .characters()
            .list_by_world(world_id)
            .await
            .context("Failed to list characters from repository")?;

        Ok(characters.into_iter().filter(|c| c.is_active).collect())
    }

    #[instrument(skip(self), fields(character_id = %id))]
    async fn update_character(
        &self,
        id: CharacterId,
        request: UpdateCharacterRequest,
    ) -> Result<Character> {
        Self::validate_update_request(&request)?;

        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        if let Some(name) = request.name {
            character.name = name;
        }
        if let Some(description) = request.description {
            character.description = description;
        }
        if request.sprite_asset.is_some() {
            character.sprite_asset = request.sprite_asset;
        }
        if request.portrait_asset.is_some() {
            character.portrait_asset = request.portrait_asset;
        }
        if let Some(stats) = request.stats {
            character.stats = stats;
        }
        if let Some(is_alive) = request.is_alive {
            character.is_alive = is_alive;
        }
        if let Some(is_active) = request.is_active {
            character.is_active = is_active;
        }

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to update character in repository")?;

        info!(character_id = %id, "Updated character: {}", character.name);
        Ok(character)
    }

    #[instrument(skip(self))]
    async fn delete_character(&self, id: CharacterId) -> Result<()> {
        let character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        self.repository
            .characters()
            .delete(id)
            .await
            .context("Failed to delete character from repository")?;

        info!(character_id = %id, "Deleted character: {}", character.name);
        Ok(())
    }

    #[instrument(skip(self), fields(character_id = %id, new_archetype = %request.new_archetype))]
    async fn change_archetype(
        &self,
        id: CharacterId,
        request: ChangeArchetypeRequest,
    ) -> Result<Character> {
        if request.reason.trim().is_empty() {
            anyhow::bail!("Archetype change reason cannot be empty");
        }

        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        let old_archetype = character.current_archetype;
        character.change_archetype(request.new_archetype, &request.reason);

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to update character archetype in repository")?;

        info!(
            character_id = %id,
            from = %old_archetype,
            to = %request.new_archetype,
            reason = %request.reason,
            "Changed archetype for character: {}",
            character.name
        );
        Ok(character)
    }

    #[instrument(skip(self), fields(character_id = %id, archetype = %archetype))]
    async fn assume_archetype(
        &self,
        id: CharacterId,
        archetype: CampbellArchetype,
    ) -> Result<Character> {
        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        character.assume_archetype(archetype);

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to update character temporary archetype")?;

        debug!(
            character_id = %id,
            archetype = %archetype,
            "Character {} assuming temporary archetype",
            character.name
        );
        Ok(character)
    }

    #[instrument(skip(self))]
    async fn revert_to_base_archetype(&self, id: CharacterId) -> Result<Character> {
        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        character.revert_to_base();

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to revert character to base archetype")?;

        debug!(
            character_id = %id,
            base_archetype = %character.base_archetype,
            "Character {} reverted to base archetype",
            character.name
        );
        Ok(character)
    }

    #[instrument(skip(self, want), fields(character_id = %id))]
    async fn add_want(&self, id: CharacterId, want: Want) -> Result<Character> {
        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        character.wants.push(want.clone());

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to add want to character")?;

        debug!(
            character_id = %id,
            want_id = %want.id,
            "Added want to character: {}",
            character.name
        );
        Ok(character)
    }

    #[instrument(skip(self), fields(character_id = %id, want_id = %want_id))]
    async fn remove_want(&self, id: CharacterId, want_id: WantId) -> Result<Character> {
        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        if let Some(pos) = character.wants.iter().position(|w| w.id == want_id) {
            character.wants.remove(pos);

            self.repository
                .characters()
                .update(&character)
                .await
                .context("Failed to remove want from character")?;

            debug!(
                character_id = %id,
                want_id = %want_id,
                "Removed want from character: {}",
                character.name
            );
        }

        Ok(character)
    }

    #[instrument(skip(self, wants), fields(character_id = %id, want_count = wants.len()))]
    async fn update_wants(&self, id: CharacterId, wants: Vec<Want>) -> Result<Character> {
        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        character.wants = wants;

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to update character wants")?;

        info!(character_id = %id, "Updated wants for character: {}", character.name);
        Ok(character)
    }

    #[instrument(skip(self))]
    async fn kill_character(&self, id: CharacterId) -> Result<Character> {
        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        if !character.is_alive {
            anyhow::bail!("Character {} is already dead", character.name);
        }

        character.is_alive = false;

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to update character death status")?;

        info!(character_id = %id, "Character died: {}", character.name);
        Ok(character)
    }

    #[instrument(skip(self))]
    async fn resurrect_character(&self, id: CharacterId) -> Result<Character> {
        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        if character.is_alive {
            anyhow::bail!("Character {} is already alive", character.name);
        }

        character.is_alive = true;

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to update character resurrection status")?;

        info!(character_id = %id, "Character resurrected: {}", character.name);
        Ok(character)
    }

    #[instrument(skip(self), fields(character_id = %id, active = active))]
    async fn set_active(&self, id: CharacterId, active: bool) -> Result<Character> {
        let mut character = self
            .repository
            .characters()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

        character.is_active = active;

        self.repository
            .characters()
            .update(&character)
            .await
            .context("Failed to update character active status")?;

        debug!(
            character_id = %id,
            active = active,
            "Set active status for character: {}",
            character.name
        );
        Ok(character)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_character_request_validation() {
        // Empty name should fail
        let request = CreateCharacterRequest {
            world_id: WorldId::new(),
            name: "".to_string(),
            description: None,
            archetype: CampbellArchetype::Ally,
            sprite_asset: None,
            portrait_asset: None,
            stats: None,
            wants: vec![],
        };
        assert!(CharacterServiceImpl::validate_create_request(&request).is_err());

        // Valid request should pass
        let request = CreateCharacterRequest {
            world_id: WorldId::new(),
            name: "Gandalf".to_string(),
            description: Some("A wise wizard".to_string()),
            archetype: CampbellArchetype::Mentor,
            sprite_asset: None,
            portrait_asset: None,
            stats: None,
            wants: vec![],
        };
        assert!(CharacterServiceImpl::validate_create_request(&request).is_ok());
    }

    #[test]
    fn test_update_character_request_validation() {
        // Empty name should fail
        let request = UpdateCharacterRequest {
            name: Some("".to_string()),
            description: None,
            sprite_asset: None,
            portrait_asset: None,
            stats: None,
            is_alive: None,
            is_active: None,
        };
        assert!(CharacterServiceImpl::validate_update_request(&request).is_err());

        // No updates is valid
        let request = UpdateCharacterRequest {
            name: None,
            description: None,
            sprite_asset: None,
            portrait_asset: None,
            stats: None,
            is_alive: None,
            is_active: None,
        };
        assert!(CharacterServiceImpl::validate_update_request(&request).is_ok());
    }
}
