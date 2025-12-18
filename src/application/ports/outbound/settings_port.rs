use async_trait::async_trait;
use crate::domain::value_objects::{AppSettings, WorldId};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Settings not found for world: {0}")]
    NotFound(String),
}

#[async_trait]
pub trait SettingsRepositoryPort: Send + Sync {
    /// Get global settings (no world_id)
    async fn get(&self) -> Result<AppSettings, SettingsError>;
    
    /// Save global settings
    async fn save(&self, settings: &AppSettings) -> Result<(), SettingsError>;
    
    /// Reset global settings to env/defaults
    async fn reset(&self) -> Result<AppSettings, SettingsError>;
    
    /// Get settings for a specific world, falling back to global if not found
    async fn get_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError>;
    
    /// Save per-world settings
    async fn save_for_world(&self, world_id: WorldId, settings: &AppSettings) -> Result<(), SettingsError>;
    
    /// Reset per-world settings (removes world-specific overrides, falls back to global)
    async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError>;
    
    /// Delete per-world settings when world is deleted
    async fn delete_for_world(&self, world_id: WorldId) -> Result<(), SettingsError>;
}
