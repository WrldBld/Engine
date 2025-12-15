//! Queue port - Interface for queue operations
//!
//! This port provides a storage-agnostic interface for queue operations,
//! supporting multiple backends (InMemory, SQLite, Redis) and different
//! queue types (standard, approval, processing).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::domain::value_objects::{QueueItemId, SessionId};

/// Generic queue item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem<T> {
    pub id: QueueItemId,
    pub payload: T,
    pub status: QueueItemStatus,
    pub priority: u8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl<T> QueueItem<T> {
    pub fn new(payload: T, priority: u8) -> Self {
        let now = Utc::now();
        Self {
            id: QueueItemId::new(),
            payload,
            status: QueueItemStatus::Pending,
            priority,
            created_at: now,
            updated_at: now,
            scheduled_at: None,
            attempts: 0,
            max_attempts: 3,
            error_message: None,
            metadata: HashMap::new(),
        }
    }
}

/// Status of a queue item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueueItemStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Delayed,
    Expired,
}

impl QueueItemStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueItemStatus::Pending => "pending",
            QueueItemStatus::Processing => "processing",
            QueueItemStatus::Completed => "completed",
            QueueItemStatus::Failed => "failed",
            QueueItemStatus::Delayed => "delayed",
            QueueItemStatus::Expired => "expired",
        }
    }
}

/// Errors that can occur during queue operations
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue item not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Queue is full")]
    QueueFull,

    #[error("Invalid operation for current status")]
    InvalidStatus,

    #[error("Max attempts exceeded")]
    MaxAttemptsExceeded,

    #[error("Database error: {0}")]
    Database(String),
}

/// Core queue port - storage-agnostic interface
#[async_trait]
pub trait QueuePort<T>: Send + Sync
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    /// Add item to queue
    async fn enqueue(&self, payload: T, priority: u8) -> Result<QueueItemId, QueueError>;

    /// Get next item for processing (marks as Processing)
    async fn dequeue(&self) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Peek at next item without removing
    async fn peek(&self) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Mark item as completed
    async fn complete(&self, id: QueueItemId) -> Result<(), QueueError>;

    /// Mark item as failed (may retry based on attempts)
    async fn fail(&self, id: QueueItemId, error: &str) -> Result<(), QueueError>;

    /// Delay item for later processing
    async fn delay(&self, id: QueueItemId, until: DateTime<Utc>) -> Result<(), QueueError>;

    /// Get item by ID
    async fn get(&self, id: QueueItemId) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Get all items with status
    async fn list_by_status(&self, status: QueueItemStatus) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Get queue depth (pending items)
    async fn depth(&self) -> Result<usize, QueueError>;

    /// Clear completed/failed items older than duration
    async fn cleanup(&self, older_than: Duration) -> Result<usize, QueueError>;
}

/// Extended port for approval queues with human-facing features
#[async_trait]
pub trait ApprovalQueuePort<T>: QueuePort<T>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    /// Get items by session
    async fn list_by_session(&self, session_id: SessionId) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Get history (completed/failed/expired items)
    async fn get_history(&self, session_id: SessionId, limit: usize) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Expire items older than duration
    async fn expire_old(&self, older_than: Duration) -> Result<usize, QueueError>;
}

/// Port for processing queues with concurrency control
#[async_trait]
pub trait ProcessingQueuePort<T>: QueuePort<T>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    /// Get batch size configuration
    fn batch_size(&self) -> usize;

    /// Get number of items currently processing
    async fn processing_count(&self) -> Result<usize, QueueError>;

    /// Check if can accept more work
    async fn has_capacity(&self) -> Result<bool, QueueError>;
}
