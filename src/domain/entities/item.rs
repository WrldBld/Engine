//! Item entity - Objects that can be possessed or interacted with
//!
//! # Graph-First Design (Phase 0.C)
//!
//! Items exist as nodes in the world. Possession is modeled as an edge:
//!
//! ```cypher
//! (world:World)-[:CONTAINS_ITEM]->(item:Item)
//! (character:Character)-[:POSSESSES {quantity: 1, equipped: true}]->(item:Item)
//! ```

use chrono::{DateTime, Utc};

use crate::domain::value_objects::{ItemId, WorldId};

/// An object that can be possessed or interacted with
#[derive(Debug, Clone)]
pub struct Item {
    pub id: ItemId,
    pub world_id: WorldId,
    pub name: String,
    pub description: Option<String>,
    /// Type of item (e.g., "Weapon", "Consumable", "Key", "Quest")
    pub item_type: Option<String>,
    /// Whether only one of this item can exist
    pub is_unique: bool,
    /// Item-specific properties (JSON - acceptable per ADR)
    pub properties: Option<String>,
}

impl Item {
    pub fn new(world_id: WorldId, name: impl Into<String>) -> Self {
        Self {
            id: ItemId::new(),
            world_id,
            name: name.into(),
            description: None,
            item_type: None,
            is_unique: false,
            properties: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_type(mut self, item_type: impl Into<String>) -> Self {
        self.item_type = Some(item_type.into());
        self
    }

    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    pub fn with_properties(mut self, properties: impl Into<String>) -> Self {
        self.properties = Some(properties.into());
        self
    }
}

/// Data for the POSSESSES edge between Character/PlayerCharacter and Item
#[derive(Debug, Clone)]
pub struct InventoryItem {
    /// The item being possessed
    pub item: Item,
    /// How many of this item the character has
    pub quantity: u32,
    /// Whether the item is currently equipped/held
    pub equipped: bool,
    /// When the item was acquired
    pub acquired_at: DateTime<Utc>,
    /// How the item was acquired
    pub acquisition_method: Option<AcquisitionMethod>,
}

impl InventoryItem {
    pub fn new(item: Item, quantity: u32) -> Self {
        Self {
            item,
            quantity,
            equipped: false,
            acquired_at: Utc::now(),
            acquisition_method: None,
        }
    }

    pub fn equipped(mut self) -> Self {
        self.equipped = true;
        self
    }

    pub fn with_acquisition(mut self, method: AcquisitionMethod) -> Self {
        self.acquisition_method = Some(method);
        self
    }
}

/// How an item was acquired
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionMethod {
    Found,
    Purchased,
    Gifted,
    Looted,
    Crafted,
    Inherited,
}

impl std::fmt::Display for AcquisitionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Found => write!(f, "Found"),
            Self::Purchased => write!(f, "Purchased"),
            Self::Gifted => write!(f, "Gifted"),
            Self::Looted => write!(f, "Looted"),
            Self::Crafted => write!(f, "Crafted"),
            Self::Inherited => write!(f, "Inherited"),
        }
    }
}

impl std::str::FromStr for AcquisitionMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Found" => Ok(Self::Found),
            "Purchased" => Ok(Self::Purchased),
            "Gifted" => Ok(Self::Gifted),
            "Looted" => Ok(Self::Looted),
            "Crafted" => Ok(Self::Crafted),
            "Inherited" => Ok(Self::Inherited),
            _ => Err(()),
        }
    }
}

/// How often a character frequents a location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyLevel {
    Rarely,
    Sometimes,
    Often,
    Always,
}

impl std::fmt::Display for FrequencyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rarely => write!(f, "Rarely"),
            Self::Sometimes => write!(f, "Sometimes"),
            Self::Often => write!(f, "Often"),
            Self::Always => write!(f, "Always"),
        }
    }
}

impl std::str::FromStr for FrequencyLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Rarely" => Ok(Self::Rarely),
            "Sometimes" => Ok(Self::Sometimes),
            "Often" => Ok(Self::Often),
            "Always" => Ok(Self::Always),
            _ => Err(()),
        }
    }
}
