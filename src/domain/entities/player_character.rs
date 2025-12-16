//! Player Character entity - PCs created by players, distinct from NPCs

use chrono::{DateTime, Utc};
use crate::domain::value_objects::{
    LocationId, PlayerCharacterId, SessionId, WorldId,
};
use crate::domain::entities::sheet_template::CharacterSheetData;

/// A player character (PC) - distinct from NPCs
///
/// PCs are created by players when joining a session, have character sheets,
/// and track their current location for scene resolution.
#[derive(Debug, Clone)]
pub struct PlayerCharacter {
    pub id: PlayerCharacterId,
    pub session_id: SessionId,
    pub user_id: String,  // Anonymous user ID from Player
    pub world_id: WorldId,
    
    // Character identity
    pub name: String,
    pub description: Option<String>,
    
    // Character sheet data (matches CharacterSheetData from Phase 14)
    pub sheet_data: Option<CharacterSheetData>,
    
    // Location tracking
    pub current_location_id: LocationId,
    pub starting_location_id: LocationId,  // For reference/history
    
    // Visual assets (optional, can be generated later)
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    
    // Metadata
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

impl PlayerCharacter {
    /// Create a new player character
    pub fn new(
        session_id: SessionId,
        user_id: impl Into<String>,
        world_id: WorldId,
        name: impl Into<String>,
        starting_location_id: LocationId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: PlayerCharacterId::new(),
            session_id,
            user_id: user_id.into(),
            world_id,
            name: name.into(),
            description: None,
            sheet_data: None,
            current_location_id: starting_location_id,
            starting_location_id,
            sprite_asset: None,
            portrait_asset: None,
            created_at: now,
            last_active_at: now,
        }
    }

    /// Set the character description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the character sheet data
    pub fn with_sheet_data(mut self, sheet_data: CharacterSheetData) -> Self {
        self.sheet_data = Some(sheet_data);
        self
    }

    /// Set the sprite asset
    pub fn with_sprite(mut self, asset_path: impl Into<String>) -> Self {
        self.sprite_asset = Some(asset_path.into());
        self
    }

    /// Set the portrait asset
    pub fn with_portrait(mut self, asset_path: impl Into<String>) -> Self {
        self.portrait_asset = Some(asset_path.into());
        self
    }

    /// Update the character's current location
    pub fn update_location(&mut self, location_id: LocationId) {
        self.current_location_id = location_id;
        self.last_active_at = Utc::now();
    }

    /// Update the last active timestamp
    pub fn touch(&mut self) {
        self.last_active_at = Utc::now();
    }

    /// Validate that the character has required fields
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Character name cannot be empty".to_string());
        }
        Ok(())
    }
}

