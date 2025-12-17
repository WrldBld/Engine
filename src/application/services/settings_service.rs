use std::sync::Arc;
use tokio::sync::RwLock;
use crate::application::ports::outbound::{SettingsRepositoryPort, SettingsError};
use crate::domain::value_objects::AppSettings;

pub struct SettingsService {
    repository: Arc<dyn SettingsRepositoryPort>,
    cache: RwLock<Option<AppSettings>>,
}

impl SettingsService {
    pub fn new(repository: Arc<dyn SettingsRepositoryPort>) -> Self {
        Self {
            repository,
            cache: RwLock::new(None),
        }
    }

    /// Get current settings (cached)
    pub async fn get(&self) -> AppSettings {
        let cache = self.cache.read().await;
        if let Some(settings) = &*cache {
            return settings.clone();
        }
        drop(cache);

        // Load from DB
        match self.repository.get().await {
            Ok(settings) => {
                *self.cache.write().await = Some(settings.clone());
                settings
            }
            Err(_) => AppSettings::from_env(),
        }
    }

    /// Update settings and invalidate cache
    pub async fn update(&self, settings: AppSettings) -> Result<(), SettingsError> {
        self.repository.save(&settings).await?;
        *self.cache.write().await = Some(settings);
        Ok(())
    }

    /// Reset to env/defaults and clear DB values
    pub async fn reset(&self) -> Result<AppSettings, SettingsError> {
        let settings = self.repository.reset().await?;
        *self.cache.write().await = Some(settings.clone());
        Ok(settings)
    }
}
