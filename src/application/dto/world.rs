use serde::{Deserialize, Serialize};

use crate::application::dto::{RuleSystemConfigDto, RuleSystemVariantDto};
use crate::domain::entities::{Act, MonomythStage, World};
use crate::domain::value_objects::RuleSystemConfig;

/// Flexible input for rule system - either a variant name or full config.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RuleSystemInputDto {
    VariantOnly { variant: RuleSystemVariantDto },
    Full(RuleSystemConfigDto),
}

impl RuleSystemInputDto {
    pub fn into_domain(self) -> RuleSystemConfig {
        match self {
            RuleSystemInputDto::VariantOnly { variant } => {
                RuleSystemConfig::from_variant(variant.into())
            }
            RuleSystemInputDto::Full(config) => config.into(),
        }
    }
}

/// Request to create a world - accepts just the variant and expands to full config.
#[derive(Debug, Deserialize)]
pub struct CreateWorldRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub rule_system: Option<RuleSystemInputDto>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorldRequestDto {
    pub name: String,
    pub description: String,
    pub rule_system: RuleSystemConfigDto,
}

#[derive(Debug, Serialize)]
pub struct WorldResponseDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_system: RuleSystemConfigDto,
    pub created_at: String,
    pub updated_at: String,
}

impl From<World> for WorldResponseDto {
    fn from(world: World) -> Self {
        Self {
            id: world.id.to_string(),
            name: world.name,
            description: world.description,
            rule_system: world.rule_system.into(),
            created_at: world.created_at.to_rfc3339(),
            updated_at: world.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateActRequestDto {
    pub name: String,
    pub stage: String,
    #[serde(default)]
    pub description: String,
    pub order: u32,
}

#[derive(Debug, Serialize)]
pub struct ActResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub stage: String,
    pub description: String,
    pub order: u32,
}

impl From<Act> for ActResponseDto {
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

pub fn parse_monomyth_stage(s: &str) -> MonomythStage {
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

