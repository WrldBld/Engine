use serde::{Deserialize, Serialize};

use crate::domain::entities::{EventChainMembership, FeaturedNpc, NarrativeEvent};
use crate::domain::value_objects::{ActId, LocationId, SceneId};

/// Query parameters for listing narrative events.
#[derive(Debug, Deserialize)]
pub struct ListNarrativeEventsQueryDto {
    #[serde(default)]
    pub act_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub location_id: Option<String>,
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

/// Narrative event response - simplified view for API (list view, no edge data).
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
            // NOTE: scene_id, location_id, act_id, chain_id, chain_position, featured_npcs
            // are now stored as graph edges - use NarrativeEventDetailResponseDto for full data
            outcome_count: e.outcomes.len(),
            trigger_condition_count: e.trigger_conditions.len(),
            created_at: e.created_at.to_rfc3339(),
            updated_at: e.updated_at.to_rfc3339(),
        }
    }
}

/// Detailed narrative event response - includes edge data for single-event view.
#[derive(Debug, Serialize)]
pub struct NarrativeEventDetailResponseDto {
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
    // Edge data (fetched separately)
    pub scene_id: Option<String>,
    pub location_id: Option<String>,
    pub act_id: Option<String>,
    pub chain_memberships: Vec<ChainMembershipDto>,
    pub featured_npcs: Vec<FeaturedNpcDto>,
    pub outcome_count: usize,
    pub trigger_condition_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Chain membership info for a narrative event.
#[derive(Debug, Serialize)]
pub struct ChainMembershipDto {
    pub chain_id: String,
    pub position: u32,
    pub is_completed: bool,
}

impl From<EventChainMembership> for ChainMembershipDto {
    fn from(m: EventChainMembership) -> Self {
        Self {
            chain_id: m.chain_id.to_string(),
            position: m.position,
            is_completed: m.is_completed,
        }
    }
}

/// Featured NPC info for a narrative event.
#[derive(Debug, Serialize)]
pub struct FeaturedNpcDto {
    pub character_id: String,
    pub role: Option<String>,
}

impl From<FeaturedNpc> for FeaturedNpcDto {
    fn from(npc: FeaturedNpc) -> Self {
        Self {
            character_id: npc.character_id.to_string(),
            role: npc.role,
        }
    }
}

impl NarrativeEventDetailResponseDto {
    /// Create a detail response from the event entity and its edge data.
    pub fn new(
        event: NarrativeEvent,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        act_id: Option<ActId>,
        chain_memberships: Vec<EventChainMembership>,
        featured_npcs: Vec<FeaturedNpc>,
    ) -> Self {
        Self {
            id: event.id.to_string(),
            world_id: event.world_id.to_string(),
            name: event.name,
            description: event.description,
            scene_direction: event.scene_direction,
            suggested_opening: event.suggested_opening,
            trigger_count: event.trigger_count,
            is_active: event.is_active,
            is_triggered: event.is_triggered,
            triggered_at: event.triggered_at.map(|t| t.to_rfc3339()),
            selected_outcome: event.selected_outcome,
            is_repeatable: event.is_repeatable,
            delay_turns: event.delay_turns,
            expires_after_turns: event.expires_after_turns,
            priority: event.priority,
            is_favorite: event.is_favorite,
            tags: event.tags,
            scene_id: scene_id.map(|s| s.to_string()),
            location_id: location_id.map(|l| l.to_string()),
            act_id: act_id.map(|a| a.to_string()),
            chain_memberships: chain_memberships.into_iter().map(|m| m.into()).collect(),
            featured_npcs: featured_npcs.into_iter().map(|n| n.into()).collect(),
            outcome_count: event.outcomes.len(),
            trigger_condition_count: event.trigger_conditions.len(),
            created_at: event.created_at.to_rfc3339(),
            updated_at: event.updated_at.to_rfc3339(),
        }
    }
}

