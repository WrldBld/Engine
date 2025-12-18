use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::application::ports::outbound::{SettingsRepositoryPort, SettingsError};
use crate::domain::value_objects::{AppSettings, WorldId};

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        // Create global settings table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(&pool).await?;

        // Create per-world settings table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS world_settings (
                world_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (world_id, key)
            )
        "#).execute(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Apply key-value pairs to settings struct
    fn apply_settings(settings: &mut AppSettings, rows: Vec<(String, String)>) {
        for (key, value) in rows {
            match key.as_str() {
                // Session & Conversation
                "max_conversation_turns" => if let Ok(v) = value.parse() { settings.max_conversation_turns = v; },
                "conversation_history_turns" => if let Ok(v) = value.parse() { settings.conversation_history_turns = v; },
                
                // Circuit Breaker & Health
                "circuit_breaker_failure_threshold" => if let Ok(v) = value.parse() { settings.circuit_breaker_failure_threshold = v; },
                "circuit_breaker_open_duration_secs" => if let Ok(v) = value.parse() { settings.circuit_breaker_open_duration_secs = v; },
                "health_check_cache_ttl_secs" => if let Ok(v) = value.parse() { settings.health_check_cache_ttl_secs = v; },
                
                // Validation
                "max_name_length" => if let Ok(v) = value.parse() { settings.max_name_length = v; },
                "max_description_length" => if let Ok(v) = value.parse() { settings.max_description_length = v; },
                
                // Animation
                "typewriter_sentence_delay_ms" => if let Ok(v) = value.parse() { settings.typewriter_sentence_delay_ms = v; },
                "typewriter_pause_delay_ms" => if let Ok(v) = value.parse() { settings.typewriter_pause_delay_ms = v; },
                "typewriter_char_delay_ms" => if let Ok(v) = value.parse() { settings.typewriter_char_delay_ms = v; },
                
                // Game Defaults
                "default_max_stat_value" => if let Ok(v) = value.parse() { settings.default_max_stat_value = v; },
                
                // Challenge System
                "outcome_branch_count" => if let Ok(v) = value.parse() { settings.outcome_branch_count = v; },
                "outcome_branch_min" => if let Ok(v) = value.parse() { settings.outcome_branch_min = v; },
                "outcome_branch_max" => if let Ok(v) = value.parse() { settings.outcome_branch_max = v; },
                "suggestion_tokens_per_branch" => if let Ok(v) = value.parse() { settings.suggestion_tokens_per_branch = v; },
                
                // LLM Context Budget
                "context_budget.total_budget_tokens" => if let Ok(v) = value.parse() { settings.context_budget.total_budget_tokens = v; },
                "context_budget.scene_tokens" => if let Ok(v) = value.parse() { settings.context_budget.scene_tokens = v; },
                "context_budget.character_tokens" => if let Ok(v) = value.parse() { settings.context_budget.character_tokens = v; },
                "context_budget.conversation_history_tokens" => if let Ok(v) = value.parse() { settings.context_budget.conversation_history_tokens = v; },
                "context_budget.challenges_tokens" => if let Ok(v) = value.parse() { settings.context_budget.challenges_tokens = v; },
                "context_budget.narrative_events_tokens" => if let Ok(v) = value.parse() { settings.context_budget.narrative_events_tokens = v; },
                "context_budget.directorial_notes_tokens" => if let Ok(v) = value.parse() { settings.context_budget.directorial_notes_tokens = v; },
                "context_budget.location_context_tokens" => if let Ok(v) = value.parse() { settings.context_budget.location_context_tokens = v; },
                "context_budget.player_context_tokens" => if let Ok(v) = value.parse() { settings.context_budget.player_context_tokens = v; },
                "context_budget.enable_summarization" => if let Ok(v) = value.parse() { settings.context_budget.enable_summarization = v; },
                "context_budget.summarization_model" => {
                    settings.context_budget.summarization_model = if value.is_empty() { None } else { Some(value) };
                },
                
                _ => {}
            }
        }
    }

    /// Convert settings to key-value pairs for storage
    fn settings_to_pairs(settings: &AppSettings) -> Vec<(&'static str, String)> {
        vec![
            // Session & Conversation
            ("max_conversation_turns", settings.max_conversation_turns.to_string()),
            ("conversation_history_turns", settings.conversation_history_turns.to_string()),
            
            // Circuit Breaker & Health
            ("circuit_breaker_failure_threshold", settings.circuit_breaker_failure_threshold.to_string()),
            ("circuit_breaker_open_duration_secs", settings.circuit_breaker_open_duration_secs.to_string()),
            ("health_check_cache_ttl_secs", settings.health_check_cache_ttl_secs.to_string()),
            
            // Validation
            ("max_name_length", settings.max_name_length.to_string()),
            ("max_description_length", settings.max_description_length.to_string()),
            
            // Animation
            ("typewriter_sentence_delay_ms", settings.typewriter_sentence_delay_ms.to_string()),
            ("typewriter_pause_delay_ms", settings.typewriter_pause_delay_ms.to_string()),
            ("typewriter_char_delay_ms", settings.typewriter_char_delay_ms.to_string()),
            
            // Game Defaults
            ("default_max_stat_value", settings.default_max_stat_value.to_string()),
            
            // Challenge System
            ("outcome_branch_count", settings.outcome_branch_count.to_string()),
            ("outcome_branch_min", settings.outcome_branch_min.to_string()),
            ("outcome_branch_max", settings.outcome_branch_max.to_string()),
            ("suggestion_tokens_per_branch", settings.suggestion_tokens_per_branch.to_string()),
            
            // LLM Context Budget
            ("context_budget.total_budget_tokens", settings.context_budget.total_budget_tokens.to_string()),
            ("context_budget.scene_tokens", settings.context_budget.scene_tokens.to_string()),
            ("context_budget.character_tokens", settings.context_budget.character_tokens.to_string()),
            ("context_budget.conversation_history_tokens", settings.context_budget.conversation_history_tokens.to_string()),
            ("context_budget.challenges_tokens", settings.context_budget.challenges_tokens.to_string()),
            ("context_budget.narrative_events_tokens", settings.context_budget.narrative_events_tokens.to_string()),
            ("context_budget.directorial_notes_tokens", settings.context_budget.directorial_notes_tokens.to_string()),
            ("context_budget.location_context_tokens", settings.context_budget.location_context_tokens.to_string()),
            ("context_budget.player_context_tokens", settings.context_budget.player_context_tokens.to_string()),
            ("context_budget.enable_summarization", settings.context_budget.enable_summarization.to_string()),
            ("context_budget.summarization_model", settings.context_budget.summarization_model.clone().unwrap_or_default()),
        ]
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

        Self::apply_settings(&mut settings, rows);
        Ok(settings)
    }

    async fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        let pairs = Self::settings_to_pairs(settings);

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

    async fn get_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        // Start with global settings
        let mut settings = self.get().await?;
        settings.world_id = Some(world_id);

        // Override with world-specific values
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM world_settings WHERE world_id = ?"
        )
            .bind(world_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        Self::apply_settings(&mut settings, rows);
        Ok(settings)
    }

    async fn save_for_world(&self, world_id: WorldId, settings: &AppSettings) -> Result<(), SettingsError> {
        let pairs = Self::settings_to_pairs(settings);
        let world_id_str = world_id.to_string();

        for (key, value) in pairs {
            sqlx::query(
                "INSERT OR REPLACE INTO world_settings (world_id, key, value, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP)"
            )
                .bind(&world_id_str)
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await
                .map_err(|e| SettingsError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        // Delete world-specific settings
        sqlx::query("DELETE FROM world_settings WHERE world_id = ?")
            .bind(world_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        // Return global settings with world_id set
        let mut settings = self.get().await?;
        settings.world_id = Some(world_id);
        Ok(settings)
    }

    async fn delete_for_world(&self, world_id: WorldId) -> Result<(), SettingsError> {
        sqlx::query("DELETE FROM world_settings WHERE world_id = ?")
            .bind(world_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        Ok(())
    }
}
