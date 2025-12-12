//! World Aggregate - The root aggregate for a game world
//!
//! A World Aggregate contains all entities that belong to a single game world.
//! All modifications to world data should go through this aggregate root
//! to maintain consistency.

use crate::domain::entities::{World, Act, Scene, Character, Location};
use crate::domain::value_objects::{WorldId, ActId, SceneId, CharacterId, LocationId};

/// The World Aggregate Root
///
/// Contains all entities that belong to a world and provides methods
/// to modify them while maintaining consistency invariants.
#[derive(Debug, Clone)]
pub struct WorldAggregate {
    /// The root world entity
    world: World,
    /// Acts within this world (story structure)
    acts: Vec<Act>,
    /// All scenes in the world
    scenes: Vec<Scene>,
    /// All characters in the world
    characters: Vec<Character>,
    /// All locations in the world
    locations: Vec<Location>,
}

impl WorldAggregate {
    /// Create a new WorldAggregate from an existing world
    pub fn new(world: World) -> Self {
        Self {
            world,
            acts: Vec::new(),
            scenes: Vec::new(),
            characters: Vec::new(),
            locations: Vec::new(),
        }
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Get the world ID
    pub fn id(&self) -> &WorldId {
        &self.world.id
    }

    /// Get the world entity
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Get all acts
    pub fn acts(&self) -> &[Act] {
        &self.acts
    }

    /// Get all scenes
    pub fn scenes(&self) -> &[Scene] {
        &self.scenes
    }

    /// Get all characters
    pub fn characters(&self) -> &[Character] {
        &self.characters
    }

    /// Get all locations
    pub fn locations(&self) -> &[Location] {
        &self.locations
    }

    // ========================================================================
    // Finders
    // ========================================================================

    /// Find an act by ID
    pub fn find_act(&self, id: &ActId) -> Option<&Act> {
        self.acts.iter().find(|a| &a.id == id)
    }

    /// Find a scene by ID
    pub fn find_scene(&self, id: &SceneId) -> Option<&Scene> {
        self.scenes.iter().find(|s| &s.id == id)
    }

    /// Find a character by ID
    pub fn find_character(&self, id: &CharacterId) -> Option<&Character> {
        self.characters.iter().find(|c| &c.id == id)
    }

    /// Find a location by ID
    pub fn find_location(&self, id: &LocationId) -> Option<&Location> {
        self.locations.iter().find(|l| &l.id == id)
    }

    // ========================================================================
    // Mutators
    // ========================================================================

    /// Add an act to the world
    pub fn add_act(&mut self, act: Act) {
        self.acts.push(act);
    }

    /// Add a scene to the world
    pub fn add_scene(&mut self, scene: Scene) {
        self.scenes.push(scene);
    }

    /// Add a character to the world
    ///
    /// # Invariants
    /// - Character name must not be empty
    pub fn add_character(&mut self, character: Character) -> Result<(), AggregateError> {
        if character.name.is_empty() {
            return Err(AggregateError::ValidationError(
                "Character name cannot be empty".to_string(),
            ));
        }
        self.characters.push(character);
        Ok(())
    }

    /// Remove a character from the world
    pub fn remove_character(&mut self, id: &CharacterId) -> Option<Character> {
        if let Some(pos) = self.characters.iter().position(|c| &c.id == id) {
            Some(self.characters.remove(pos))
        } else {
            None
        }
    }

    /// Add a location to the world
    ///
    /// # Invariants
    /// - Location name must not be empty
    pub fn add_location(&mut self, location: Location) -> Result<(), AggregateError> {
        if location.name.is_empty() {
            return Err(AggregateError::ValidationError(
                "Location name cannot be empty".to_string(),
            ));
        }
        self.locations.push(location);
        Ok(())
    }

    /// Remove a location from the world
    pub fn remove_location(&mut self, id: &LocationId) -> Option<Location> {
        if let Some(pos) = self.locations.iter().position(|l| &l.id == id) {
            Some(self.locations.remove(pos))
        } else {
            None
        }
    }

    /// Update the world's metadata
    pub fn update_world(&mut self, name: Option<String>, description: Option<String>) {
        if let Some(n) = name {
            self.world.name = n;
        }
        if let Some(d) = description {
            self.world.description = d;
        }
    }

    // ========================================================================
    // Bulk Loading
    // ========================================================================

    /// Load acts into the aggregate (for hydration from persistence)
    pub fn with_acts(mut self, acts: Vec<Act>) -> Self {
        self.acts = acts;
        self
    }

    /// Load scenes into the aggregate (for hydration from persistence)
    pub fn with_scenes(mut self, scenes: Vec<Scene>) -> Self {
        self.scenes = scenes;
        self
    }

    /// Load characters into the aggregate (for hydration from persistence)
    pub fn with_characters(mut self, characters: Vec<Character>) -> Self {
        self.characters = characters;
        self
    }

    /// Load locations into the aggregate (for hydration from persistence)
    pub fn with_locations(mut self, locations: Vec<Location>) -> Self {
        self.locations = locations;
        self
    }
}

/// Errors that can occur when modifying the aggregate
#[derive(Debug, Clone)]
pub enum AggregateError {
    /// A validation rule was violated
    ValidationError(String),
    /// Entity not found
    NotFound(String),
}

impl std::fmt::Display for AggregateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregateError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AggregateError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for AggregateError {}
