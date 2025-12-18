use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::application::ports::outbound::{SettingsRepositoryPort, SettingsError};
use crate::domain::value_objects::{AppSettings, WorldId};

pub struct SettingsService {
    repository: Arc<dyn SettingsRepositoryPort>,
    /// Cache for global settings
    global_cache: RwLock<Option<AppSettings>>,
    /// Cache for per-world settings
    world_cache: RwLock<HashMap<WorldId, AppSettings>>,
}

impl SettingsService {
    pub fn new(repository: Arc<dyn SettingsRepositoryPort>) -> Self {
        Self {
            repository,
            global_cache: RwLock::new(None),
            world_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get global settings (cached)
    pub async fn get(&self) -> AppSettings {
        let cache = self.global_cache.read().await;
        if let Some(settings) = &*cache {
            return settings.clone();
        }
        drop(cache);

        // Load from DB
        match self.repository.get().await {
            Ok(settings) => {
                *self.global_cache.write().await = Some(settings.clone());
                settings
            }
            Err(_) => AppSettings::from_env(),
        }
    }

    /// Update global settings and invalidate cache
    pub async fn update(&self, settings: AppSettings) -> Result<(), SettingsError> {
        self.repository.save(&settings).await?;
        *self.global_cache.write().await = Some(settings);
        // Also invalidate world caches since they inherit from global
        self.world_cache.write().await.clear();
        Ok(())
    }

    /// Reset global settings to env/defaults and clear DB values
    pub async fn reset(&self) -> Result<AppSettings, SettingsError> {
        let settings = self.repository.reset().await?;
        *self.global_cache.write().await = Some(settings.clone());
        // Also invalidate world caches
        self.world_cache.write().await.clear();
        Ok(settings)
    }

    /// Get settings for a specific world (cached)
    pub async fn get_for_world(&self, world_id: WorldId) -> AppSettings {
        // Check cache first
        {
            let cache = self.world_cache.read().await;
            if let Some(settings) = cache.get(&world_id) {
                return settings.clone();
            }
        }

        // Load from DB
        match self.repository.get_for_world(world_id).await {
            Ok(settings) => {
                self.world_cache.write().await.insert(world_id, settings.clone());
                settings
            }
            Err(_) => {
                // Fall back to global settings with world_id set
                let mut settings = self.get().await;
                settings.world_id = Some(world_id);
                settings
            }
        }
    }

    /// Update per-world settings
    pub async fn update_for_world(&self, world_id: WorldId, mut settings: AppSettings) -> Result<(), SettingsError> {
        settings.world_id = Some(world_id);
        self.repository.save_for_world(world_id, &settings).await?;
        self.world_cache.write().await.insert(world_id, settings);
        Ok(())
    }

    /// Reset per-world settings to global defaults
    pub async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        let settings = self.repository.reset_for_world(world_id).await?;
        self.world_cache.write().await.insert(world_id, settings.clone());
        Ok(settings)
    }

    /// Delete per-world settings when world is deleted
    pub async fn delete_for_world(&self, world_id: WorldId) -> Result<(), SettingsError> {
        self.repository.delete_for_world(world_id).await?;
        self.world_cache.write().await.remove(&world_id);
        Ok(())
    }

    /// Invalidate all caches (useful when settings are modified externally)
    pub async fn invalidate_cache(&self) {
        *self.global_cache.write().await = None;
        self.world_cache.write().await.clear();
    }
}
