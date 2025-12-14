use serde::{Deserialize, Serialize};

use crate::domain::entities::NarrativeEvent;

/// Query parameters for listing narrative events.
#[derive(Debug, Deserialize)]
pub struct ListNarrativeEventsQueryDto {
    #[serde(default)]
    pub act_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
}

/// Request to create a narrative event.
#[derive(Debug, Deserialize)]
pub struct CreateNarrativeEventRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scene_direction: String,
    #[serde(default)]
    pub suggested_opening: Option<String>,
    #[serde(default)]
    pub is_repeatable: bool,
    #[serde(default)]
    pub delay_turns: u32,
    #[serde(default)]
    pub expires_after_turns: Option<u32>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Request to update a narrative event.
#[derive(Debug, Deserialize)]
pub struct UpdateNarrativeEventRequestDto {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scene_direction: Option<String>,
    #[serde(default)]
    pub suggested_opening: Option<String>,
    #[serde(default)]
    pub is_repeatable: Option<bool>,
    #[serde(default)]
    pub delay_turns: Option<u32>,
    #[serde(default)]
    pub expires_after_turns: Option<u32>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Narrative event response - simplified view for API.
#[derive(Debug, Serialize)]
pub struct NarrativeEventResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub scene_direction: String,
    pub suggested_opening: Option<String>,
    pub trigger_count: u32,
    pub is_active: bool,
    pub is_triggered: bool,
    pub triggered_at: Option<String>,
    pub selected_outcome: Option<String>,
    pub is_repeatable: bool,
    pub delay_turns: u32,
    pub expires_after_turns: Option<u32>,
    pub priority: i32,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub scene_id: Option<String>,
    pub location_id: Option<String>,
    pub act_id: Option<String>,
    pub chain_id: Option<String>,
    pub chain_position: Option<u32>,
    pub outcome_count: usize,
    pub trigger_condition_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl From<NarrativeEvent> for NarrativeEventResponseDto {
    fn from(e: NarrativeEvent) -> Self {
        Self {
            id: e.id.to_string(),
            world_id: e.world_id.to_string(),
            name: e.name,
            description: e.description,
            scene_direction: e.scene_direction,
            suggested_opening: e.suggested_opening,
            trigger_count: e.trigger_count,
            is_active: e.is_active,
            is_triggered: e.is_triggered,
            triggered_at: e.triggered_at.map(|t| t.to_rfc3339()),
            selected_outcome: e.selected_outcome,
            is_repeatable: e.is_repeatable,
            delay_turns: e.delay_turns,
            expires_after_turns: e.expires_after_turns,
            priority: e.priority,
            is_favorite: e.is_favorite,
            tags: e.tags,
            scene_id: e.scene_id.map(|s| s.to_string()),
            location_id: e.location_id.map(|l| l.to_string()),
            act_id: e.act_id.map(|a| a.to_string()),
            chain_id: e.chain_id.map(|c| c.to_string()),
            chain_position: e.chain_position,
            outcome_count: e.outcomes.len(),
            trigger_condition_count: e.trigger_conditions.len(),
            created_at: e.created_at.to_rfc3339(),
            updated_at: e.updated_at.to_rfc3339(),
        }
    }
}

