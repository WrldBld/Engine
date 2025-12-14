use serde::{Deserialize, Serialize};

use crate::domain::entities::{ChainStatus, EventChain};

/// Request to create an event chain.
#[derive(Debug, Deserialize)]
pub struct CreateEventChainRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub act_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub is_active: bool,
}

/// Request to update an event chain.
#[derive(Debug, Deserialize)]
pub struct UpdateEventChainRequestDto {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub events: Option<Vec<String>>,
    #[serde(default)]
    pub act_id: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Request to add an event to a chain.
#[derive(Debug, Deserialize)]
pub struct AddEventRequestDto {
    pub event_id: String,
    #[serde(default)]
    pub position: Option<usize>,
}

/// Event chain response.
#[derive(Debug, Serialize)]
pub struct EventChainResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub current_position: u32,
    pub completed_events: Vec<String>,
    pub act_id: Option<String>,
    pub tags: Vec<String>,
    pub color: Option<String>,
    pub is_favorite: bool,
    pub progress_percent: u32,
    pub is_complete: bool,
    pub remaining_events: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl From<EventChain> for EventChainResponseDto {
    fn from(c: EventChain) -> Self {
        let progress_percent = (c.progress() * 100.0) as u32;
        let is_complete = c.is_complete();
        let remaining_events = c.remaining_events();

        Self {
            id: c.id.to_string(),
            world_id: c.world_id.to_string(),
            name: c.name,
            description: c.description,
            events: c.events.iter().map(|e| e.to_string()).collect(),
            is_active: c.is_active,
            current_position: c.current_position,
            completed_events: c.completed_events.iter().map(|e| e.to_string()).collect(),
            act_id: c.act_id.map(|a| a.to_string()),
            tags: c.tags,
            color: c.color,
            is_favorite: c.is_favorite,
            progress_percent,
            is_complete,
            remaining_events,
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

/// Chain status response.
#[derive(Debug, Serialize)]
pub struct ChainStatusResponseDto {
    pub chain_id: String,
    pub chain_name: String,
    pub is_active: bool,
    pub is_complete: bool,
    pub total_events: usize,
    pub completed_events: usize,
    pub progress_percent: u32,
    pub current_event_id: Option<String>,
}

impl From<ChainStatus> for ChainStatusResponseDto {
    fn from(s: ChainStatus) -> Self {
        Self {
            chain_id: s.chain_id.to_string(),
            chain_name: s.chain_name,
            is_active: s.is_active,
            is_complete: s.is_complete,
            total_events: s.total_events,
            completed_events: s.completed_events,
            progress_percent: s.progress_percent,
            current_event_id: s.current_event_id.map(|e| e.to_string()),
        }
    }
}

