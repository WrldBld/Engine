//! Region API routes
//!
//! Endpoints for managing regions within locations.
//! Regions are sub-areas within a Location, each with their own backdrop.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::dto::{CharacterResponseDto, CreateRegionRequestDto, MapBoundsDto, RegionResponseDto};
use crate::domain::entities::{MapBounds, NpcObservation, Region, RegionConnection, RegionExit};
use crate::domain::value_objects::{CharacterId, GameTime, LocationId, PlayerCharacterId, RegionId, SessionId, TimeOfDay, WorldId};
use crate::infrastructure::persistence::{RegionRelationshipType, RegionShift, RegionFrequency};
use crate::infrastructure::state::AppState;

// =============================================================================
// Request/Response DTOs specific to routes
// =============================================================================

#[derive(Debug, Serialize)]
pub struct SpawnPointResponseDto {
    pub region: RegionResponseDto,
    pub location_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRegionConnectionRequestDto {
    pub to_region_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_bidirectional")]
    pub bidirectional: bool,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub lock_description: Option<String>,
}

fn default_bidirectional() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct RegionConnectionResponseDto {
    pub from_region_id: String,
    pub to_region_id: String,
    pub description: Option<String>,
    pub bidirectional: bool,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

impl From<RegionConnection> for RegionConnectionResponseDto {
    fn from(c: RegionConnection) -> Self {
        Self {
            from_region_id: c.from_region.to_string(),
            to_region_id: c.to_region.to_string(),
            description: c.description,
            bidirectional: c.bidirectional,
            is_locked: c.is_locked,
            lock_description: c.lock_description,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRegionExitRequestDto {
    pub to_location_id: String,
    pub arrival_region_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_bidirectional")]
    pub bidirectional: bool,
}

#[derive(Debug, Serialize)]
pub struct RegionExitResponseDto {
    pub from_region_id: String,
    pub to_location_id: String,
    pub arrival_region_id: String,
    pub description: Option<String>,
    pub bidirectional: bool,
}

impl From<RegionExit> for RegionExitResponseDto {
    fn from(e: RegionExit) -> Self {
        Self {
            from_region_id: e.from_region.to_string(),
            to_location_id: e.to_location.to_string(),
            arrival_region_id: e.arrival_region_id.to_string(),
            description: e.description,
            bidirectional: e.bidirectional,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateRegionRequestDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub map_bounds: Option<MapBoundsDto>,
    pub is_spawn_point: Option<bool>,
    pub order: Option<u32>,
}

// =============================================================================
// Region CRUD
// =============================================================================

/// List all regions in a location
///
/// GET /api/locations/{location_id}/regions
pub async fn list_regions(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
) -> Result<Json<Vec<RegionResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
    let location_id = LocationId::from_uuid(uuid);

    let regions = state
        .repository
        .regions()
        .list_by_location(location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(regions.into_iter().map(RegionResponseDto::from).collect()))
}

/// Get a region by ID
///
/// GET /api/regions/{region_id}
pub async fn get_region(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<Json<RegionResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let region = state
        .repository
        .regions()
        .get(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Region not found".to_string()))?;

    Ok(Json(RegionResponseDto::from(region)))
}

/// Create a region in a location
///
/// POST /api/locations/{location_id}/regions
pub async fn create_region(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Json(req): Json<CreateRegionRequestDto>,
) -> Result<(StatusCode, Json<RegionResponseDto>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
    let location_id = LocationId::from_uuid(uuid);

    // Verify the location exists
    let location = state
        .repository
        .locations()
        .get(location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Location not found".to_string()))?;

    let mut region = Region::new(location_id, &req.name);
    region = region.with_description(&req.description);

    if let Some(backdrop) = req.backdrop_asset {
        region = region.with_backdrop(backdrop);
    }
    if let Some(atmosphere) = req.atmosphere {
        region = region.with_atmosphere(atmosphere);
    }
    if let Some(bounds) = req.map_bounds {
        region = region.with_map_bounds(MapBounds::from(bounds));
    }
    if req.is_spawn_point {
        region = region.as_spawn_point();
    }
    region = region.with_order(req.order);

    state
        .repository
        .regions()
        .create(&region)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(RegionResponseDto::from(region))))
}

/// Update a region
///
/// PATCH /api/regions/{region_id}
pub async fn update_region(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
    Json(req): Json<UpdateRegionRequestDto>,
) -> Result<Json<RegionResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let mut region = state
        .repository
        .regions()
        .get(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Region not found".to_string()))?;

    if let Some(name) = req.name {
        region.name = name;
    }
    if let Some(description) = req.description {
        region.description = description;
    }
    if let Some(backdrop) = req.backdrop_asset {
        region.backdrop_asset = Some(backdrop);
    }
    if let Some(atmosphere) = req.atmosphere {
        region.atmosphere = Some(atmosphere);
    }
    if let Some(bounds) = req.map_bounds {
        region.map_bounds = Some(MapBounds::from(bounds));
    }
    if let Some(is_spawn) = req.is_spawn_point {
        region.is_spawn_point = is_spawn;
    }
    if let Some(order) = req.order {
        region.order = order;
    }

    state
        .repository
        .regions()
        .update(&region)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RegionResponseDto::from(region)))
}

/// Delete a region
///
/// DELETE /api/regions/{region_id}
pub async fn delete_region(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    state
        .repository
        .regions()
        .delete(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Spawn Points
// =============================================================================

/// List all spawn points in a world
///
/// GET /api/worlds/{world_id}/spawn-points
pub async fn list_spawn_points(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<RegionResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let regions = state
        .repository
        .regions()
        .list_spawn_points(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(regions.into_iter().map(RegionResponseDto::from).collect()))
}

// =============================================================================
// Region Connections
// =============================================================================

/// List connections from a region
///
/// GET /api/regions/{region_id}/connections
pub async fn list_region_connections(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<Json<Vec<RegionConnectionResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let connections = state
        .repository
        .regions()
        .get_connections(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        connections
            .into_iter()
            .map(RegionConnectionResponseDto::from)
            .collect(),
    ))
}

/// Create a connection between regions
///
/// POST /api/regions/{region_id}/connections
pub async fn create_region_connection(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
    Json(req): Json<CreateRegionConnectionRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let to_uuid = Uuid::parse_str(&req.to_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid to_region_id".to_string()))?;

    let from_region = RegionId::from_uuid(from_uuid);
    let to_region = RegionId::from_uuid(to_uuid);

    let connection = RegionConnection {
        from_region,
        to_region,
        description: req.description,
        bidirectional: req.bidirectional,
        is_locked: req.is_locked,
        lock_description: req.lock_description,
    };

    state
        .repository
        .regions()
        .create_connection(&connection)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Delete a connection between regions
///
/// DELETE /api/regions/{from_region_id}/connections/{to_region_id}
pub async fn delete_region_connection(
    State(state): State<Arc<AppState>>,
    Path((from_region_id, to_region_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&from_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid from region ID".to_string()))?;
    let to_uuid = Uuid::parse_str(&to_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid to region ID".to_string()))?;

    let from_region = RegionId::from_uuid(from_uuid);
    let to_region = RegionId::from_uuid(to_uuid);

    state
        .repository
        .regions()
        .delete_connection(from_region, to_region)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Unlock a connection between regions
///
/// POST /api/regions/{from_region_id}/connections/{to_region_id}/unlock
pub async fn unlock_region_connection(
    State(state): State<Arc<AppState>>,
    Path((from_region_id, to_region_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&from_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid from region ID".to_string()))?;
    let to_uuid = Uuid::parse_str(&to_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid to region ID".to_string()))?;

    let from_region = RegionId::from_uuid(from_uuid);
    let to_region = RegionId::from_uuid(to_uuid);

    state
        .repository
        .regions()
        .unlock_connection(from_region, to_region)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

// =============================================================================
// Region Exits
// =============================================================================

/// List exits from a region
///
/// GET /api/regions/{region_id}/exits
pub async fn list_region_exits(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<Json<Vec<RegionExitResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let exits = state
        .repository
        .regions()
        .get_exits(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(exits.into_iter().map(RegionExitResponseDto::from).collect()))
}

/// Create an exit from a region to another location
///
/// POST /api/regions/{region_id}/exits
pub async fn create_region_exit(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
    Json(req): Json<CreateRegionExitRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let to_location_uuid = Uuid::parse_str(&req.to_location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid to_location_id".to_string()))?;
    let arrival_uuid = Uuid::parse_str(&req.arrival_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid arrival_region_id".to_string()))?;

    let from_region = RegionId::from_uuid(from_uuid);
    let to_location = LocationId::from_uuid(to_location_uuid);
    let arrival_region = RegionId::from_uuid(arrival_uuid);

    let exit = RegionExit {
        from_region,
        to_location,
        arrival_region_id: arrival_region,
        description: req.description,
        bidirectional: req.bidirectional,
    };

    state
        .repository
        .regions()
        .create_exit(&exit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Delete an exit from a region to a location
///
/// DELETE /api/regions/{region_id}/exits/{location_id}
pub async fn delete_region_exit(
    State(state): State<Arc<AppState>>,
    Path((region_id, location_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let region_uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let location_uuid = Uuid::parse_str(&location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let from_region = RegionId::from_uuid(region_uuid);
    let to_location = LocationId::from_uuid(location_uuid);

    state
        .repository
        .regions()
        .delete_exit(from_region, to_location)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// NPC-Region Relationships (Phase 23C)
// =============================================================================

/// DTO for NPC with their relationship to a region
#[derive(Debug, Serialize)]
pub struct NpcWithRegionRelationshipDto {
    pub character: CharacterResponseDto,
    #[serde(flatten)]
    pub relationship: RegionRelationshipTypeDto,
}

/// DTO for region relationship type (matches character_routes)
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegionRelationshipTypeDto {
    Home,
    WorksAt { shift: String },
    Frequents { frequency: String },
    Avoids { reason: String },
}

impl From<RegionRelationshipType> for RegionRelationshipTypeDto {
    fn from(rel: RegionRelationshipType) -> Self {
        match rel {
            RegionRelationshipType::Home => RegionRelationshipTypeDto::Home,
            RegionRelationshipType::WorksAt { shift } => RegionRelationshipTypeDto::WorksAt {
                shift: shift.to_string(),
            },
            RegionRelationshipType::Frequents { frequency } => {
                RegionRelationshipTypeDto::Frequents {
                    frequency: frequency.to_string(),
                }
            }
            RegionRelationshipType::Avoids { reason } => RegionRelationshipTypeDto::Avoids { reason },
        }
    }
}

/// List all NPCs related to a region
///
/// GET /api/regions/{region_id}/npcs
pub async fn list_region_npcs(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<Json<Vec<NpcWithRegionRelationshipDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let npcs = state
        .repository
        .characters()
        .get_npcs_related_to_region(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        npcs.into_iter()
            .map(|(character, rel_type)| NpcWithRegionRelationshipDto {
                character: CharacterResponseDto::from(character),
                relationship: RegionRelationshipTypeDto::from(rel_type),
            })
            .collect(),
    ))
}

// =============================================================================
// Derived Scene Query (Phase 23C)
// =============================================================================

/// DTO for derived scene response
#[derive(Debug, Serialize)]
pub struct DerivedSceneDto {
    /// The region info
    pub region: RegionResponseDto,
    /// Parent location info
    pub location_name: String,
    /// Backdrop to display (region backdrop or fallback to location)
    pub backdrop_asset: Option<String>,
    /// Atmosphere text
    pub atmosphere: Option<String>,
    /// NPCs currently present in this region
    pub npcs_present: Vec<NpcPresenceDto>,
    /// Navigation options from this region
    pub navigation: NavigationOptionsDto,
    /// Current game time info
    pub game_time: GameTimeDto,
}

/// DTO for NPC presence in derived scene
#[derive(Debug, Serialize)]
pub struct NpcPresenceDto {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    /// Only included for DMs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_reasoning: Option<String>,
}

/// DTO for navigation options
#[derive(Debug, Serialize)]
pub struct NavigationOptionsDto {
    /// Connected regions within same location
    pub connected_regions: Vec<NavigationTargetDto>,
    /// Exits to other locations
    pub exits: Vec<NavigationExitDto>,
}

/// DTO for a navigation target (region)
#[derive(Debug, Serialize)]
pub struct NavigationTargetDto {
    pub region_id: String,
    pub name: String,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

/// DTO for a navigation exit (to location)
#[derive(Debug, Serialize)]
pub struct NavigationExitDto {
    pub location_id: String,
    pub location_name: String,
    pub arrival_region_id: String,
    pub description: Option<String>,
}

/// DTO for game time display
#[derive(Debug, Serialize)]
pub struct GameTimeDto {
    pub display: String,
    pub time_of_day: String,
    pub is_paused: bool,
}

impl From<&GameTime> for GameTimeDto {
    fn from(gt: &GameTime) -> Self {
        Self {
            display: gt.display_date(),
            time_of_day: gt.time_of_day().to_string(),
            is_paused: gt.is_paused(),
        }
    }
}

/// Request for derived scene query
#[derive(Debug, Deserialize)]
pub struct DerivedSceneRequest {
    /// Session ID to get game time from
    pub session_id: String,
    /// Optional PC ID - if provided, auto-creates Direct observations for NPCs present
    #[serde(default)]
    pub pc_id: Option<String>,
    /// Whether to include DM-only info (presence reasoning)
    #[serde(default)]
    pub include_dm_info: bool,
}

/// Get a derived scene for a region
///
/// This combines region data with dynamically determined NPC presence
/// based on the session's game time.
///
/// POST /api/regions/{region_id}/scene
pub async fn get_derived_scene(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
    Json(req): Json<DerivedSceneRequest>,
) -> Result<Json<DerivedSceneDto>, (StatusCode, String)> {
    let region_uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let session_uuid = Uuid::parse_str(&req.session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;

    let region_id = RegionId::from_uuid(region_uuid);
    let session_id = SessionId::from_uuid(session_uuid);

    // Get the region
    let region = state
        .repository
        .regions()
        .get(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Region not found".to_string()))?;

    // Get the parent location for name and fallback backdrop
    let location = state
        .repository
        .locations()
        .get(region.location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Parent location not found".to_string()))?;

    // Get game time from session
    let game_time = {
        let sessions = state.sessions.read().await;
        let session = sessions
            .get_session(session_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;
        session.game_time().clone()
    };

    // Determine NPC presence using simple rules (LLM integration would be async/complex)
    // For now, use the simple rule-based approach
    let npc_relationships = state
        .repository
        .characters()
        .get_npcs_related_to_region(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let time_of_day = game_time.time_of_day();
    
    // Determine which NPCs are present and collect their info
    let mut npcs_present: Vec<NpcPresenceDto> = Vec::new();
    let mut present_npc_ids: Vec<CharacterId> = Vec::new();
    
    for (character, rel_type) in npc_relationships {
        let (is_present, reasoning) = determine_simple_presence(&rel_type, time_of_day);
        if is_present {
            present_npc_ids.push(character.id);
            npcs_present.push(NpcPresenceDto {
                character_id: character.id.to_string(),
                name: character.name,
                sprite_asset: character.sprite_asset,
                presence_reasoning: if req.include_dm_info {
                    Some(reasoning)
                } else {
                    None
                },
            });
        }
    }
    
    // If a PC ID was provided, auto-create Direct observations for all present NPCs
    if let Some(ref pc_id_str) = req.pc_id {
        if let Ok(pc_uuid) = Uuid::parse_str(pc_id_str) {
            let pc_id = PlayerCharacterId::from_uuid(pc_uuid);
            let observations: Vec<NpcObservation> = present_npc_ids
                .iter()
                .map(|npc_id| {
                    NpcObservation::direct(
                        pc_id,
                        *npc_id,
                        region.location_id,
                        region_id,
                        game_time.current(),
                    )
                })
                .collect();
            
            if !observations.is_empty() {
                if let Err(e) = state.repository.observations().batch_upsert(&observations).await {
                    tracing::warn!("Failed to create observations: {}", e);
                    // Non-fatal: continue with the response even if observations fail
                }
            }
        }
    }

    // Get navigation options
    let connections = state
        .repository
        .regions()
        .get_connections(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let exits = state
        .repository
        .regions()
        .get_exits(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Build navigation targets (need to fetch region names)
    let mut connected_regions = Vec::new();
    for conn in connections {
        if let Ok(Some(target_region)) = state.repository.regions().get(conn.to_region).await {
            connected_regions.push(NavigationTargetDto {
                region_id: conn.to_region.to_string(),
                name: target_region.name,
                is_locked: conn.is_locked,
                lock_description: conn.lock_description,
            });
        }
    }

    // Build exit targets (need to fetch location names)
    let mut exit_targets = Vec::new();
    for exit in exits {
        if let Ok(Some(target_location)) = state.repository.locations().get(exit.to_location).await {
            exit_targets.push(NavigationExitDto {
                location_id: exit.to_location.to_string(),
                location_name: target_location.name,
                arrival_region_id: exit.arrival_region_id.to_string(),
                description: exit.description,
            });
        }
    }

    // Determine backdrop (region backdrop or fallback to location)
    let backdrop_asset = region
        .backdrop_asset
        .clone()
        .or_else(|| location.backdrop_asset.clone());

    Ok(Json(DerivedSceneDto {
        region: RegionResponseDto::from(region),
        location_name: location.name,
        backdrop_asset,
        atmosphere: location.atmosphere,
        npcs_present,
        navigation: NavigationOptionsDto {
            connected_regions,
            exits: exit_targets,
        },
        game_time: GameTimeDto::from(&game_time),
    }))
}

/// Simple rule-based presence determination
fn determine_simple_presence(rel_type: &RegionRelationshipType, time_of_day: TimeOfDay) -> (bool, String) {
    match rel_type {
        RegionRelationshipType::Home => {
            let present = matches!(time_of_day, TimeOfDay::Night | TimeOfDay::Evening);
            (present, format!("Lives here. {} is typically home time.", time_of_day))
        }
        RegionRelationshipType::WorksAt { shift } => {
            let present = match shift {
                RegionShift::Always => true,
                RegionShift::Day => matches!(time_of_day, TimeOfDay::Morning | TimeOfDay::Afternoon),
                RegionShift::Night => matches!(time_of_day, TimeOfDay::Evening | TimeOfDay::Night),
            };
            (present, format!("Works here ({:?} shift). Current time: {}", shift, time_of_day))
        }
        RegionRelationshipType::Frequents { frequency } => {
            let present = match frequency {
                RegionFrequency::Often => true,
                RegionFrequency::Sometimes => matches!(time_of_day, TimeOfDay::Afternoon | TimeOfDay::Evening),
                RegionFrequency::Rarely => false,
            };
            (present, format!("Frequents here ({:?}). Current time: {}", frequency, time_of_day))
        }
        RegionRelationshipType::Avoids { reason } => {
            (false, format!("Avoids this location: {}", reason))
        }
    }
}
