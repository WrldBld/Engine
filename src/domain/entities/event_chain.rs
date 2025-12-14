//! EventChain entity - Connected sequences of narrative events
//!
//! EventChains link multiple NarrativeEvents together to form story arcs
//! with branching paths and progression tracking.

use chrono::{DateTime, Utc};

use crate::domain::value_objects::{ActId, EventChainId, NarrativeEventId, WorldId};

/// A chain of connected narrative events forming a story arc
#[derive(Debug, Clone)]
pub struct EventChain {
    pub id: EventChainId,
    pub world_id: WorldId,

    /// Name of this story arc/chain
    pub name: String,
    /// Description of what this chain represents narratively
    pub description: String,

    /// IDs of events in this chain (ordered by chain_position)
    pub events: Vec<NarrativeEventId>,

    /// Whether this chain is currently active
    pub is_active: bool,
    /// Current position in the chain (index of next event)
    pub current_position: u32,
    /// Events that have been completed in this chain
    pub completed_events: Vec<NarrativeEventId>,

    /// Optional: Act this chain belongs to
    pub act_id: Option<ActId>,

    /// Tags for organization
    pub tags: Vec<String>,

    /// Color for visualization (hex)
    pub color: Option<String>,

    /// Is this a favorite for quick access
    pub is_favorite: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl EventChain {
    pub fn new(world_id: WorldId, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: EventChainId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            events: Vec::new(),
            is_active: true,
            current_position: 0,
            completed_events: Vec::new(),
            act_id: None,
            tags: Vec::new(),
            color: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add an event to the chain
    pub fn add_event(&mut self, event_id: NarrativeEventId) {
        self.events.push(event_id);
        self.updated_at = Utc::now();
    }

    /// Add an event at a specific position
    pub fn insert_event(&mut self, position: usize, event_id: NarrativeEventId) {
        if position <= self.events.len() {
            self.events.insert(position, event_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove an event from the chain
    pub fn remove_event(&mut self, event_id: &NarrativeEventId) -> bool {
        let original_len = self.events.len();
        self.events.retain(|e| e != event_id);
        if self.events.len() != original_len {
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Reorder events in the chain
    pub fn reorder_events(&mut self, event_ids: Vec<NarrativeEventId>) {
        self.events = event_ids;
        self.updated_at = Utc::now();
    }

    /// Mark an event as completed
    pub fn complete_event(&mut self, event_id: NarrativeEventId) {
        if !self.completed_events.contains(&event_id) {
            self.completed_events.push(event_id);
        }

        // Advance current position if this was the current event
        if let Some(pos) = self.events.iter().position(|e| *e == event_id) {
            if pos as u32 == self.current_position {
                self.current_position = (pos as u32) + 1;
            }
        }

        self.updated_at = Utc::now();
    }

    /// Get the current event in the chain (if any)
    pub fn current_event(&self) -> Option<&NarrativeEventId> {
        self.events.get(self.current_position as usize)
    }

    /// Get the next event after current (if any)
    pub fn next_event(&self) -> Option<&NarrativeEventId> {
        self.events.get((self.current_position + 1) as usize)
    }

    /// Check if the chain is complete
    pub fn is_complete(&self) -> bool {
        self.current_position as usize >= self.events.len()
    }

    /// Get progress as a fraction (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.events.is_empty() {
            0.0
        } else {
            self.completed_events.len() as f32 / self.events.len() as f32
        }
    }

    /// Get progress as percentage string
    pub fn progress_string(&self) -> String {
        format!("{}%", (self.progress() * 100.0) as u32)
    }

    /// Get remaining events count
    pub fn remaining_events(&self) -> usize {
        self.events.len() - self.completed_events.len()
    }

    /// Reset the chain to the beginning
    pub fn reset(&mut self) {
        self.current_position = 0;
        self.completed_events.clear();
        self.updated_at = Utc::now();
    }

    /// Deactivate the chain
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }

    /// Activate the chain
    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Utc::now();
    }

    /// Check if an event is in this chain
    pub fn contains_event(&self, event_id: &NarrativeEventId) -> bool {
        self.events.contains(event_id)
    }

    /// Get the position of an event in this chain
    pub fn event_position(&self, event_id: &NarrativeEventId) -> Option<usize> {
        self.events.iter().position(|e| e == event_id)
    }

    /// Check if an event has been completed in this chain
    pub fn is_event_completed(&self, event_id: &NarrativeEventId) -> bool {
        self.completed_events.contains(event_id)
    }
}

/// Summary information about a chain's state
#[derive(Debug, Clone)]
pub struct ChainStatus {
    pub chain_id: EventChainId,
    pub chain_name: String,
    pub is_active: bool,
    pub is_complete: bool,
    pub total_events: usize,
    pub completed_events: usize,
    pub progress_percent: u32,
    pub current_event_id: Option<NarrativeEventId>,
}

impl From<&EventChain> for ChainStatus {
    fn from(chain: &EventChain) -> Self {
        Self {
            chain_id: chain.id,
            chain_name: chain.name.clone(),
            is_active: chain.is_active,
            is_complete: chain.is_complete(),
            total_events: chain.events.len(),
            completed_events: chain.completed_events.len(),
            progress_percent: (chain.progress() * 100.0) as u32,
            current_event_id: chain.current_event().copied(),
        }
    }
}
