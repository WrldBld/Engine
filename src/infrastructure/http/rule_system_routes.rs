//! Rule System API routes
//!
//! Provides endpoints for listing available rule systems and their presets.

use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};

use crate::application::dto::{
    parse_system_type, parse_variant, RuleSystemPresetDetailsDto, RuleSystemPresetSummaryDto,
    RuleSystemSummaryDto, RuleSystemTypeDetailsDto,
};
use crate::domain::value_objects::{RuleSystemConfig, RuleSystemType, RuleSystemVariant};

/// List all available rule system types
pub async fn list_rule_systems() -> impl IntoResponse {
    let systems = vec![
        RuleSystemSummaryDto {
            system_type: RuleSystemType::D20.into(),
            name: "D20 System".to_string(),
            description: "Roll d20 + modifier vs Difficulty Class. Used by D&D, Pathfinder, and similar games.".to_string(),
            dice_notation: "1d20".to_string(),
            presets: RuleSystemVariant::variants_for_type(RuleSystemType::D20)
                .into_iter()
                .map(|v| {
                    let config = RuleSystemConfig::from_variant(v.clone());
                    RuleSystemPresetSummaryDto {
                        variant: v.into(),
                        name: config.name,
                        description: config.description,
                    }
                })
                .collect(),
        },
        RuleSystemSummaryDto {
            system_type: RuleSystemType::D100.into(),
            name: "D100 System".to_string(),
            description: "Roll percentile dice under skill value. Used by Call of Cthulhu, RuneQuest, and similar games.".to_string(),
            dice_notation: "1d100".to_string(),
            presets: RuleSystemVariant::variants_for_type(RuleSystemType::D100)
                .into_iter()
                .map(|v| {
                    let config = RuleSystemConfig::from_variant(v.clone());
                    RuleSystemPresetSummaryDto {
                        variant: v.into(),
                        name: config.name,
                        description: config.description,
                    }
                })
                .collect(),
        },
        RuleSystemSummaryDto {
            system_type: RuleSystemType::Narrative.into(),
            name: "Narrative System".to_string(),
            description: "Fiction-first with descriptive outcomes. Used by Kids on Bikes, FATE, PbtA games.".to_string(),
            dice_notation: "Varies".to_string(),
            presets: RuleSystemVariant::variants_for_type(RuleSystemType::Narrative)
                .into_iter()
                .map(|v| {
                    let config = RuleSystemConfig::from_variant(v.clone());
                    RuleSystemPresetSummaryDto {
                        variant: v.into(),
                        name: config.name,
                        description: config.description,
                    }
                })
                .collect(),
        },
        RuleSystemSummaryDto {
            system_type: RuleSystemType::Custom.into(),
            name: "Custom System".to_string(),
            description: "Build your own rule system from scratch with custom dice and mechanics.".to_string(),
            dice_notation: "Custom".to_string(),
            presets: vec![],
        },
    ];

    Json(systems)
}

/// Get details about a specific rule system type
pub async fn get_rule_system(
    Path(system_type): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let system_type = parse_system_type(&system_type).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let (name, description, dice_notation) = match system_type {
        RuleSystemType::D20 => (
            "D20 System",
            "Roll d20 + modifier vs Difficulty Class. Higher is better. Natural 20 is critical success, natural 1 is critical failure.",
            "1d20",
        ),
        RuleSystemType::D100 => (
            "D100 System",
            "Roll percentile dice (d100) and compare to skill value. Roll equal to or under to succeed. Lower rolls are better successes.",
            "1d100",
        ),
        RuleSystemType::Narrative => (
            "Narrative System",
            "Fiction-first systems where outcomes are described rather than strictly calculated. Dice inform the narrative.",
            "Varies by game",
        ),
        RuleSystemType::Custom => (
            "Custom System",
            "Define your own dice mechanics, stats, and success conditions.",
            "Custom",
        ),
    };

    let presets: Vec<RuleSystemPresetSummaryDto> = RuleSystemVariant::variants_for_type(system_type)
        .into_iter()
        .map(|v| {
            let config = RuleSystemConfig::from_variant(v.clone());
            RuleSystemPresetSummaryDto {
                variant: v.into(),
                name: config.name,
                description: config.description,
            }
        })
        .collect();

    Ok(Json(RuleSystemTypeDetailsDto {
        system_type: system_type.into(),
        name: name.to_string(),
        description: description.to_string(),
        dice_notation: dice_notation.to_string(),
        presets,
    }))
}

/// List presets for a rule system type
pub async fn list_presets(
    Path(system_type): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let system_type = parse_system_type(&system_type).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let presets: Vec<RuleSystemPresetDetailsDto> = RuleSystemVariant::variants_for_type(system_type)
        .into_iter()
        .map(|v| RuleSystemPresetDetailsDto {
            variant: v.clone().into(),
            config: RuleSystemConfig::from_variant(v).into(),
        })
        .collect();

    Ok(Json(presets))
}

/// Get a specific preset configuration
pub async fn get_preset(
    Path((system_type, variant)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let _system_type =
        parse_system_type(&system_type).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;
    let variant = parse_variant(&variant).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let config = RuleSystemConfig::from_variant(variant.clone());

    Ok(Json(RuleSystemPresetDetailsDto {
        variant: variant.into(),
        config: config.into(),
    }))
}
