//! SQLite-backed implementation of GenerationReadStatePort.
//!
//! Stores per-user read markers for generation queue items (batches and
//! suggestions) so the Engine can reconstruct read/unread state across
//! devices and sessions.

use anyhow::Result;
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::outbound::{GenerationReadKind, GenerationReadStatePort};

/// SQLite implementation of GenerationReadStatePort
pub struct SqliteGenerationReadStateRepository {
    pool: SqlitePool,
}

impl SqliteGenerationReadStateRepository {
    /// Create a new repository backed by the given SqlitePool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Ensure the underlying table exists.
    pub async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS generation_read_state (
                user_id TEXT NOT NULL,
                world_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                item_id TEXT NOT NULL,
                read_at INTEGER NOT NULL,
                PRIMARY KEY (user_id, world_id, entity_type, item_id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl GenerationReadStatePort for SqliteGenerationReadStateRepository {
    async fn mark_read(
        &self,
        user_id: &str,
        world_id: &str,
        item_id: &str,
        kind: GenerationReadKind,
    ) -> Result<()> {
        let entity_type = match kind {
            GenerationReadKind::Batch => "batch",
            GenerationReadKind::Suggestion => "suggestion",
        };

        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO generation_read_state (user_id, world_id, entity_type, item_id, read_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(user_id, world_id, entity_type, item_id)
            DO UPDATE SET read_at = excluded.read_at;
            "#,
        )
        .bind(user_id)
        .bind(world_id)
        .bind(entity_type)
        .bind(item_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_read_for_user_world(
        &self,
        user_id: &str,
        world_id: &str,
    ) -> Result<Vec<(String, GenerationReadKind)>> {
        let rows = sqlx::query(
            r#"
            SELECT entity_type, item_id
            FROM generation_read_state
            WHERE user_id = ? AND world_id = ?
            "#,
        )
        .bind(user_id)
        .bind(world_id)
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let entity_type: String = row.get("entity_type");
            let item_id: String = row.get("item_id");
            let kind = match entity_type.as_str() {
                "batch" => GenerationReadKind::Batch,
                "suggestion" => GenerationReadKind::Suggestion,
                _ => continue,
            };
            result.push((item_id, kind));
        }

        Ok(result)
    }
}


