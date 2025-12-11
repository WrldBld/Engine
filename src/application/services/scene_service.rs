//! Scene Service - Application service for scene management
//!
//! This service provides use case implementations for creating, updating,
//! and managing scenes, including character assignment and directorial notes.

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, info, instrument};

use crate::domain::entities::{Character, Location, Scene, SceneCondition, TimeContext};
use crate::domain::value_objects::{ActId, CharacterId, LocationId, SceneId};
use crate::infrastructure::persistence::Neo4jRepository;

/// Request to create a new scene
#[derive(Debug, Clone)]
pub struct CreateSceneRequest {
    pub act_id: ActId,
    pub name: String,
    pub location_id: LocationId,
    pub time_context: Option<TimeContext>,
    pub backdrop_override: Option<String>,
    pub featured_characters: Vec<CharacterId>,
    pub directorial_notes: Option<String>,
    pub entry_conditions: Vec<SceneCondition>,
    pub order: u32,
}

/// Request to update an existing scene
#[derive(Debug, Clone)]
pub struct UpdateSceneRequest {
    pub name: Option<String>,
    pub time_context: Option<TimeContext>,
    pub backdrop_override: Option<String>,
    pub entry_conditions: Option<Vec<SceneCondition>>,
    pub order: Option<u32>,
}

/// Scene with all related data
#[derive(Debug, Clone)]
pub struct SceneWithRelations {
    pub scene: Scene,
    pub location: Location,
    pub featured_characters: Vec<Character>,
}

/// Scene service trait defining the application use cases
#[async_trait]
pub trait SceneService: Send + Sync {
    /// Create a new scene with character assignment
    async fn create_scene(&self, request: CreateSceneRequest) -> Result<Scene>;

    /// Get a scene by ID
    async fn get_scene(&self, id: SceneId) -> Result<Option<Scene>>;

    /// Get a scene with all related data (location, characters)
    async fn get_scene_with_relations(&self, id: SceneId) -> Result<Option<SceneWithRelations>>;

    /// Update a scene with validation
    async fn update_scene(&self, id: SceneId, request: UpdateSceneRequest) -> Result<Scene>;

    /// Delete a scene
    async fn delete_scene(&self, id: SceneId) -> Result<()>;

    /// Update directorial notes for a scene
    async fn update_directorial_notes(&self, id: SceneId, notes: String) -> Result<Scene>;

    /// List all scenes in an act (ordered)
    async fn list_scenes_by_act(&self, act_id: ActId) -> Result<Vec<Scene>>;

    /// List all scenes at a location
    async fn list_scenes_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>>;

    /// Add a character to a scene
    async fn add_character(&self, scene_id: SceneId, character_id: CharacterId) -> Result<Scene>;

    /// Remove a character from a scene
    async fn remove_character(&self, scene_id: SceneId, character_id: CharacterId) -> Result<Scene>;

    /// Update featured characters for a scene
    async fn update_featured_characters(
        &self,
        scene_id: SceneId,
        character_ids: Vec<CharacterId>,
    ) -> Result<Scene>;

    /// Add an entry condition to a scene
    async fn add_entry_condition(
        &self,
        scene_id: SceneId,
        condition: SceneCondition,
    ) -> Result<Scene>;
}

/// Default implementation of SceneService using Neo4j repository
pub struct SceneServiceImpl {
    repository: Neo4jRepository,
}

impl SceneServiceImpl {
    /// Create a new SceneServiceImpl with the given repository
    pub fn new(repository: Neo4jRepository) -> Self {
        Self { repository }
    }

    /// Validate a scene creation request
    fn validate_create_request(request: &CreateSceneRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            anyhow::bail!("Scene name cannot be empty");
        }
        if request.name.len() > 255 {
            anyhow::bail!("Scene name cannot exceed 255 characters");
        }
        if let Some(ref notes) = request.directorial_notes {
            if notes.len() > 10000 {
                anyhow::bail!("Directorial notes cannot exceed 10000 characters");
            }
        }
        Ok(())
    }

    /// Validate a scene update request
    fn validate_update_request(request: &UpdateSceneRequest) -> Result<()> {
        if let Some(ref name) = request.name {
            if name.trim().is_empty() {
                anyhow::bail!("Scene name cannot be empty");
            }
            if name.len() > 255 {
                anyhow::bail!("Scene name cannot exceed 255 characters");
            }
        }
        Ok(())
    }
}

