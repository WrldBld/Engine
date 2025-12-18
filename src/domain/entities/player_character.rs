//! Player Character entity - PCs created by players, distinct from NPCs

use chrono::{DateTime, Utc};
use crate::domain::value_objects::{
    LocationId, PlayerCharacterId, RegionId, SessionId, WorldId,
};
use crate::domain::entities::sheet_template::CharacterSheetData;

/// A player character (PC) - distinct from NPCs
///
/// PCs are created by players when joining a session, have character sheets,
/// and track their current location/region for scene resolution.
///
/// # Session Binding
///
/// A PC can exist without a session (`session_id = None`), allowing:
/// - Creating a PC before joining a session
/// - Selecting an existing PC when joining a new session
/// - Importing a PC from another world
///
/// When `session_id` is `Some`, the PC is actively bound to that session.
#[derive(Debug, Clone)]
pub struct PlayerCharacter {
    pub id: PlayerCharacterId,
    /// The session this PC is bound to (None = standalone/selectable)
    pub session_id: Option<SessionId>,
    pub user_id: String,  // Anonymous user ID from Player
    pub world_id: WorldId,
    
    // Character identity
    pub name: String,
    pub description: Option<String>,
    
    // Character sheet data (matches CharacterSheetData from Phase 14)
    pub sheet_data: Option<CharacterSheetData>,
    
    // Location tracking
    pub current_location_id: LocationId,
    /// The specific region within the location (for JRPG-style navigation)
    pub current_region_id: Option<RegionId>,
    pub starting_location_id: LocationId,  // For reference/history
    
    // Visual assets (optional, can be generated later)
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    
    // Metadata
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

impl PlayerCharacter {
    /// Create a new player character (standalone, not bound to a session)
    pub fn new(
        user_id: impl Into<String>,
        world_id: WorldId,
        name: impl Into<String>,
        starting_location_id: LocationId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: PlayerCharacterId::new(),
            session_id: None,
            user_id: user_id.into(),
            world_id,
            name: name.into(),
            description: None,
            sheet_data: None,
            current_location_id: starting_location_id,
            current_region_id: None,
            starting_location_id,
            sprite_asset: None,
            portrait_asset: None,
            created_at: now,
            last_active_at: now,
        }
    }

    /// Create a new player character bound to a session
    pub fn new_in_session(
        session_id: SessionId,
        user_id: impl Into<String>,
        world_id: WorldId,
        name: impl Into<String>,
        starting_location_id: LocationId,
    ) -> Self {
        let mut pc = Self::new(user_id, world_id, name, starting_location_id);
        pc.session_id = Some(session_id);
        pc
    }

    /// Bind this character to a session
    pub fn bind_to_session(&mut self, session_id: SessionId) {
        self.session_id = Some(session_id);
        self.last_active_at = Utc::now();
    }

    /// Unbind this character from its session (make it standalone)
    pub fn unbind_from_session(&mut self) {
        self.session_id = None;
    }

    /// Check if this character is bound to a session
    pub fn is_bound_to_session(&self) -> bool {
        self.session_id.is_some()
    }

    /// Set the starting region (spawn point)
    pub fn with_starting_region(mut self, region_id: RegionId) -> Self {
        self.current_region_id = Some(region_id);
        self
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

    /// Update the character's current location (clears region)
    pub fn update_location(&mut self, location_id: LocationId) {
        self.current_location_id = location_id;
        self.current_region_id = None;  // Region needs to be set separately
        self.last_active_at = Utc::now();
    }

    /// Update the character's current region (within current location)
    pub fn update_region(&mut self, region_id: RegionId) {
        self.current_region_id = Some(region_id);
        self.last_active_at = Utc::now();
    }

    /// Update both location and region at once
    pub fn update_position(&mut self, location_id: LocationId, region_id: Option<RegionId>) {
        self.current_location_id = location_id;
        self.current_region_id = region_id;
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

