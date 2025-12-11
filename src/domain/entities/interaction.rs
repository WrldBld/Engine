//! Interaction template entity
//!
//! Defines available interactions within a scene that players can perform.

use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{CharacterId, InteractionId, ItemId, SceneId};

/// A template defining an available interaction within a scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionTemplate {
    pub id: InteractionId,
    pub scene_id: SceneId,
    pub name: String,
    pub interaction_type: InteractionType,
    pub target: InteractionTarget,
    /// Hints for the LLM on how to handle this interaction
    pub prompt_hints: String,
    /// What tools the LLM is allowed to call for this interaction
    pub allowed_tools: Vec<String>,
    /// Conditions that must be met to show this interaction
    pub conditions: Vec<InteractionCondition>,
    /// Whether this interaction is currently available
    pub is_available: bool,
    /// Display order in the UI
    pub order: u32,
}

impl InteractionTemplate {
    pub fn new(
        scene_id: SceneId,
        name: impl Into<String>,
        interaction_type: InteractionType,
        target: InteractionTarget,
    ) -> Self {
        Self {
            id: InteractionId::new(),
            scene_id,
            name: name.into(),
            interaction_type,
            target,
            prompt_hints: String::new(),
            allowed_tools: Vec::new(),
            conditions: Vec::new(),
            is_available: true,
            order: 0,
        }
    }

    pub fn with_prompt_hints(mut self, hints: impl Into<String>) -> Self {
        self.prompt_hints = hints.into();
        self
    }

    pub fn with_allowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.allowed_tools.push(tool.into());
        self
    }

    pub fn with_condition(mut self, condition: InteractionCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.is_available = false;
        self
    }
}

/// Types of interactions players can perform
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InteractionType {
    /// Talk to an NPC
    Dialogue,
    /// Examine something in the scene
    Examine,
    /// Use an item from inventory
    UseItem,
    /// Pick up an item
    PickUp,
    /// Give an item to someone
    GiveItem,
    /// Attack (initiates combat or hostile action)
    Attack,
    /// Move to another location
    Travel,
    /// Custom interaction type
    Custom(String),
}

/// What the interaction targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionTarget {
    /// Target a specific character
    Character(CharacterId),
    /// Target a specific item
    Item(ItemId),
    /// Target something in the environment (described by string)
    Environment(String),
    /// No specific target (general action)
    None,
}

/// Conditions for an interaction to be available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionCondition {
    /// Player must have this item
    HasItem(ItemId),
    /// A specific character must be present in the scene
    CharacterPresent(CharacterId),
    /// A relationship must exist between player and target
    HasRelationship {
        with_character: CharacterId,
        relationship_type: Option<String>,
    },
    /// A game flag must be set
    FlagSet(String),
    /// A game flag must not be set
    FlagNotSet(String),
    /// Custom condition (evaluated by game logic)
    Custom(String),
}
