//! Character API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{
    ChangeArchetypeRequest as ServiceChangeArchetypeRequest, CharacterService,
    CreateCharacterRequest as ServiceCreateCharacterRequest, RelationshipService,
    UpdateCharacterRequest as ServiceUpdateCharacterRequest,
};
use crate::domain::value_objects::{CharacterId, RegionId, Relationship, RelationshipId, WorldId};
use crate::application::ports::outbound::SocialNetwork;
use crate::application::dto::{
    ChangeArchetypeRequestDto, CharacterResponseDto, CreateCharacterRequestDto,
    CreateRelationshipRequestDto, CreatedIdResponseDto, InventoryItemResponseDto,
    parse_archetype, parse_relationship_type,
};
use crate::infrastructure::persistence::{
    RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift,
};
use crate::infrastructure::state::AppState;

/// List characters in a world
pub async fn list_characters(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<CharacterResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let characters = state
        .core.character_service
        .list_characters(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(characters.into_iter().map(CharacterResponseDto::from).collect()))
}

/// Create a character
pub async fn create_character(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateCharacterRequestDto>,
) -> Result<(StatusCode, Json<CharacterResponseDto>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let archetype = parse_archetype(&req.archetype);
    let service_request = ServiceCreateCharacterRequest {
        world_id: WorldId::from_uuid(uuid),
        name: req.name,
        description: if req.description.is_empty() {
            None
        } else {
            Some(req.description)
        },
        archetype,
        sprite_asset: req.sprite_asset,
        portrait_asset: req.portrait_asset,
        stats: None,
        initial_wants: vec![],
    };

    let character = state
        .core.character_service
        .create_character(service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(CharacterResponseDto::from(character)),
    ))
}

/// Get a character by ID
pub async fn get_character(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<CharacterResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    let character = state
        .core.character_service
        .get_character(CharacterId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Character not found".to_string()))?;

    Ok(Json(CharacterResponseDto::from(character)))
}

/// Update a character
pub async fn update_character(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateCharacterRequestDto>,
) -> Result<Json<CharacterResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    let service_request = ServiceUpdateCharacterRequest {
        name: Some(req.name),
        description: if req.description.is_empty() {
            None
        } else {
            Some(req.description)
        },
        sprite_asset: req.sprite_asset,
        portrait_asset: req.portrait_asset,
        stats: None,
        is_alive: None,
        is_active: None,
    };

    let character = state
        .core.character_service
        .update_character(CharacterId::from_uuid(uuid), service_request)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Character not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(Json(CharacterResponseDto::from(character)))
}

/// Delete a character
pub async fn delete_character(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    state
        .core.character_service
        .delete_character(CharacterId::from_uuid(uuid))
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Character not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Change a character's archetype
pub async fn change_archetype(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ChangeArchetypeRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    let archetype = parse_archetype(&req.archetype);
    let service_request = ServiceChangeArchetypeRequest {
        new_archetype: archetype,
        reason: req.reason,
    };

    state
        .core.character_service
        .change_archetype(CharacterId::from_uuid(uuid), service_request)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Character not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

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
        .core.relationship_service
        .get_social_network(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(network))
}

/// Create a relationship between characters
pub async fn create_relationship(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRelationshipRequestDto>,
) -> Result<(StatusCode, Json<CreatedIdResponseDto>), (StatusCode, String)> {
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
        .core.relationship_service
        .create_relationship(&relationship)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreatedIdResponseDto {
            id: relationship.id.to_string(),
        }),
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
        .core.relationship_service
        .delete_relationship(RelationshipId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// NOTE: parsing helpers live in `application/dto/character.rs`.

// =============================================================================
// Inventory Routes (Phase 23B - US-CHAR-009)
// =============================================================================

/// Get character's inventory
pub async fn get_inventory(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
) -> Result<Json<Vec<InventoryItemResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    let inventory = state
        .repository
        .characters()
        .get_inventory(CharacterId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        inventory
            .into_iter()
            .map(InventoryItemResponseDto::from)
            .collect(),
    ))
}

// =============================================================================
// Region Relationship DTOs (Phase 23C)
// =============================================================================

