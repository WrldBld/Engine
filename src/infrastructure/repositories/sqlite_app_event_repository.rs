//! SQLite App Event Repository - Persistent event storage
//!
//! Stores application events in SQLite for durability, replay, and polling.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::application::dto::AppEvent;
use crate::application::ports::outbound::{AppEventRepositoryError, AppEventRepositoryPort};

/// SQLite implementation of AppEventRepositoryPort
pub struct SqliteAppEventRepository {
    pool: SqlitePool,
}

impl SqliteAppEventRepository {
    /// Create a new repository and ensure the table exists
    pub async fn new(pool: SqlitePool) -> Result<Self, AppEventRepositoryError> {
        // Create the app_events table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS app_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL,
                processed INTEGER DEFAULT 0,
                processed_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| AppEventRepositoryError::StorageError(e.to_string()))?;

        // Create index on id for efficient fetch_since queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_app_events_id_created 
            ON app_events(id, created_at)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| AppEventRepositoryError::StorageError(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Expose underlying pool for related repositories that share the same DB.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl AppEventRepositoryPort for SqliteAppEventRepository {
    async fn insert(&self, event: &AppEvent) -> Result<i64, AppEventRepositoryError> {
        let event_type = event.event_type();
        let payload = serde_json::to_string(event)
            .map_err(|e| AppEventRepositoryError::SerializationError(e.to_string()))?;
        let created_at = Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO app_events (event_type, payload, created_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(event_type)
        .bind(&payload)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppEventRepositoryError::StorageError(e.to_string()))?;

        Ok(result.last_insert_rowid())
    }

    async fn fetch_since(
        &self,
        last_id: i64,
        limit: u32,
    ) -> Result<Vec<(i64, AppEvent, DateTime<Utc>)>, AppEventRepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT id, payload, created_at
            FROM app_events
            WHERE id > ?
            ORDER BY id ASC
            LIMIT ?
            "#,
        )
        .bind(last_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppEventRepositoryError::StorageError(e.to_string()))?;

        let mut events = Vec::new();
        for row in rows {
            let id: i64 = row.get("id");
            let payload: String = row.get("payload");
            let created_at_str: String = row.get("created_at");

            let event: AppEvent = serde_json::from_str(&payload)
                .map_err(|e| AppEventRepositoryError::SerializationError(e.to_string()))?;

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| {
                    AppEventRepositoryError::StorageError(format!("Invalid timestamp: {}", e))
                })?
                .with_timezone(&Utc);

            events.push((id, event, created_at));
        }

        Ok(events)
    }
}