#[async_trait]
impl SceneService for SceneServiceImpl {
    #[instrument(skip(self), fields(act_id = %request.act_id, name = %request.name))]
    async fn create_scene(&self, request: CreateSceneRequest) -> Result<Scene> {
        Self::validate_create_request(&request)?;

        // Verify the location exists
        let _ = self
            .repository
            .locations()
            .get(request.location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", request.location_id))?;

        // Verify all featured characters exist
        for char_id in &request.featured_characters {
            let _ = self
                .repository
                .characters()
                .get(*char_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Character not found: {}", char_id))?;
        }

        let mut scene = Scene::new(request.act_id, &request.name, request.location_id);

        if let Some(time_context) = request.time_context {
            scene = scene.with_time(time_context);
        }

        if let Some(notes) = request.directorial_notes {
            scene = scene.with_directorial_notes(notes);
        }

        scene.backdrop_override = request.backdrop_override;
        scene.featured_characters = request.featured_characters;
        scene.entry_conditions = request.entry_conditions;
        scene.order = request.order;

        self.repository
            .scenes()
            .create(&scene)
            .await
            .context("Failed to create scene in repository")?;

        info!(scene_id = %scene.id, "Created scene: {} in act {}", scene.name, request.act_id);
        Ok(scene)
    }

    #[instrument(skip(self))]
    async fn get_scene(&self, id: SceneId) -> Result<Option<Scene>> {
        debug!(scene_id = %id, "Fetching scene");
        self.repository
            .scenes()
            .get(id)
            .await
            .context("Failed to get scene from repository")
    }

    #[instrument(skip(self))]
    async fn get_scene_with_relations(&self, id: SceneId) -> Result<Option<SceneWithRelations>> {
        debug!(scene_id = %id, "Fetching scene with relations");

        let scene = match self.repository.scenes().get(id).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        // Load the location
        let location = self
            .repository
            .locations()
            .get(scene.location_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Location not found for scene: {}", scene.location_id)
            })?;

        // Load all featured characters
        let mut featured_characters = Vec::new();
        for char_id in &scene.featured_characters {
            if let Some(character) = self.repository.characters().get(*char_id).await? {
                featured_characters.push(character);
            }
        }

        Ok(Some(SceneWithRelations {
            scene,
            location,
            featured_characters,
        }))
    }

    #[instrument(skip(self), fields(scene_id = %id))]
    async fn update_scene(&self, id: SceneId, request: UpdateSceneRequest) -> Result<Scene> {
        Self::validate_update_request(&request)?;

        let mut scene = self
            .repository
            .scenes()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene not found: {}", id))?;

        if let Some(name) = request.name {
            scene.name = name;
        }
        if let Some(time_context) = request.time_context {
            scene.time_context = time_context;
        }
        if request.backdrop_override.is_some() {
            scene.backdrop_override = request.backdrop_override;
        }
        if let Some(conditions) = request.entry_conditions {
            scene.entry_conditions = conditions;
        }
        if let Some(order) = request.order {
            scene.order = order;
        }

        self.repository
            .scenes()
            .update(&scene)
            .await
            .context("Failed to update scene in repository")?;

        info!(scene_id = %id, "Updated scene: {}", scene.name);
        Ok(scene)
    }

    #[instrument(skip(self))]
    async fn delete_scene(&self, id: SceneId) -> Result<()> {
        let scene = self
            .repository
            .scenes()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene not found: {}", id))?;

        self.repository
            .scenes()
            .delete(id)
            .await
            .context("Failed to delete scene from repository")?;

        info!(scene_id = %id, "Deleted scene: {}", scene.name);
        Ok(())
    }

    #[instrument(skip(self), fields(scene_id = %id))]
    async fn update_directorial_notes(&self, id: SceneId, notes: String) -> Result<Scene> {
        if notes.len() > 10000 {
            anyhow::bail!("Directorial notes cannot exceed 10000 characters");
        }

        self.repository
            .scenes()
            .update_directorial_notes(id, &notes)
            .await
            .context("Failed to update directorial notes")?;

        // Fetch and return the updated scene
        self.repository
            .scenes()
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene not found after update: {}", id))
    }

    #[instrument(skip(self))]
    async fn list_scenes_by_act(&self, act_id: ActId) -> Result<Vec<Scene>> {
        debug!(act_id = %act_id, "Listing scenes by act");
        self.repository
            .scenes()
            .list_by_act(act_id)
            .await
            .context("Failed to list scenes by act")
    }

    #[instrument(skip(self))]
    async fn list_scenes_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>> {
        debug!(location_id = %location_id, "Listing scenes by location");
        self.repository
            .scenes()
            .list_by_location(location_id)
            .await
            .context("Failed to list scenes by location")
    }

    #[instrument(skip(self), fields(scene_id = %scene_id, character_id = %character_id))]
    async fn add_character(&self, scene_id: SceneId, character_id: CharacterId) -> Result<Scene> {
        // Verify the character exists
        let _ = self
            .repository
            .characters()
            .get(character_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", character_id))?;

        let mut scene = self
            .repository
            .scenes()
            .get(scene_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene not found: {}", scene_id))?;

        // Add if not already present
        if !scene.featured_characters.contains(&character_id) {
            scene.featured_characters.push(character_id);

            self.repository
                .scenes()
                .update(&scene)
                .await
                .context("Failed to update scene with new character")?;

            debug!(scene_id = %scene_id, character_id = %character_id, "Added character to scene");
        }

        Ok(scene)
    }

    #[instrument(skip(self), fields(scene_id = %scene_id, character_id = %character_id))]
    async fn remove_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<Scene> {
        let mut scene = self
            .repository
            .scenes()
            .get(scene_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene not found: {}", scene_id))?;

        if let Some(pos) = scene
            .featured_characters
            .iter()
            .position(|id| *id == character_id)
        {
            scene.featured_characters.remove(pos);

            self.repository
                .scenes()
                .update(&scene)
                .await
                .context("Failed to update scene after removing character")?;

            debug!(scene_id = %scene_id, character_id = %character_id, "Removed character from scene");
        }

        Ok(scene)
    }

    #[instrument(skip(self, character_ids), fields(scene_id = %scene_id, character_count = character_ids.len()))]
    async fn update_featured_characters(
        &self,
        scene_id: SceneId,
        character_ids: Vec<CharacterId>,
    ) -> Result<Scene> {
        // Verify all characters exist
        for char_id in &character_ids {
            let _ = self
                .repository
                .characters()
                .get(*char_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Character not found: {}", char_id))?;
        }

        let mut scene = self
            .repository
            .scenes()
            .get(scene_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene not found: {}", scene_id))?;

        scene.featured_characters = character_ids;

        self.repository
            .scenes()
            .update(&scene)
            .await
            .context("Failed to update scene featured characters")?;

        info!(scene_id = %scene_id, "Updated featured characters for scene");
        Ok(scene)
    }

    #[instrument(skip(self, condition), fields(scene_id = %scene_id))]
    async fn add_entry_condition(
        &self,
        scene_id: SceneId,
        condition: SceneCondition,
    ) -> Result<Scene> {
        let mut scene = self
            .repository
            .scenes()
            .get(scene_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene not found: {}", scene_id))?;

        scene.entry_conditions.push(condition);

        self.repository
            .scenes()
            .update(&scene)
            .await
            .context("Failed to update scene with new entry condition")?;

        debug!(scene_id = %scene_id, "Added entry condition to scene");
        Ok(scene)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_scene_request_validation() {
        // Empty name should fail
        let request = CreateSceneRequest {
            act_id: ActId::new(),
            name: "".to_string(),
            location_id: LocationId::new(),
            time_context: None,
            backdrop_override: None,
            featured_characters: vec![],
            directorial_notes: None,
            entry_conditions: vec![],
            order: 0,
        };
        assert!(SceneServiceImpl::validate_create_request(&request).is_err());

        // Valid request should pass
        let request = CreateSceneRequest {
            act_id: ActId::new(),
            name: "Opening Scene".to_string(),
            location_id: LocationId::new(),
            time_context: None,
            backdrop_override: None,
            featured_characters: vec![],
            directorial_notes: None,
            entry_conditions: vec![],
            order: 0,
        };
        assert!(SceneServiceImpl::validate_create_request(&request).is_ok());
    }

    #[test]
    fn test_update_scene_request_validation() {
        // Empty name should fail
        let request = UpdateSceneRequest {
            name: Some("".to_string()),
            time_context: None,
            backdrop_override: None,
            entry_conditions: None,
            order: None,
        };
        assert!(SceneServiceImpl::validate_update_request(&request).is_err());

        // No updates is valid
        let request = UpdateSceneRequest {
            name: None,
            time_context: None,
            backdrop_override: None,
            entry_conditions: None,
            order: None,
        };
        assert!(SceneServiceImpl::validate_update_request(&request).is_ok());
    }
}
