//! World API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::{
    CreateActRequest as ServiceCreateActRequest, CreateWorldRequest as ServiceCreateWorldRequest,
    UpdateWorldRequest as ServiceUpdateWorldRequest, WorldService,
};
use crate::domain::entities::{Act, MonomythStage, World};
use crate::domain::value_objects::{RuleSystemConfig, RuleSystemVariant, WorldId};
use crate::infrastructure::state::AppState;

/// Request to create a world - accepts just the variant and expands to full config
#[derive(Debug, Deserialize)]
pub struct CreateWorldRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Rule system configuration - can be a full config or just a variant
    #[serde(default)]
    pub rule_system: Option<RuleSystemInput>,
}

/// Flexible input for rule system - either a variant name or full config
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RuleSystemInput {
    /// Just specify a variant, and we'll expand to full config
    VariantOnly {
        variant: RuleSystemVariant,
    },
    /// Full configuration (for custom systems)
    Full(RuleSystemConfig),
}

impl RuleSystemInput {
    /// Convert to a full RuleSystemConfig
    pub fn into_config(self) -> RuleSystemConfig {
        match self {
            RuleSystemInput::VariantOnly { variant } => RuleSystemConfig::from_variant(variant),
            RuleSystemInput::Full(config) => config,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorldRequest {
    pub name: String,
    pub description: String,
    pub rule_system: RuleSystemConfig,
}

#[derive(Debug, Serialize)]
pub struct WorldResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_system: RuleSystemConfig,
    pub created_at: String,
    pub updated_at: String,
}

impl From<World> for WorldResponse {
    fn from(world: World) -> Self {
        Self {
            id: world.id.to_string(),
            name: world.name,
            description: world.description,
            rule_system: world.rule_system,
            created_at: world.created_at.to_rfc3339(),
            updated_at: world.updated_at.to_rfc3339(),
        }
    }
}

/// List all worlds
pub async fn list_worlds(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WorldResponse>>, (StatusCode, String)> {
    let worlds = state
        .world_service
        .list_worlds()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(worlds.into_iter().map(WorldResponse::from).collect()))
}

/// Create a new world
pub async fn create_world(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorldRequest>,
) -> Result<(StatusCode, Json<WorldResponse>), (StatusCode, String)> {
    let service_request = ServiceCreateWorldRequest {
        name: req.name,
        description: req.description,
        rule_system: req.rule_system.map(|r| r.into_config()),
    };

    let world = state
        .world_service
        .create_world(service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(WorldResponse::from(world))))
}

/// Get a world by ID
pub async fn get_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<WorldResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let world = state
        .world_service
        .get_world(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    Ok(Json(WorldResponse::from(world)))
}

/// Update a world
pub async fn update_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorldRequest>,
) -> Result<Json<WorldResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let service_request = ServiceUpdateWorldRequest {
        name: Some(req.name),
        description: Some(req.description),
        rule_system: Some(req.rule_system),
    };

    let world = state
        .world_service
        .update_world(WorldId::from_uuid(uuid), service_request)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "World not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(Json(WorldResponse::from(world)))
}

/// Delete a world
pub async fn delete_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    state
        .world_service
        .delete_world(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "World not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

// Act endpoints

#[derive(Debug, Deserialize)]
pub struct CreateActRequest {
    pub name: String,
    pub stage: String,
    #[serde(default)]
    pub description: String,
    pub order: u32,
}

#[derive(Debug, Serialize)]
pub struct ActResponse {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub stage: String,
    pub description: String,
    pub order: u32,
}

impl From<Act> for ActResponse {
    fn from(act: Act) -> Self {
        Self {
            id: act.id.to_string(),
            world_id: act.world_id.to_string(),
            name: act.name,
            stage: format!("{:?}", act.stage),
            description: act.description,
            order: act.order,
        }
    }
}

/// List acts in a world
pub async fn list_acts(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ActResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let acts = state
        .world_service
        .get_acts(WorldId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(acts.into_iter().map(ActResponse::from).collect()))
}

/// Create an act in a world
pub async fn create_act(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateActRequest>,
) -> Result<(StatusCode, Json<ActResponse>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;

    let stage = parse_monomyth_stage(&req.stage);
    let service_request = ServiceCreateActRequest {
        name: req.name,
        stage,
        description: if req.description.is_empty() {
            None
        } else {
            Some(req.description)
        },
        order: req.order,
    };

    let act = state
        .world_service
        .create_act(WorldId::from_uuid(uuid), service_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(ActResponse::from(act))))
}

fn parse_monomyth_stage(s: &str) -> MonomythStage {
    match s {
        "OrdinaryWorld" => MonomythStage::OrdinaryWorld,
        "CallToAdventure" => MonomythStage::CallToAdventure,
        "RefusalOfTheCall" => MonomythStage::RefusalOfTheCall,
        "MeetingTheMentor" => MonomythStage::MeetingTheMentor,
        "CrossingTheThreshold" => MonomythStage::CrossingTheThreshold,
        "TestsAlliesEnemies" => MonomythStage::TestsAlliesEnemies,
        "ApproachToInnermostCave" => MonomythStage::ApproachToInnermostCave,
        "Ordeal" => MonomythStage::Ordeal,
        "Reward" => MonomythStage::Reward,
        "TheRoadBack" => MonomythStage::TheRoadBack,
        "Resurrection" => MonomythStage::Resurrection,
        "ReturnWithElixir" => MonomythStage::ReturnWithElixir,
        _ => MonomythStage::OrdinaryWorld,
    }
}
