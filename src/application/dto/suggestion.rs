use serde::{Deserialize, Serialize};

use crate::application::services::{SuggestionContext, SuggestionType};

/// Request body for suggestion endpoints.
#[derive(Debug, Deserialize)]
pub struct SuggestionRequestDto {
    /// Type of entity (e.g., "character", "location", "tavern", "forest")
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Name of the entity (if already set)
    #[serde(default)]
    pub entity_name: Option<String>,
    /// World/setting name or type
    #[serde(default)]
    pub world_setting: Option<String>,
    /// Hints or keywords to guide generation
    #[serde(default)]
    pub hints: Option<String>,
    /// Additional context from other fields
    #[serde(default)]
    pub additional_context: Option<String>,
}

impl From<SuggestionRequestDto> for SuggestionContext {
    fn from(req: SuggestionRequestDto) -> Self {
        SuggestionContext {
            entity_type: req.entity_type,
            entity_name: req.entity_name,
            world_setting: req.world_setting,
            hints: req.hints,
            additional_context: req.additional_context,
        }
    }
}

/// Unified suggestion endpoint request - uses `suggestion_type` in body.
#[derive(Debug, Deserialize)]
pub struct UnifiedSuggestionRequestDto {
    /// Type of suggestion to generate
    pub suggestion_type: SuggestionType,
    /// World ID for routing the response back to the correct clients
    pub world_id: String,
    /// Context for the suggestion
    #[serde(flatten)]
    pub context: SuggestionRequestDto,
}

