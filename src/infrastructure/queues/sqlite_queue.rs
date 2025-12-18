//! SQLite queue implementation for production persistence
//!
//! This implementation uses SQLite for persistent queue storage, enabling
//! crash recovery and multi-process deployments (with file locking).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use sqlx::{SqlitePool, Row};

use crate::application::ports::outbound::{
    ApprovalQueuePort, ProcessingQueuePort, QueueError, QueueItem, QueueItemId, QueueItemStatus,
    QueueNotificationPort, QueuePort,
};
use crate::domain::value_objects::SessionId;

/// SQLite queue implementation
pub struct SqliteQueue<T, N: QueueNotificationPort> {
    pool: SqlitePool,
    queue_name: String,
    batch_size: usize,
    notifier: N,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, N: QueueNotificationPort> SqliteQueue<T, N> {
    /// Get the notifier for this queue
    pub fn notifier(&self) -> &N {
        &self.notifier
    }
}

impl<T, N: QueueNotificationPort> SqliteQueue<T, N>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    pub async fn new(pool: SqlitePool, queue_name: impl Into<String>, batch_size: usize, notifier: N) -> Result<Self, QueueError> {
        let queue_name = queue_name.into();
        
        // Ensure table exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS queue_items (
                id TEXT PRIMARY KEY,
                queue_name TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                priority INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                scheduled_at TEXT,
                attempts INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                error_message TEXT,
                metadata_json TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        // Create indexes
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_queue_status 
            ON queue_items(queue_name, status, priority DESC, created_at)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_queue_scheduled 
            ON queue_items(queue_name, status, scheduled_at) 
            WHERE status = 'delayed'
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        Ok(Self {
            pool,
            queue_name,
            batch_size,
            notifier,
            _phantom: std::marker::PhantomData,
        })
    }

    fn status_to_str(status: QueueItemStatus) -> &'static str {
        match status {
            QueueItemStatus::Pending => "pending",
            QueueItemStatus::Processing => "processing",
            QueueItemStatus::Completed => "completed",
            QueueItemStatus::Failed => "failed",
            QueueItemStatus::Delayed => "delayed",
            QueueItemStatus::Expired => "expired",
        }
    }

    fn str_to_status(s: &str) -> QueueItemStatus {
        match s {
            "pending" => QueueItemStatus::Pending,
            "processing" => QueueItemStatus::Processing,
            "completed" => QueueItemStatus::Completed,
            "failed" => QueueItemStatus::Failed,
            "delayed" => QueueItemStatus::Delayed,
            "expired" => QueueItemStatus::Expired,
            _ => QueueItemStatus::Pending,
        }
    }

    fn parse_metadata(metadata_json: Option<&str>) -> HashMap<String, String> {
        metadata_json
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    fn serialize_metadata(metadata: &HashMap<String, String>) -> String {
        serde_json::to_string(metadata).unwrap_or_else(|_| "{}".to_string())
    }

    async fn row_to_item(&self, row: sqlx::sqlite::SqliteRow) -> Result<QueueItem<T>, QueueError> {
        let id_str: String = row.get("id");
        let id = uuid::Uuid::parse_str(&id_str)
            .map_err(|e| QueueError::Backend(format!("Invalid UUID: {}", e)))?;
        let id = QueueItemId::from_uuid(id);

        let payload_json: String = row.get("payload_json");
        let payload: T = serde_json::from_str(&payload_json)?;

        let status_str: String = row.get("status");
        let status = Self::str_to_status(&status_str);

        let priority: i64 = row.get("priority");
        let priority = priority as u8;

        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| QueueError::Backend(format!("Invalid datetime: {}", e)))?
            .with_timezone(&Utc);

        let updated_at_str: String = row.get("updated_at");
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| QueueError::Backend(format!("Invalid datetime: {}", e)))?
            .with_timezone(&Utc);

        let scheduled_at_str: Option<String> = row.get("scheduled_at");
        let scheduled_at = scheduled_at_str
            .map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map_err(|e| QueueError::Backend(format!("Invalid datetime: {}", e)))
                    .map(|dt| dt.with_timezone(&Utc))
            })
            .transpose()?;

        let attempts: i64 = row.get("attempts");
        let attempts = attempts as u32;

        let max_attempts: i64 = row.get("max_attempts");
        let max_attempts = max_attempts as u32;

        let error_message: Option<String> = row.get("error_message");

        let metadata_json: Option<String> = row.get("metadata_json");
        let metadata = Self::parse_metadata(metadata_json.as_deref());

        Ok(QueueItem {
            id,
            payload,
            status,
            priority,
            created_at,
            updated_at,
            scheduled_at,
            attempts,
            max_attempts,
            error_message,
            metadata,
        })
    }
}

