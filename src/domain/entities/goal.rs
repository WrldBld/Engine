//! Goal entity - Abstract want targets
//!
//! # Graph-First Design (Phase 0.C)
//!
//! A Goal is an abstract desire target for Wants that don't target a specific
//! Character or Item. Examples: "power", "revenge", "peace", "recognition".
//!
//! ```cypher
//! (world:World)-[:CONTAINS_GOAL]->(goal:Goal)
//! (want:Want)-[:TARGETS]->(goal:Goal)
//! ```

use crate::domain::value_objects::{GoalId, WorldId};

/// Abstract desire target (for Wants that don't target a Character or Item)
#[derive(Debug, Clone)]
pub struct Goal {
    pub id: GoalId,
    pub world_id: WorldId,
    /// Name of the goal (e.g., "Power", "Revenge", "Peace")
    pub name: String,
    /// Optional description of what this goal means
    pub description: Option<String>,
}

impl Goal {
    pub fn new(world_id: WorldId, name: impl Into<String>) -> Self {
        Self {
            id: GoalId::new(),
            world_id,
            name: name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
