use serde::{Deserialize, Serialize};

use crate::domain::entities::Character;
use crate::domain::value_objects::{CampbellArchetype, RelationshipType};

#[derive(Debug, Deserialize)]
pub struct CreateCharacterRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub archetype: String,
    #[serde(default)]
    pub sprite_asset: Option<String>,
    #[serde(default)]
    pub portrait_asset: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeArchetypeRequestDto {
    pub archetype: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct CharacterResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub base_archetype: String,
    pub current_archetype: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_alive: bool,
    pub is_active: bool,
}

impl From<Character> for CharacterResponseDto {
    fn from(c: Character) -> Self {
        Self {
            id: c.id.to_string(),
            world_id: c.world_id.to_string(),
            name: c.name,
            description: c.description,
            base_archetype: format!("{:?}", c.base_archetype),
            current_archetype: format!("{:?}", c.current_archetype),
            sprite_asset: c.sprite_asset,
            portrait_asset: c.portrait_asset,
            is_alive: c.is_alive,
            is_active: c.is_active,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRelationshipRequestDto {
    pub from_character_id: String,
    pub to_character_id: String,
    pub relationship_type: String,
    #[serde(default)]
    pub sentiment: f32,
    #[serde(default = "default_known")]
    pub known_to_player: bool,
}

fn default_known() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct CreatedIdResponseDto {
    pub id: String,
}

/// Parse a CampbellArchetype from a string label.
///
/// This helper is used from both application DTOs and persistence code to keep
/// archetype mapping consistent in a single place.
pub fn parse_archetype(s: &str) -> CampbellArchetype {
    match s {
        "Hero" => CampbellArchetype::Hero,
        "Mentor" => CampbellArchetype::Mentor,
        "ThresholdGuardian" => CampbellArchetype::ThresholdGuardian,
        "Herald" => CampbellArchetype::Herald,
        "Shapeshifter" => CampbellArchetype::Shapeshifter,
        "Shadow" => CampbellArchetype::Shadow,
        "Trickster" => CampbellArchetype::Trickster,
        "Ally" => CampbellArchetype::Ally,
        _ => CampbellArchetype::Ally,
    }
}

pub fn parse_relationship_type(s: &str) -> RelationshipType {
    match s {
        "Romantic" => RelationshipType::Romantic,
        "Professional" => RelationshipType::Professional,
        "Rivalry" => RelationshipType::Rivalry,
        "Friendship" => RelationshipType::Friendship,
        "Mentorship" => RelationshipType::Mentorship,
        "Enmity" => RelationshipType::Enmity,
        _ => RelationshipType::Custom(s.to_string()),
    }
}

