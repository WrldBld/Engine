use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::{InteractionTarget, InteractionTemplate, InteractionType};
use crate::domain::value_objects::{CharacterId, ItemId};

#[derive(Debug, Deserialize)]
pub struct CreateInteractionRequestDto {
    pub name: String,
    pub interaction_type: String,
    #[serde(default)]
    pub target_type: String,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub target_description: Option<String>,
    #[serde(default)]
    pub prompt_hints: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub order: u32,
}

#[derive(Debug, Serialize)]
pub struct InteractionResponseDto {
    pub id: String,
    pub scene_id: String,
    pub name: String,
    pub interaction_type: String,
    pub target: String,
    pub prompt_hints: String,
    pub allowed_tools: Vec<String>,
    pub is_available: bool,
    pub order: u32,
}

impl From<InteractionTemplate> for InteractionResponseDto {
    fn from(i: InteractionTemplate) -> Self {
        Self {
            id: i.id.to_string(),
            scene_id: i.scene_id.to_string(),
            name: i.name,
            interaction_type: format!("{:?}", i.interaction_type),
            target: format!("{:?}", i.target),
            prompt_hints: i.prompt_hints,
            allowed_tools: i.allowed_tools,
            is_available: i.is_available,
            order: i.order,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SetAvailabilityRequestDto {
    pub available: bool,
}

pub fn parse_interaction_type(s: &str) -> InteractionType {
    match s {
        "Dialogue" => InteractionType::Dialogue,
        "Examine" => InteractionType::Examine,
        "UseItem" => InteractionType::UseItem,
        "PickUp" => InteractionType::PickUp,
        "GiveItem" => InteractionType::GiveItem,
        "Attack" => InteractionType::Attack,
        "Travel" => InteractionType::Travel,
        other => InteractionType::Custom(other.to_string()),
    }
}

pub fn parse_target(
    target_type: &str,
    target_id: Option<&str>,
    description: Option<&str>,
) -> Result<InteractionTarget, String> {
    match target_type {
        "Character" => {
            let id = target_id.ok_or_else(|| "Character target requires target_id".to_string())?;
            let uuid =
                Uuid::parse_str(id).map_err(|_| "Invalid character ID".to_string())?;
            Ok(InteractionTarget::Character(CharacterId::from_uuid(uuid)))
        }
        "Item" => {
            let id = target_id.ok_or_else(|| "Item target requires target_id".to_string())?;
            let uuid = Uuid::parse_str(id).map_err(|_| "Invalid item ID".to_string())?;
            Ok(InteractionTarget::Item(ItemId::from_uuid(uuid)))
        }
        "Environment" => {
            let desc = description
                .ok_or_else(|| "Environment target requires target_description".to_string())?;
            Ok(InteractionTarget::Environment(desc.to_string()))
        }
        "None" | "" => Ok(InteractionTarget::None),
        _ => Ok(InteractionTarget::None),
    }
}