/// Request to create a region relationship
#[derive(Debug, Deserialize)]
pub struct CreateRegionRelationshipRequest {
    pub region_id: String,
    #[serde(flatten)]
    pub relationship_type: RegionRelationshipTypeDto,
}

/// DTO for region relationship type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegionRelationshipTypeDto {
    Home,
    WorksAt {
        #[serde(default = "default_shift")]
        shift: String,
    },
    Frequents {
        #[serde(default = "default_frequency")]
        frequency: String,
    },
    Avoids {
        #[serde(default)]
        reason: String,
    },
}

fn default_shift() -> String {
    "always".to_string()
}

fn default_frequency() -> String {
    "sometimes".to_string()
}

/// Response for region relationship
#[derive(Debug, Serialize)]
pub struct RegionRelationshipResponse {
    pub region_id: String,
    pub region_name: String,
    #[serde(flatten)]
    pub relationship_type: RegionRelationshipTypeDto,
}

impl From<RegionRelationship> for RegionRelationshipResponse {
    fn from(rel: RegionRelationship) -> Self {
        let relationship_type = match rel.relationship_type {
            RegionRelationshipType::Home => RegionRelationshipTypeDto::Home,
            RegionRelationshipType::WorksAt { shift } => RegionRelationshipTypeDto::WorksAt {
                shift: shift.to_string(),
            },
            RegionRelationshipType::Frequents { frequency } => {
                RegionRelationshipTypeDto::Frequents {
                    frequency: frequency.to_string(),
                }
            }
            RegionRelationshipType::Avoids { reason } => {
                RegionRelationshipTypeDto::Avoids { reason }
            }
        };

        Self {
            region_id: rel.region_id.to_string(),
            region_name: rel.region_name,
            relationship_type,
        }
    }
}

// =============================================================================
// Region Relationship Routes (Phase 23C)
// =============================================================================

/// List all region relationships for a character
pub async fn list_region_relationships(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
) -> Result<Json<Vec<RegionRelationshipResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;

    let relationships = state
        .repository
        .characters()
        .list_region_relationships(CharacterId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        relationships
            .into_iter()
            .map(RegionRelationshipResponse::from)
            .collect(),
    ))
}

/// Add a region relationship for a character
pub async fn add_region_relationship(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
    Json(req): Json<CreateRegionRelationshipRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let char_uuid = Uuid::parse_str(&character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
    let region_uuid = Uuid::parse_str(&req.region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;

    let char_id = CharacterId::from_uuid(char_uuid);
    let region_id = RegionId::from_uuid(region_uuid);
    let repo = state.repository.characters();

    match req.relationship_type {
        RegionRelationshipTypeDto::Home => {
            repo.set_home_region(char_id, region_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        RegionRelationshipTypeDto::WorksAt { shift } => {
            let shift: RegionShift = shift
                .parse()
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid shift value".to_string()))?;
            repo.set_work_region(char_id, region_id, shift)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        RegionRelationshipTypeDto::Frequents { frequency } => {
            let frequency: RegionFrequency = frequency
                .parse()
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid frequency value".to_string()))?;
            repo.add_frequented_region(char_id, region_id, frequency)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        RegionRelationshipTypeDto::Avoids { reason } => {
            repo.add_avoided_region(char_id, region_id, reason)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    Ok(StatusCode::CREATED)
}

/// Remove a region relationship for a character
pub async fn remove_region_relationship(
    State(state): State<Arc<AppState>>,
    Path((character_id, region_id, rel_type)): Path<(String, String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let char_uuid = Uuid::parse_str(&character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
    let region_uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;

    let char_id = CharacterId::from_uuid(char_uuid);
    let region_id = RegionId::from_uuid(region_uuid);
    let repo = state.repository.characters();

    match rel_type.as_str() {
        "home" => {
            repo.remove_home_region(char_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        "works_at" => {
            repo.remove_work_region(char_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        "frequents" => {
            repo.remove_frequented_region(char_id, region_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        "avoids" => {
            repo.remove_avoided_region(char_id, region_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid relationship type: {}", rel_type),
            ));
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
