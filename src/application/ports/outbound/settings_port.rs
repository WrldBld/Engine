use async_trait::async_trait;
use crate::domain::value_objects::AppSettings;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[async_trait]
pub trait SettingsRepositoryPort: Send + Sync {
    async fn get(&self) -> Result<AppSettings, SettingsError>;
    async fn save(&self, settings: &AppSettings) -> Result<(), SettingsError>;
    async fn reset(&self) -> Result<AppSettings, SettingsError>;
}
