//! Rule System API routes
//!
//! Provides endpoints for listing available rule systems and their presets.

use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::domain::value_objects::{RuleSystemConfig, RuleSystemType, RuleSystemVariant};

/// Summary of a rule system type
#[derive(Debug, Serialize)]
struct RuleSystemSummary {
    system_type: RuleSystemType,
    name: String,
    description: String,
    dice_notation: String,
    presets: Vec<PresetSummary>,
}

/// Summary of a preset
#[derive(Debug, Serialize)]
struct PresetSummary {
    variant: RuleSystemVariant,
    name: String,
    description: String,
}

/// Full preset details
#[derive(Debug, Serialize)]
struct PresetDetails {
    variant: RuleSystemVariant,
    config: RuleSystemConfig,
}

/// List all available rule system types
pub async fn list_rule_systems() -> impl IntoResponse {
    let systems = vec![
        RuleSystemSummary {
            system_type: RuleSystemType::D20,
            name: "D20 System".to_string(),
            description: "Roll d20 + modifier vs Difficulty Class. Used by D&D, Pathfinder, and similar games.".to_string(),
            dice_notation: "1d20".to_string(),
            presets: RuleSystemVariant::variants_for_type(RuleSystemType::D20)
                .into_iter()
                .map(|v| {
                    let config = RuleSystemConfig::from_variant(v.clone());
                    PresetSummary {
                        variant: v,
                        name: config.name,
                        description: config.description,
                    }
                })
                .collect(),
        },
        RuleSystemSummary {
            system_type: RuleSystemType::D100,
            name: "D100 System".to_string(),
            description: "Roll percentile dice under skill value. Used by Call of Cthulhu, RuneQuest, and similar games.".to_string(),
            dice_notation: "1d100".to_string(),
            presets: RuleSystemVariant::variants_for_type(RuleSystemType::D100)
                .into_iter()
                .map(|v| {
                    let config = RuleSystemConfig::from_variant(v.clone());
                    PresetSummary {
                        variant: v,
                        name: config.name,
                        description: config.description,
                    }
                })
                .collect(),
        },
        RuleSystemSummary {
            system_type: RuleSystemType::Narrative,
            name: "Narrative System".to_string(),
            description: "Fiction-first with descriptive outcomes. Used by Kids on Bikes, FATE, PbtA games.".to_string(),
            dice_notation: "Varies".to_string(),
            presets: RuleSystemVariant::variants_for_type(RuleSystemType::Narrative)
                .into_iter()
                .map(|v| {
                    let config = RuleSystemConfig::from_variant(v.clone());
                    PresetSummary {
                        variant: v,
                        name: config.name,
                        description: config.description,
                    }
                })
                .collect(),
        },
        RuleSystemSummary {
            system_type: RuleSystemType::Custom,
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
    let system_type = parse_system_type(&system_type)?;

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

    let presets: Vec<PresetSummary> = RuleSystemVariant::variants_for_type(system_type)
        .into_iter()
        .map(|v| {
            let config = RuleSystemConfig::from_variant(v.clone());
            PresetSummary {
                variant: v,
                name: config.name,
                description: config.description,
            }
        })
        .collect();

    Ok(Json(serde_json::json!({
        "system_type": system_type,
        "name": name,
        "description": description,
        "dice_notation": dice_notation,
        "presets": presets,
    })))
}

/// List presets for a rule system type
pub async fn list_presets(
    Path(system_type): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let system_type = parse_system_type(&system_type)?;

    let presets: Vec<PresetDetails> = RuleSystemVariant::variants_for_type(system_type)
        .into_iter()
        .map(|v| PresetDetails {
            variant: v.clone(),
            config: RuleSystemConfig::from_variant(v),
        })
        .collect();

    Ok(Json(presets))
}

/// Get a specific preset configuration
pub async fn get_preset(
    Path((system_type, variant)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let _system_type = parse_system_type(&system_type)?;
    let variant = parse_variant(&variant)?;

    let config = RuleSystemConfig::from_variant(variant.clone());

    Ok(Json(PresetDetails { variant, config }))
}

/// Parse a system type from URL path
fn parse_system_type(s: &str) -> Result<RuleSystemType, (StatusCode, String)> {
    match s.to_lowercase().as_str() {
        "d20" => Ok(RuleSystemType::D20),
        "d100" => Ok(RuleSystemType::D100),
        "narrative" => Ok(RuleSystemType::Narrative),
        "custom" => Ok(RuleSystemType::Custom),
        _ => Err((
            StatusCode::BAD_REQUEST,
            format!("Unknown rule system type: {}. Valid types: d20, d100, narrative, custom", s),
        )),
    }
}

/// Parse a variant from URL path
fn parse_variant(s: &str) -> Result<RuleSystemVariant, (StatusCode, String)> {
    match s.to_lowercase().replace("-", "_").as_str() {
        "dnd5e" | "dnd_5e" => Ok(RuleSystemVariant::Dnd5e),
        "pathfinder2e" | "pathfinder_2e" => Ok(RuleSystemVariant::Pathfinder2e),
        "generic_d20" | "genericd20" => Ok(RuleSystemVariant::GenericD20),
        "coc7e" | "coc_7e" | "callofcthulhu7e" | "call_of_cthulhu_7e" => Ok(RuleSystemVariant::CallOfCthulhu7e),
        "runequest" | "rune_quest" => Ok(RuleSystemVariant::RuneQuest),
        "generic_d100" | "genericd100" => Ok(RuleSystemVariant::GenericD100),
        "kidsonbikes" | "kids_on_bikes" => Ok(RuleSystemVariant::KidsOnBikes),
        "fatecore" | "fate_core" | "fate" => Ok(RuleSystemVariant::FateCore),
        "pbta" | "poweredbyapocalypse" | "powered_by_apocalypse" => Ok(RuleSystemVariant::PoweredByApocalypse),
        _ => Err((
            StatusCode::BAD_REQUEST,
            format!("Unknown variant: {}. Valid variants: dnd5e, pathfinder2e, generic_d20, coc7e, runequest, generic_d100, kidsonbikes, fatecore, pbta", s),
        )),
    }
}
