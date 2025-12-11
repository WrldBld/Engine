//! Character API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::Character;
use crate::domain::value_objects::{
    CampbellArchetype, CharacterId, Relationship, RelationshipId, RelationshipType, Want, WorldId,
};
use crate::infrastructure::persistence::SocialNetwork;
use crate::infrastructure::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateCharacterRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub archetype: String,
    #[serde(default)]
    pub sprite_asset: Option<String>,
    #[serde(default)]
    pub portrait_asset: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CharacterResponse {
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

impl From<Character> for CharacterResponse {
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

/// List characters in a world
pub async fn list_characters(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<CharacterResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let characters = state
        .repository
        .characters()
        .list_by_world(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        characters
            .into_iter()
            .map(CharacterResponse::from)
            .collect(),
    ))
}

/// Create a character
pub async fn create_character(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateCharacterRequest>,
) -> Result<(StatusCode, Json<CharacterResponse>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let archetype = parse_archetype(&req.archetype);
    let mut character = Character::new(WorldId::from_uuid(uuid), &req.name, archetype);

    if !req.description.is_empty() {
        character = character.with_description(&req.description);
    }
    if let Some(sprite) = req.sprite_asset {
        character = character.with_sprite(&sprite);
    }
    if let Some(portrait) = req.portrait_asset {
        character = character.with_portrait(&portrait);
    }

    state
        .repository
        .characters()
        .create(&character)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(CharacterResponse::from(character)),
    ))
}

/// Get a character by ID
pub async fn get_character(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<CharacterResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    let character = state
        .repository
        .characters()
        .get(CharacterId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Character not found".to_string()))?;

    Ok(Json(CharacterResponse::from(character)))
}

/// Update a character
pub async fn update_character(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateCharacterRequest>,
) -> Result<Json<CharacterResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    let mut character = state
        .repository
        .characters()
        .get(CharacterId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Character not found".to_string()))?;

    character.name = req.name;
    character.description = req.description;
    character.sprite_asset = req.sprite_asset;
    character.portrait_asset = req.portrait_asset;

    state
        .repository
        .characters()
        .update(&character)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CharacterResponse::from(character)))
}

/// Delete a character
pub async fn delete_character(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    state
        .repository
        .characters()
        .delete(CharacterId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ChangeArchetypeRequest {
    pub archetype: String,
    pub reason: String,
}

/// Change a character's archetype
pub async fn change_archetype(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ChangeArchetypeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    let archetype = parse_archetype(&req.archetype);

    state
        .repository
        .characters()
        .change_archetype(CharacterId::from_uuid(uuid), archetype, &req.reason)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

// Social network / Relationships

/// Get social network for a world
pub async fn get_social_network(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<SocialNetwork>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let network = state
        .repository
        .relationships()
        .get_social_network(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(network))
}

#[derive(Debug, Deserialize)]
pub struct CreateRelationshipRequest {
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

/// Create a relationship between characters
pub async fn create_relationship(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRelationshipRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&req.from_character_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid from character ID".to_string(),
        )
    })?;
    let to_uuid = Uuid::parse_str(&req.to_character_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid to character ID".to_string(),
        )
    })?;

    let rel_type = parse_relationship_type(&req.relationship_type);

    let mut relationship = Relationship::new(
        CharacterId::from_uuid(from_uuid),
        CharacterId::from_uuid(to_uuid),
        rel_type,
    )
    .with_sentiment(req.sentiment);

    if !req.known_to_player {
        relationship = relationship.secret();
    }

    state
        .repository
        .relationships()
        .create(&relationship)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": relationship.id.to_string()
        })),
    ))
}

/// Delete a relationship
pub async fn delete_relationship(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid relationship ID".to_string(),
        )
    })?;

    state
        .repository
        .relationships()
        .delete(RelationshipId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

fn parse_archetype(s: &str) -> CampbellArchetype {
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

fn parse_relationship_type(s: &str) -> RelationshipType {
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
