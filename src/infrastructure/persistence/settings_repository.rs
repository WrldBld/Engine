use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::application::ports::outbound::{SettingsRepositoryPort, SettingsError};
use crate::domain::value_objects::AppSettings;

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        // Create table if not exists
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl SettingsRepositoryPort for SqliteSettingsRepository {
    async fn get(&self) -> Result<AppSettings, SettingsError> {
        let mut settings = AppSettings::from_env(); // Start with env defaults

        // Override with DB values
        let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        for (key, value) in rows {
            match key.as_str() {
                "max_conversation_turns" => if let Ok(v) = value.parse() { settings.max_conversation_turns = v; },
                "circuit_breaker_failure_threshold" => if let Ok(v) = value.parse() { settings.circuit_breaker_failure_threshold = v; },
                "circuit_breaker_open_duration_secs" => if let Ok(v) = value.parse() { settings.circuit_breaker_open_duration_secs = v; },
                "health_check_cache_ttl_secs" => if let Ok(v) = value.parse() { settings.health_check_cache_ttl_secs = v; },
                "max_name_length" => if let Ok(v) = value.parse() { settings.max_name_length = v; },
                "max_description_length" => if let Ok(v) = value.parse() { settings.max_description_length = v; },
                "typewriter_sentence_delay_ms" => if let Ok(v) = value.parse() { settings.typewriter_sentence_delay_ms = v; },
                "typewriter_pause_delay_ms" => if let Ok(v) = value.parse() { settings.typewriter_pause_delay_ms = v; },
                "typewriter_char_delay_ms" => if let Ok(v) = value.parse() { settings.typewriter_char_delay_ms = v; },
                "default_max_stat_value" => if let Ok(v) = value.parse() { settings.default_max_stat_value = v; },
                _ => {}
            }
        }

        Ok(settings)
    }

    async fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        let pairs = [
            ("max_conversation_turns", settings.max_conversation_turns.to_string()),
            ("circuit_breaker_failure_threshold", settings.circuit_breaker_failure_threshold.to_string()),
            ("circuit_breaker_open_duration_secs", settings.circuit_breaker_open_duration_secs.to_string()),
            ("health_check_cache_ttl_secs", settings.health_check_cache_ttl_secs.to_string()),
            ("max_name_length", settings.max_name_length.to_string()),
            ("max_description_length", settings.max_description_length.to_string()),
            ("typewriter_sentence_delay_ms", settings.typewriter_sentence_delay_ms.to_string()),
            ("typewriter_pause_delay_ms", settings.typewriter_pause_delay_ms.to_string()),
            ("typewriter_char_delay_ms", settings.typewriter_char_delay_ms.to_string()),
            ("default_max_stat_value", settings.default_max_stat_value.to_string()),
        ];

        for (key, value) in pairs {
            sqlx::query("INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)")
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await
                .map_err(|e| SettingsError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn reset(&self) -> Result<AppSettings, SettingsError> {
        sqlx::query("DELETE FROM settings")
            .execute(&self.pool)
            .await
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        Ok(AppSettings::from_env())
    }
}
