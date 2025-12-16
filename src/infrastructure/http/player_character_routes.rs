//! HTTP routes for player character management

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{
    PlayerCharacterService, PlayerCharacterServiceImpl,
    CreatePlayerCharacterRequest, UpdatePlayerCharacterRequest,
};
use crate::domain::entities::PlayerCharacter;
use crate::domain::entities::sheet_template::CharacterSheetData;
use crate::domain::value_objects::{
    LocationId, PlayerCharacterId, SessionId, WorldId,
};
use crate::infrastructure::state::AppState;

// =============================================================================
// Request/Response DTOs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlayerCharacterRequestDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub starting_location_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<CharacterSheetDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite_asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portrait_asset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlayerCharacterRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<CharacterSheetDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite_asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portrait_asset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSheetDataDto {
    pub values: std::collections::HashMap<String, FieldValueDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FieldValueDto {
    Number(i32),
    Text(String),
    Boolean(bool),
    Resource { current: i32, max: i32 },
    List(Vec<String>),
    SkillEntry {
        skill_id: String,
        proficient: bool,
        bonus: i32,
    },
}

impl From<FieldValueDto> for crate::domain::entities::sheet_template::FieldValue {
    fn from(dto: FieldValueDto) -> Self {
        match dto {
            FieldValueDto::Number(n) => Self::Number(n),
            FieldValueDto::Text(s) => Self::Text(s),
            FieldValueDto::Boolean(b) => Self::Boolean(b),
            FieldValueDto::Resource { current, max } => Self::Resource { current, max },
            FieldValueDto::List(l) => Self::List(l),
            FieldValueDto::SkillEntry { skill_id, proficient, bonus } => {
                Self::SkillEntry { skill_id, proficient, bonus }
            }
        }
    }
}

impl From<crate::domain::entities::sheet_template::FieldValue> for FieldValueDto {
    fn from(value: crate::domain::entities::sheet_template::FieldValue) -> Self {
        match value {
            crate::domain::entities::sheet_template::FieldValue::Number(n) => Self::Number(n),
            crate::domain::entities::sheet_template::FieldValue::Text(s) => Self::Text(s),
            crate::domain::entities::sheet_template::FieldValue::Boolean(b) => Self::Boolean(b),
            crate::domain::entities::sheet_template::FieldValue::Resource { current, max } => {
                Self::Resource { current, max }
            }
            crate::domain::entities::sheet_template::FieldValue::List(l) => Self::List(l),
            crate::domain::entities::sheet_template::FieldValue::SkillEntry { skill_id, proficient, bonus } => {
                Self::SkillEntry { skill_id, proficient, bonus }
            }
        }
    }
}

impl From<CharacterSheetDataDto> for CharacterSheetData {
    fn from(dto: CharacterSheetDataDto) -> Self {
        let mut sheet = CharacterSheetData::new();
        for (field_id, value) in dto.values {
            sheet.set(field_id, value.into());
        }
        sheet
    }
}

