//! Item and Inventory DTOs
//!
//! Data transfer objects for items and character inventory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::entities::{AcquisitionMethod, InventoryItem, Item};

/// Response DTO for an item
#[derive(Debug, Serialize)]
pub struct ItemResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: Option<String>,
    pub item_type: Option<String>,
    pub is_unique: bool,
    pub properties: Option<String>,
}

impl From<Item> for ItemResponseDto {
    fn from(item: Item) -> Self {
        Self {
            id: item.id.to_string(),
            world_id: item.world_id.to_string(),
            name: item.name,
            description: item.description,
            item_type: item.item_type,
            is_unique: item.is_unique,
            properties: item.properties,
        }
    }
}

impl From<&Item> for ItemResponseDto {
    fn from(item: &Item) -> Self {
        Self {
            id: item.id.to_string(),
            world_id: item.world_id.to_string(),
            name: item.name.clone(),
            description: item.description.clone(),
            item_type: item.item_type.clone(),
            is_unique: item.is_unique,
            properties: item.properties.clone(),
        }
    }
}

/// Response DTO for an inventory item (item + possession data)
#[derive(Debug, Serialize)]
pub struct InventoryItemResponseDto {
    /// The item details
    pub item: ItemResponseDto,
    /// Quantity possessed
    pub quantity: u32,
    /// Whether equipped/held
    pub equipped: bool,
    /// When acquired
    pub acquired_at: DateTime<Utc>,
    /// How acquired (if known)
    pub acquisition_method: Option<String>,
}

impl From<InventoryItem> for InventoryItemResponseDto {
    fn from(inv: InventoryItem) -> Self {
        Self {
            item: ItemResponseDto::from(inv.item),
            quantity: inv.quantity,
            equipped: inv.equipped,
            acquired_at: inv.acquired_at,
            acquisition_method: inv.acquisition_method.map(|m| m.to_string()),
        }
    }
}

/// Request DTO for creating an item
#[derive(Debug, Deserialize)]
pub struct CreateItemRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub item_type: Option<String>,
    #[serde(default)]
    pub is_unique: bool,
    #[serde(default)]
    pub properties: Option<String>,
}

/// Request DTO for adding an item to inventory
#[derive(Debug, Deserialize)]
pub struct AddInventoryItemRequestDto {
    pub item_id: String,
    #[serde(default = "default_quantity")]
    pub quantity: u32,
    #[serde(default)]
    pub equipped: bool,
    #[serde(default)]
    pub acquisition_method: Option<String>,
}

fn default_quantity() -> u32 {
    1
}

/// Request DTO for updating inventory item
#[derive(Debug, Deserialize)]
pub struct UpdateInventoryItemRequestDto {
    #[serde(default)]
    pub quantity: Option<u32>,
    #[serde(default)]
    pub equipped: Option<bool>,
}

/// Parse acquisition method from string
pub fn parse_acquisition_method(s: &str) -> Option<AcquisitionMethod> {
    s.parse().ok()
}