#[async_trait]
impl<T, N: QueueNotificationPort + 'static> QueuePort<T> for SqliteQueue<T, N>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
{
    async fn enqueue(&self, payload: T, priority: u8) -> Result<QueueItemId, QueueError> {
        let id = QueueItemId::new();
        let payload_json = serde_json::to_string(&payload)?;
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO queue_items 
            (id, queue_name, payload_json, status, priority, created_at, updated_at, attempts, max_attempts, metadata_json)
            VALUES (?, ?, ?, 'pending', ?, ?, ?, 0, 3, '{}')
            "#,
        )
        .bind(id.to_string())
        .bind(&self.queue_name)
        .bind(&payload_json)
        .bind(priority as i64)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        // Notify workers that work is available
        self.notifier.notify_work_available().await;

        Ok(id)
    }

    async fn dequeue(&self) -> Result<Option<QueueItem<T>>, QueueError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // Use atomic UPDATE with subquery to avoid TOCTOU race condition.
        // This atomically selects and updates the next available item.
        // The WHERE clause includes status check to prevent double-processing.
        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'processing', updated_at = ?, attempts = attempts + 1
            WHERE id = (
                SELECT id FROM queue_items
                WHERE queue_name = ?
                AND (
                    (status = 'pending')
                    OR (status = 'delayed' AND scheduled_at <= ?)
                )
                ORDER BY priority DESC, created_at ASC
                LIMIT 1
            )
            AND queue_name = ?
            AND (status = 'pending' OR status = 'delayed')
            RETURNING id
            "#,
        )
        .bind(&now_str)
        .bind(&self.queue_name)
        .bind(&now_str)
        .bind(&self.queue_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        if let Some(row) = result {
            let id_str: String = row.get("id");
            let id = uuid::Uuid::parse_str(&id_str)
                .map_err(|e| QueueError::Backend(format!("Invalid UUID: {}", e)))?;
            let id = QueueItemId::from_uuid(id);

            // Fetch the full item with all fields
            self.get(id).await
        } else {
            Ok(None)
        }
    }

    async fn peek(&self) -> Result<Option<QueueItem<T>>, QueueError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let row = sqlx::query(
            r#"
            SELECT * FROM queue_items
            WHERE queue_name = ?
            AND (
                (status = 'pending')
                OR (status = 'delayed' AND scheduled_at <= ?)
            )
            ORDER BY priority DESC, created_at ASC
            LIMIT 1
            "#,
        )
        .bind(&self.queue_name)
        .bind(&now_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        if let Some(row) = row {
            Ok(Some(self.row_to_item(row).await?))
        } else {
            Ok(None)
        }
    }

    async fn complete(&self, id: QueueItemId) -> Result<(), QueueError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'completed', updated_at = ?
            WHERE id = ? AND queue_name = ?
            "#,
        )
        .bind(&now_str)
        .bind(id.to_string())
        .bind(&self.queue_name)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QueueError::NotFound(id.to_string()));
        }

        Ok(())
    }

    async fn fail(&self, id: QueueItemId, error: &str) -> Result<(), QueueError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'failed', updated_at = ?, error_message = ?
            WHERE id = ? AND queue_name = ?
            "#,
        )
        .bind(&now_str)
        .bind(error)
        .bind(id.to_string())
        .bind(&self.queue_name)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QueueError::NotFound(id.to_string()));
        }

        Ok(())
    }

    async fn delay(&self, id: QueueItemId, until: DateTime<Utc>) -> Result<(), QueueError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let until_str = until.to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'delayed', updated_at = ?, scheduled_at = ?
            WHERE id = ? AND queue_name = ?
            "#,
        )
        .bind(&now_str)
        .bind(&until_str)
        .bind(id.to_string())
        .bind(&self.queue_name)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QueueError::NotFound(id.to_string()));
        }

        Ok(())
    }

    async fn get(&self, id: QueueItemId) -> Result<Option<QueueItem<T>>, QueueError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM queue_items
            WHERE id = ? AND queue_name = ?
            "#,
        )
        .bind(id.to_string())
        .bind(&self.queue_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        if let Some(row) = row {
            Ok(Some(self.row_to_item(row).await?))
        } else {
            Ok(None)
        }
    }

    async fn list_by_status(&self, status: QueueItemStatus) -> Result<Vec<QueueItem<T>>, QueueError> {
        let status_str = Self::status_to_str(status);

        let rows = sqlx::query(
            r#"
            SELECT * FROM queue_items
            WHERE queue_name = ? AND status = ?
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .bind(&self.queue_name)
        .bind(status_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(self.row_to_item(row).await?);
        }
        Ok(items)
    }

    async fn depth(&self) -> Result<usize, QueueError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count FROM queue_items
            WHERE queue_name = ? AND status = 'pending'
            "#,
        )
        .bind(&self.queue_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        let count: i64 = row.get("count");
        Ok(count as usize)
    }

    async fn cleanup(&self, older_than: Duration) -> Result<usize, QueueError> {
        let cutoff = Utc::now() - older_than;
        let cutoff_str = cutoff.to_rfc3339();

        let result = sqlx::query(
            r#"
            DELETE FROM queue_items
            WHERE queue_name = ?
            AND status IN ('completed', 'failed')
            AND updated_at < ?
            "#,
        )
        .bind(&self.queue_name)
        .bind(&cutoff_str)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }
}

