//! Character API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{
    ChangeArchetypeRequest as ServiceChangeArchetypeRequest, CharacterService,
    CreateCharacterRequest as ServiceCreateCharacterRequest, RelationshipService,
    UpdateCharacterRequest as ServiceUpdateCharacterRequest,
};
use crate::domain::value_objects::{
    CharacterId, Relationship, RelationshipId, WorldId,
};
use crate::application::ports::outbound::SocialNetwork;
use crate::application::dto::{
    ChangeArchetypeRequestDto, CharacterResponseDto, CreateCharacterRequestDto,
    CreateRelationshipRequestDto, CreatedIdResponseDto, parse_archetype, parse_relationship_type,
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
        .character_service
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
        wants: vec![],
    };

    let character = state
        .character_service
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
        .character_service
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
        .character_service
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
        .character_service
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
        .character_service
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
        .relationship_service
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
        .relationship_service
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
        .relationship_service
        .delete_relationship(RelationshipId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// NOTE: parsing helpers live in `application/dto/character.rs`.
