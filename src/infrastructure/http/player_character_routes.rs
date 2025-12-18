//! HTTP routes for player character management

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::outbound::PlayerCharacterRepositoryPort;
use crate::application::services::{
    PlayerCharacterService,
    SceneResolutionService,
    CreatePlayerCharacterRequest, UpdatePlayerCharacterRequest,
};
use crate::domain::entities::PlayerCharacter;
use crate::domain::entities::{CharacterSheetData, FieldValue};
use crate::domain::value_objects::{
    LocationId, PlayerCharacterId, RegionId, SessionId, WorldId,
};
use crate::infrastructure::state::AppState;

/// Extract user ID from X-User-Id header, falling back to a default if not provided
fn extract_user_id(headers: &HeaderMap) -> String {
    headers
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "anonymous".to_string())
}

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

impl From<FieldValueDto> for FieldValue {
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

impl From<FieldValue> for FieldValueDto {
    fn from(value: FieldValue) -> Self {
        match value {
            FieldValue::Number(n) => Self::Number(n),
            FieldValue::Text(s) => Self::Text(s),
            FieldValue::Boolean(b) => Self::Boolean(b),
            FieldValue::Resource { current, max } => {
                Self::Resource { current, max }
            }
            FieldValue::List(l) => Self::List(l),
            FieldValue::SkillEntry { skill_id, proficient, bonus } => {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub user_id: String,
    pub world_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<CharacterSheetDataDto>,
    pub current_location_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_region_id: Option<String>,
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
            session_id: pc.session_id.map(|s| s.to_string()),
            user_id: pc.user_id,
            world_id: pc.world_id.to_string(),
            name: pc.name,
            description: pc.description,
            sheet_data: pc.sheet_data.map(|s| s.into()),
            current_location_id: pc.current_location_id.to_string(),
            current_region_id: pc.current_region_id.map(|r| r.to_string()),
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
    headers: HeaderMap,
    Json(req): Json<CreatePlayerCharacterRequestDto>,
) -> Result<(StatusCode, Json<PlayerCharacterResponseDto>), (StatusCode, String)> {
    // Extract user_id from X-User-Id header (set by Player client)
    let user_id = extract_user_id(&headers);

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
        session_id: Some(session_id),
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
                .player.player_character_service
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
                .player.scene_resolution_service
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
                .player.player_character_service
        .get_pcs_by_session(session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(pcs.into_iter().map(PlayerCharacterResponseDto::from).collect()))
}

/// Get current user's player character
pub async fn get_my_player_character(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<PlayerCharacterResponseDto>, (StatusCode, String)> {
    // Extract user_id from X-User-Id header (set by Player client)
    let user_id = extract_user_id(&headers);

    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    let pc = state
                .player.player_character_service
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
                .player.player_character_service
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
                .player.player_character_service
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
                .player.player_character_service
        .update_pc_location(pc_id, location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Resolve scene for the updated location
    let scene_result = state
                .player.scene_resolution_service
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
                .player.player_character_service
        .delete_pc(pc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Phase 23B.6: PC Selection Routes
// =============================================================================

/// Response DTO for available PCs (simplified for selection UI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailablePcDto {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub current_location_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_location_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_region_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_region_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite_asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portrait_asset: Option<String>,
    pub last_active_at: String,
}

/// Request DTO for selecting a PC
#[derive(Debug, Clone, Deserialize)]
pub struct SelectPcRequestDto {
    pub pc_id: String,
}

/// Response DTO for PC selection
#[derive(Debug, Clone, Serialize)]
pub struct SelectPcResponseDto {
    pub pc: PlayerCharacterResponseDto,
    pub location_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_name: Option<String>,
}

/// Query params for listing user's PCs
#[derive(Debug, Clone, Deserialize)]
pub struct ListUserPcsQuery {
    #[serde(default)]
    pub rule_system: Option<String>,
}

/// Request DTO for importing a PC
#[derive(Debug, Clone, Deserialize)]
pub struct ImportPcRequestDto {
    /// Source PC ID to copy from
    pub source_pc_id: String,
    /// Region ID where the imported PC will spawn
    pub spawn_region_id: String,
}

/// List available PCs for a user to select in a session
///
/// GET /api/sessions/{session_id}/available-pcs
pub async fn list_available_pcs(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<AvailablePcDto>>, (StatusCode, String)> {
    let user_id = extract_user_id(&headers);

    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    // Get the world ID from the session
    let world_id = {
        let sessions = state.sessions.read().await;
        let session = sessions.get_session(session_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;
        session.world_id
    };

    // Get all PCs for this user in this world
    let pcs = state.repository.player_characters()
        .get_by_user_and_world(&user_id, world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Convert to DTOs with location names
    let mut result = Vec::new();
    for pc in pcs {
        let location_name = state.repository.locations()
            .get(pc.current_location_id)
            .await
            .ok()
            .flatten()
            .map(|l| l.name);

        let region_name = if let Some(region_id) = pc.current_region_id {
            state.repository.regions()
                .get(region_id)
                .await
                .ok()
                .flatten()
                .map(|r| r.name)
        } else {
            None
        };

        result.push(AvailablePcDto {
            id: pc.id.to_string(),
            name: pc.name,
            description: pc.description,
            current_location_id: pc.current_location_id.to_string(),
            current_location_name: location_name,
            current_region_id: pc.current_region_id.map(|r| r.to_string()),
            current_region_name: region_name,
            sprite_asset: pc.sprite_asset,
            portrait_asset: pc.portrait_asset,
            last_active_at: pc.last_active_at.to_rfc3339(),
        });
    }

    Ok(Json(result))
}

/// Select a PC to play in a session
///
/// POST /api/sessions/{session_id}/select-pc
pub async fn select_pc(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<SelectPcRequestDto>,
) -> Result<Json<SelectPcResponseDto>, (StatusCode, String)> {
    let user_id = extract_user_id(&headers);

    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    let pc_uuid = Uuid::parse_str(&req.pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid PC ID".to_string()))?;
    let pc_id = PlayerCharacterId::from_uuid(pc_uuid);

    // Get the world ID from the session
    let world_id = {
        let sessions = state.sessions.read().await;
        let session = sessions.get_session(session_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;
        session.world_id
    };

    // Get the PC and verify ownership
    let pc = state.repository.player_characters()
        .get(pc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Player character not found".to_string()))?;

    if pc.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "You don't own this character".to_string()));
    }

    if pc.world_id != world_id {
        return Err((StatusCode::BAD_REQUEST, "Character is from a different world".to_string()));
    }

    // Bind PC to session if not already bound
    if pc.session_id != Some(session_id) {
        state.repository.player_characters()
            .bind_to_session(pc_id, session_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Update last_active_at
    let mut updated_pc = pc.clone();
    updated_pc.last_active_at = chrono::Utc::now();
    state.repository.player_characters()
        .update(&updated_pc)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get location name
    let location = state.repository.locations()
        .get(pc.current_location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "PC's location not found".to_string()))?;

    // Get region name if applicable
    let region_name = if let Some(region_id) = pc.current_region_id {
        state.repository.regions()
            .get(region_id)
            .await
            .ok()
            .flatten()
            .map(|r| r.name)
    } else {
        None
    };

    // Add PC to session's in-memory state
    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_session_mut(session_id) {
            // Ignore error if PC already in session
            let _ = session.add_player_character(updated_pc.clone());
        }
    }

    Ok(Json(SelectPcResponseDto {
        pc: PlayerCharacterResponseDto::from(updated_pc),
        location_name: location.name,
        region_name,
    }))
}

/// List all PCs for a user across all worlds
///
/// GET /api/users/{user_id}/pcs?rule_system=D20
pub async fn list_user_pcs(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Query(query): Query<ListUserPcsQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<PlayerCharacterResponseDto>>, (StatusCode, String)> {
    // Validate that the requester is the same user (or allow admin access)
    let requester_id = extract_user_id(&headers);
    if requester_id != user_id && requester_id != "admin" {
        return Err((StatusCode::FORBIDDEN, "Cannot access other user's characters".to_string()));
    }

    // Get all unbound PCs for this user (they can be imported)
    // In practice, we want PCs from all worlds, so we need to query differently
    // For now, get unbound PCs which represents "available for import"
    let pcs = state.repository.player_characters()
        .get_unbound_by_user(&user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // If rule_system filter is specified, filter by world's rule system
    let filtered_pcs = if let Some(rule_system) = query.rule_system {
        use crate::domain::value_objects::RuleSystemType;
        let target_type = match rule_system.to_uppercase().as_str() {
            "D20" => Some(RuleSystemType::D20),
            "D100" => Some(RuleSystemType::D100),
            "NARRATIVE" => Some(RuleSystemType::Narrative),
            "CUSTOM" => Some(RuleSystemType::Custom),
            _ => None,
        };

        if let Some(target) = target_type {
            let mut result = Vec::new();
            for pc in pcs {
                // Get the world to check its rule system
                if let Ok(Some(world)) = state.repository.worlds().get(pc.world_id).await {
                    if world.rule_system.system_type == target {
                        result.push(pc);
                    }
                }
            }
            result
        } else {
            // Invalid rule system filter - return empty
            Vec::new()
        }
    } else {
        pcs
    };

    Ok(Json(filtered_pcs.into_iter().map(PlayerCharacterResponseDto::from).collect()))
}

/// Import (copy) a PC from another world into this world
///
/// POST /api/worlds/{world_id}/import-pc
pub async fn import_pc(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ImportPcRequestDto>,
) -> Result<(StatusCode, Json<PlayerCharacterResponseDto>), (StatusCode, String)> {
    let user_id = extract_user_id(&headers);

    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let target_world_id = WorldId::from_uuid(world_uuid);

    let source_pc_uuid = Uuid::parse_str(&req.source_pc_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid source PC ID".to_string()))?;
    let source_pc_id = PlayerCharacterId::from_uuid(source_pc_uuid);

    let spawn_region_uuid = Uuid::parse_str(&req.spawn_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid spawn region ID".to_string()))?;
    let spawn_region_id = RegionId::from_uuid(spawn_region_uuid);

    // Get the source PC
    let source_pc = state.repository.player_characters()
        .get(source_pc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Source character not found".to_string()))?;

    // Verify ownership
    if source_pc.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "You don't own this character".to_string()));
    }

    // Verify source PC is from a different world
    if source_pc.world_id == target_world_id {
        return Err((StatusCode::BAD_REQUEST, "Character is already in this world".to_string()));
    }

    // Get the spawn region to verify it exists and get its location
    let spawn_region = state.repository.regions()
        .get(spawn_region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Spawn region not found".to_string()))?;

    // Verify the spawn region is a spawn point
    if !spawn_region.is_spawn_point {
        return Err((StatusCode::BAD_REQUEST, "Selected region is not a spawn point".to_string()));
    }

    // Create the new PC as a copy
    let mut new_pc = PlayerCharacter::new(
        user_id,
        target_world_id,
        source_pc.name.clone(),
        spawn_region.location_id,
    )
    .with_starting_region(spawn_region_id);

    // Copy optional fields
    if let Some(desc) = source_pc.description {
        new_pc = new_pc.with_description(desc);
    }
    if let Some(sheet) = source_pc.sheet_data {
        new_pc = new_pc.with_sheet_data(sheet);
    }
    if let Some(sprite) = source_pc.sprite_asset {
        new_pc = new_pc.with_sprite(sprite);
    }
    if let Some(portrait) = source_pc.portrait_asset {
        new_pc = new_pc.with_portrait(portrait);
    }

    // Save the new PC
    state.repository.player_characters()
        .create(&new_pc)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(PlayerCharacterResponseDto::from(new_pc))))
}