#[async_trait]
impl<T, N: QueueNotificationPort + 'static> ApprovalQueuePort<T> for SqliteQueue<T, N>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
{
    async fn list_by_session(&self, _session_id: SessionId) -> Result<Vec<QueueItem<T>>, QueueError> {
        // Extract session_id from payload JSON
        // This is a limitation - we need to know the structure of T
        // In practice, ApprovalQueuePort should only be used with ApprovalItem
        // which has session_id. For now, we'll return all pending/processing items.
        // The service layer should filter by session_id after deserialization.
        self.list_by_status(QueueItemStatus::Pending).await
    }

    async fn get_history(
        &self,
        _session_id: SessionId,
        limit: usize,
    ) -> Result<Vec<QueueItem<T>>, QueueError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM queue_items
            WHERE queue_name = ?
            AND status IN ('completed', 'failed', 'expired')
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(&self.queue_name)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(self.row_to_item(row).await?);
        }
        Ok(items)
    }

    async fn expire_old(&self, older_than: Duration) -> Result<usize, QueueError> {
        let cutoff = Utc::now() - older_than;
        let cutoff_str = cutoff.to_rfc3339();
        let now_str = Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'expired', updated_at = ?
            WHERE queue_name = ?
            AND status IN ('pending', 'delayed')
            AND created_at < ?
            "#,
        )
        .bind(&now_str)
        .bind(&self.queue_name)
        .bind(&cutoff_str)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }
}

#[async_trait]
impl<T, N: QueueNotificationPort + 'static> ProcessingQueuePort<T> for SqliteQueue<T, N>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
{
    fn batch_size(&self) -> usize {
        self.batch_size
    }

    async fn processing_count(&self) -> Result<usize, QueueError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count FROM queue_items
            WHERE queue_name = ? AND status = 'processing'
            "#,
        )
        .bind(&self.queue_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| QueueError::Database(e.to_string()))?;

        let count: i64 = row.get("count");
        Ok(count as usize)
    }

    async fn has_capacity(&self) -> Result<bool, QueueError> {
        let processing = self.processing_count().await?;
        Ok(processing < self.batch_size())
    }
}