impl From<CharacterSheetData> for CharacterSheetDataDto {
    fn from(sheet: CharacterSheetData) -> Self {
        let mut values = std::collections::HashMap::new();
        for (field_id, value) in sheet.values {
            values.insert(field_id, value.into());
        }
        Self { values }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCharacterResponseDto {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub world_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<CharacterSheetDataDto>,
    pub current_location_id: String,
    pub starting_location_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite_asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portrait_asset: Option<String>,
    pub created_at: String,
    pub last_active_at: String,
}

impl From<PlayerCharacter> for PlayerCharacterResponseDto {
    fn from(pc: PlayerCharacter) -> Self {
        Self {
            id: pc.id.to_string(),
            session_id: pc.session_id.to_string(),
            user_id: pc.user_id,
            world_id: pc.world_id.to_string(),
            name: pc.name,
            description: pc.description,
            sheet_data: pc.sheet_data.map(|s| s.into()),
            current_location_id: pc.current_location_id.to_string(),
            starting_location_id: pc.starting_location_id.to_string(),
            sprite_asset: pc.sprite_asset,
            portrait_asset: pc.portrait_asset,
            created_at: pc.created_at.to_rfc3339(),
            last_active_at: pc.last_active_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLocationRequestDto {
    pub location_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLocationResponseDto {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
}

// =============================================================================
// Route Handlers
// =============================================================================

/// Create a new player character
pub async fn create_player_character(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(req): Json<CreatePlayerCharacterRequestDto>,
) -> Result<(StatusCode, Json<PlayerCharacterResponseDto>), (StatusCode, String)> {
    // Extract user_id from session (would normally come from auth token)
    // For now, we'll need to get it from the request or session
    // TODO: Get user_id from authenticated session
    let user_id = "temp_user".to_string(); // Placeholder - should come from auth

    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    let world_id = {
        let sessions = state.sessions.read().await;
        let session = sessions.get_session(session_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;
        session.world_id
    };

    let location_uuid = Uuid::parse_str(&req.starting_location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
    let location_id = LocationId::from_uuid(location_uuid);

    let sheet_data = req.sheet_data.map(|dto| dto.into());

    let service_request = CreatePlayerCharacterRequest {
        session_id,
        user_id,
        world_id,
        name: req.name,
        description: req.description,
        starting_location_id: location_id,
        sheet_data,
        sprite_asset: req.sprite_asset,
        portrait_asset: req.portrait_asset,
    };

    let pc = state
        .player_character_service
        .create_pc(service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Add PC to session
    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_session_mut(session_id) {
            session.add_player_character(pc.clone())
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
    }

    // Resolve scene for the new PC
    let scene_result = state
        .scene_resolution_service
        .resolve_scene_for_pc(pc.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // TODO: Broadcast SceneUpdate to player if scene found

    Ok((StatusCode::CREATED, Json(PlayerCharacterResponseDto::from(pc))))
}

/// Get all player characters in a session
pub async fn list_player_characters(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<PlayerCharacterResponseDto>>, (StatusCode, String)> {
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    let pcs = state
        .player_character_service
        .get_pcs_by_session(session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(pcs.into_iter().map(PlayerCharacterResponseDto::from).collect()))
}

/// Get current user's player character
pub async fn get_my_player_character(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<PlayerCharacterResponseDto>, (StatusCode, String)> {
    // TODO: Get user_id from authenticated session
    let user_id = "temp_user".to_string(); // Placeholder

    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    let pc = state
        .player_character_service
        .get_pc_by_user_and_session(&user_id, session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Player character not found".to_string()))?;

    Ok(Json(PlayerCharacterResponseDto::from(pc)))
}

/// Get a player character by ID
pub async fn get_player_character(
    State(state): State<Arc<AppState>>,
    Path(pc_id): Path<String>,
) -> Result<Json<PlayerCharacterResponseDto>, (StatusCode, String)> {
    let pc_uuid = Uuid::parse_str(&pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid player character ID".to_string()))?;
    let pc_id = PlayerCharacterId::from_uuid(pc_uuid);

    let pc = state
        .player_character_service
        .get_pc(pc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Player character not found".to_string()))?;

    Ok(Json(PlayerCharacterResponseDto::from(pc)))
}

/// Update a player character
pub async fn update_player_character(
    State(state): State<Arc<AppState>>,
    Path(pc_id): Path<String>,
    Json(req): Json<UpdatePlayerCharacterRequestDto>,
) -> Result<Json<PlayerCharacterResponseDto>, (StatusCode, String)> {
    let pc_uuid = Uuid::parse_str(&pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid player character ID".to_string()))?;
    let pc_id = PlayerCharacterId::from_uuid(pc_uuid);

    let sheet_data = req.sheet_data.map(|dto| dto.into());

    let service_request = UpdatePlayerCharacterRequest {
        name: req.name,
        description: req.description,
        sheet_data,
        sprite_asset: req.sprite_asset,
        portrait_asset: req.portrait_asset,
    };

    let pc = state
        .player_character_service
        .update_pc(pc_id, service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PlayerCharacterResponseDto::from(pc)))
}

/// Update a player character's location
pub async fn update_player_character_location(
    State(state): State<Arc<AppState>>,
    Path(pc_id): Path<String>,
    Json(req): Json<UpdateLocationRequestDto>,
) -> Result<Json<UpdateLocationResponseDto>, (StatusCode, String)> {
    let pc_uuid = Uuid::parse_str(&pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid player character ID".to_string()))?;
    let pc_id = PlayerCharacterId::from_uuid(pc_uuid);

    let location_uuid = Uuid::parse_str(&req.location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
    let location_id = LocationId::from_uuid(location_uuid);

    state
        .player_character_service
        .update_pc_location(pc_id, location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Resolve scene for the updated location
    let scene_result = state
        .scene_resolution_service
        .resolve_scene_for_pc(pc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // TODO: Broadcast SceneUpdate to player if scene found

    Ok(Json(UpdateLocationResponseDto {
        success: true,
        scene_id: scene_result.map(|s| s.id.to_string()),
    }))
}

/// Delete a player character
pub async fn delete_player_character(
    State(state): State<Arc<AppState>>,
    Path(pc_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pc_uuid = Uuid::parse_str(&pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid player character ID".to_string()))?;
    let pc_id = PlayerCharacterId::from_uuid(pc_uuid);

    state
        .player_character_service
        .delete_pc(pc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

