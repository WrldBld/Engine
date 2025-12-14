//! Character entity - NPCs with Campbell archetypes and actantial wants

use crate::domain::value_objects::{
    ArchetypeChange, CampbellArchetype, CharacterId, ItemId, Want, WorldId,
};

/// A character (NPC) in the world
#[derive(Debug, Clone)]
pub struct Character {
    pub id: CharacterId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    /// Path to sprite image asset
    pub sprite_asset: Option<String>,
    /// Path to portrait image asset
    pub portrait_asset: Option<String>,

    // Campbell Archetype System (Layered)
    /// The character's base archetype
    pub base_archetype: CampbellArchetype,
    /// Current archetype (may differ from base)
    pub current_archetype: CampbellArchetype,
    /// History of archetype changes
    pub archetype_history: Vec<ArchetypeChange>,

    // Actantial Model
    /// The character's wants/desires
    pub wants: Vec<Want>,

    // Game Stats
    pub stats: StatBlock,
    pub inventory: Vec<ItemId>,

    // Character state
    pub is_alive: bool,
    pub is_active: bool,
}

impl Character {
    pub fn new(world_id: WorldId, name: impl Into<String>, archetype: CampbellArchetype) -> Self {
        Self {
            id: CharacterId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            sprite_asset: None,
            portrait_asset: None,
            base_archetype: archetype,
            current_archetype: archetype,
            archetype_history: Vec::new(),
            wants: Vec::new(),
            stats: StatBlock::default(),
            inventory: Vec::new(),
            is_alive: true,
            is_active: true,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_sprite(mut self, asset_path: impl Into<String>) -> Self {
        self.sprite_asset = Some(asset_path.into());
        self
    }

    pub fn with_portrait(mut self, asset_path: impl Into<String>) -> Self {
        self.portrait_asset = Some(asset_path.into());
        self
    }

    pub fn with_want(mut self, want: Want) -> Self {
        self.wants.push(want);
        self
    }

    /// Change the character's current archetype
    pub fn change_archetype(
        &mut self,
        new_archetype: CampbellArchetype,
        reason: impl Into<String>,
    ) {
        let change = ArchetypeChange {
            from: self.current_archetype,
            to: new_archetype,
            reason: reason.into(),
            timestamp: chrono::Utc::now(),
        };
        self.archetype_history.push(change);
        self.current_archetype = new_archetype;
    }

    /// Temporarily assume a different archetype (for a scene)
    pub fn assume_archetype(&mut self, archetype: CampbellArchetype) {
        // Only changes current, doesn't record in history (temporary)
        self.current_archetype = archetype;
    }

    /// Revert to base archetype
    pub fn revert_to_base(&mut self) {
        self.current_archetype = self.base_archetype;
    }

    pub fn add_item(&mut self, item_id: ItemId) {
        self.inventory.push(item_id);
    }

    pub fn remove_item(&mut self, item_id: &ItemId) -> bool {
        if let Some(pos) = self.inventory.iter().position(|id| id == item_id) {
            self.inventory.remove(pos);
            true
        } else {
            false
        }
    }
}

/// Character stats (system-agnostic)
#[derive(Debug, Clone, Default)]
pub struct StatBlock {
    /// Map of stat name to value
    pub stats: std::collections::HashMap<String, i32>,
    /// Current hit points
    pub current_hp: Option<i32>,
    /// Maximum hit points
    pub max_hp: Option<i32>,
}

impl StatBlock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_stat(mut self, name: impl Into<String>, value: i32) -> Self {
        self.stats.insert(name.into(), value);
        self
    }

    pub fn with_hp(mut self, current: i32, max: i32) -> Self {
        self.current_hp = Some(current);
        self.max_hp = Some(max);
        self
    }

    pub fn get_stat(&self, name: &str) -> Option<i32> {
        self.stats.get(name).copied()
    }

    pub fn set_stat(&mut self, name: impl Into<String>, value: i32) {
        self.stats.insert(name.into(), value);
    }
}
