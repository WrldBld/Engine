//! Queue factory - Creates queue instances based on configuration
//!
//! This module provides a factory pattern for creating queue instances
//! with different backends (InMemory, SQLite, etc.) while maintaining
//! modularity for future backends.

use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use crate::application::dto::{
    ApprovalItem, AssetGenerationItem, DMActionItem, LLMRequestItem, PlayerActionItem,
};
use crate::infrastructure::config::QueueConfig;
use crate::infrastructure::queues::{InMemoryQueue, InProcessNotifier, SqliteQueue};

/// Enum wrapper for queue backends to enable runtime selection
/// This allows us to use different backends while maintaining type safety
/// Note: This enum is not Clone - use Arc<QueueBackendEnum<T>> for sharing
pub enum QueueBackendEnum<T> {
    Memory(InMemoryQueue<T, InProcessNotifier>),
    Sqlite(SqliteQueue<T, InProcessNotifier>),
}

// Implement QueuePort for the enum
#[async_trait::async_trait]
impl<T> crate::application::ports::outbound::QueuePort<T> for QueueBackendEnum<T>
where
    T: Send + Sync + Clone + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    async fn enqueue(&self, payload: T, priority: u8) -> Result<crate::application::ports::outbound::QueueItemId, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.enqueue(payload, priority).await,
            QueueBackendEnum::Sqlite(q) => q.enqueue(payload, priority).await,
        }
    }

    async fn dequeue(&self) -> Result<Option<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.dequeue().await,
            QueueBackendEnum::Sqlite(q) => q.dequeue().await,
        }
    }

    async fn peek(&self) -> Result<Option<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.peek().await,
            QueueBackendEnum::Sqlite(q) => q.peek().await,
        }
    }

    async fn complete(&self, id: crate::application::ports::outbound::QueueItemId) -> Result<(), crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.complete(id).await,
            QueueBackendEnum::Sqlite(q) => q.complete(id).await,
        }
    }

    async fn fail(&self, id: crate::application::ports::outbound::QueueItemId, error: &str) -> Result<(), crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.fail(id, error).await,
            QueueBackendEnum::Sqlite(q) => q.fail(id, error).await,
        }
    }

    async fn delay(&self, id: crate::application::ports::outbound::QueueItemId, until: chrono::DateTime<chrono::Utc>) -> Result<(), crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.delay(id, until).await,
            QueueBackendEnum::Sqlite(q) => q.delay(id, until).await,
        }
    }

    async fn get(&self, id: crate::application::ports::outbound::QueueItemId) -> Result<Option<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.get(id).await,
            QueueBackendEnum::Sqlite(q) => q.get(id).await,
        }
    }

    async fn list_by_status(&self, status: crate::application::ports::outbound::QueueItemStatus) -> Result<Vec<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.list_by_status(status).await,
            QueueBackendEnum::Sqlite(q) => q.list_by_status(status).await,
        }
    }

    async fn depth(&self) -> Result<usize, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.depth().await,
            QueueBackendEnum::Sqlite(q) => q.depth().await,
        }
    }

    async fn cleanup(&self, older_than: std::time::Duration) -> Result<usize, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.cleanup(older_than).await,
            QueueBackendEnum::Sqlite(q) => q.cleanup(older_than).await,
        }
    }
}

impl<T> QueueBackendEnum<T> {
    /// Get the notifier for this queue
    pub fn notifier(&self) -> InProcessNotifier {
        match self {
            QueueBackendEnum::Memory(q) => q.notifier().clone(),
            QueueBackendEnum::Sqlite(q) => q.notifier().clone(),
        }
    }
}

// Implement ProcessingQueuePort for the enum
#[async_trait::async_trait]
impl<T> crate::application::ports::outbound::ProcessingQueuePort<T> for QueueBackendEnum<T>
where
    T: Send + Sync + Clone + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    fn batch_size(&self) -> usize {
        match self {
            QueueBackendEnum::Memory(q) => q.batch_size(),
            QueueBackendEnum::Sqlite(q) => q.batch_size(),
        }
    }

    async fn processing_count(&self) -> Result<usize, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.processing_count().await,
            QueueBackendEnum::Sqlite(q) => q.processing_count().await,
        }
    }

    async fn has_capacity(&self) -> Result<bool, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.has_capacity().await,
            QueueBackendEnum::Sqlite(q) => q.has_capacity().await,
        }
    }
}

// Implement ApprovalQueuePort for the enum
#[async_trait::async_trait]
impl<T> crate::application::ports::outbound::ApprovalQueuePort<T> for QueueBackendEnum<T>
where
    T: Send + Sync + Clone + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    async fn list_by_session(&self, session_id: crate::domain::value_objects::SessionId) -> Result<Vec<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.list_by_session(session_id).await,
            QueueBackendEnum::Sqlite(q) => q.list_by_session(session_id).await,
        }
    }

    async fn get_history(&self, session_id: crate::domain::value_objects::SessionId, limit: usize) -> Result<Vec<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.get_history(session_id, limit).await,
            QueueBackendEnum::Sqlite(q) => q.get_history(session_id, limit).await,
        }
    }

    async fn expire_old(&self, older_than: std::time::Duration) -> Result<usize, crate::application::ports::outbound::QueueError> {
        match self {
            QueueBackendEnum::Memory(q) => q.expire_old(older_than).await,
            QueueBackendEnum::Sqlite(q) => q.expire_old(older_than).await,
        }
    }
}

// Implement port traits for Arc<QueueBackendEnum<T>> to allow services to use Arc-wrapped queues
#[async_trait::async_trait]
impl<T> crate::application::ports::outbound::QueuePort<T> for Arc<QueueBackendEnum<T>>
where
    T: Send + Sync + Clone + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    async fn enqueue(&self, payload: T, priority: u8) -> Result<crate::application::ports::outbound::QueueItemId, crate::application::ports::outbound::QueueError> {
        self.as_ref().enqueue(payload, priority).await
    }

    async fn dequeue(&self) -> Result<Option<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        self.as_ref().dequeue().await
    }

    async fn peek(&self) -> Result<Option<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        self.as_ref().peek().await
    }

    async fn complete(&self, id: crate::application::ports::outbound::QueueItemId) -> Result<(), crate::application::ports::outbound::QueueError> {
        self.as_ref().complete(id).await
    }

    async fn fail(&self, id: crate::application::ports::outbound::QueueItemId, error: &str) -> Result<(), crate::application::ports::outbound::QueueError> {
        self.as_ref().fail(id, error).await
    }

    async fn delay(&self, id: crate::application::ports::outbound::QueueItemId, until: chrono::DateTime<chrono::Utc>) -> Result<(), crate::application::ports::outbound::QueueError> {
        self.as_ref().delay(id, until).await
    }

    async fn depth(&self) -> Result<usize, crate::application::ports::outbound::QueueError> {
        self.as_ref().depth().await
    }

    async fn get(&self, id: crate::application::ports::outbound::QueueItemId) -> Result<Option<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        self.as_ref().get(id).await
    }

    async fn list_by_status(&self, status: crate::application::ports::outbound::QueueItemStatus) -> Result<Vec<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        self.as_ref().list_by_status(status).await
    }

    async fn cleanup(&self, older_than: std::time::Duration) -> Result<usize, crate::application::ports::outbound::QueueError> {
        self.as_ref().cleanup(older_than).await
    }
}


#[async_trait::async_trait]
impl<T> crate::application::ports::outbound::ProcessingQueuePort<T> for Arc<QueueBackendEnum<T>>
where
    T: Send + Sync + Clone + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    fn batch_size(&self) -> usize {
        self.as_ref().batch_size()
    }

    async fn processing_count(&self) -> Result<usize, crate::application::ports::outbound::QueueError> {
        self.as_ref().processing_count().await
    }

    async fn has_capacity(&self) -> Result<bool, crate::application::ports::outbound::QueueError> {
        self.as_ref().has_capacity().await
    }
}

#[async_trait::async_trait]
impl<T> crate::application::ports::outbound::ApprovalQueuePort<T> for Arc<QueueBackendEnum<T>>
where
    T: Send + Sync + Clone + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    async fn list_by_session(&self, session_id: crate::domain::value_objects::SessionId) -> Result<Vec<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        self.as_ref().list_by_session(session_id).await
    }

    async fn get_history(&self, session_id: crate::domain::value_objects::SessionId, limit: usize) -> Result<Vec<crate::application::ports::outbound::QueueItem<T>>, crate::application::ports::outbound::QueueError> {
        self.as_ref().get_history(session_id, limit).await
    }

    async fn expire_old(&self, older_than: std::time::Duration) -> Result<usize, crate::application::ports::outbound::QueueError> {
        self.as_ref().expire_old(older_than).await
    }
}

/// Queue factory for creating queue instances
pub struct QueueFactory {
    config: QueueConfig,
    sqlite_pool: Option<SqlitePool>,
    // Shared notifiers per queue type
    player_action_notifier: InProcessNotifier,
    dm_action_notifier: InProcessNotifier,
    llm_notifier: InProcessNotifier,
    asset_generation_notifier: InProcessNotifier,
    approval_notifier: InProcessNotifier,
}

impl QueueFactory {
    /// Create a new queue factory
    pub async fn new(config: QueueConfig) -> Result<Self> {
        let sqlite_pool = if config.backend == "sqlite" {
            // Ensure data directory exists
            if let Some(parent) = std::path::Path::new(&config.sqlite_path).parent() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create queue database directory")?;
            }

            let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", config.sqlite_path))
                .await
                .context("Failed to connect to SQLite queue database")?;
            tracing::info!("Connected to SQLite queue database: {}", config.sqlite_path);
            Some(pool)
        } else {
            None
        };

        Ok(Self {
            config,
            sqlite_pool,
            player_action_notifier: InProcessNotifier::new("player_actions"),
            dm_action_notifier: InProcessNotifier::new("dm_actions"),
            llm_notifier: InProcessNotifier::new("llm_requests"),
            asset_generation_notifier: InProcessNotifier::new("asset_generation"),
            approval_notifier: InProcessNotifier::new("approvals"),
        })
    }

    /// Get the player action notifier
    pub fn player_action_notifier(&self) -> InProcessNotifier {
        self.player_action_notifier.clone()
    }

    /// Get the DM action notifier
    pub fn dm_action_notifier(&self) -> InProcessNotifier {
        self.dm_action_notifier.clone()
    }

    /// Get the LLM notifier
    pub fn llm_notifier(&self) -> InProcessNotifier {
        self.llm_notifier.clone()
    }

    /// Get the asset generation notifier
    pub fn asset_generation_notifier(&self) -> InProcessNotifier {
        self.asset_generation_notifier.clone()
    }

    /// Get the approval notifier
    pub fn approval_notifier(&self) -> InProcessNotifier {
        self.approval_notifier.clone()
    }

    /// Create a player action queue
    pub async fn create_player_action_queue(
        &self,
    ) -> Result<Arc<QueueBackendEnum<PlayerActionItem>>> {
        match self.config.backend.as_str() {
            "memory" => Ok(Arc::new(QueueBackendEnum::Memory(InMemoryQueue::new("player_actions", self.player_action_notifier.clone())))),
            "sqlite" => {
                let pool = self
                    .sqlite_pool
                    .as_ref()
                    .context("SQLite pool not initialized")?;
                let queue = SqliteQueue::new(pool.clone(), "player_actions", 1, self.player_action_notifier.clone()).await?;
                Ok(Arc::new(QueueBackendEnum::Sqlite(queue)))
            }
            backend => anyhow::bail!("Unsupported queue backend: {}", backend),
        }
    }

    /// Create an LLM request queue (processing queue)
    pub async fn create_llm_queue(
        &self,
    ) -> Result<Arc<QueueBackendEnum<LLMRequestItem>>> {
        match self.config.backend.as_str() {
            "memory" => Ok(Arc::new(QueueBackendEnum::Memory(InMemoryQueue::new("llm_requests", self.llm_notifier.clone())))),
            "sqlite" => {
                let pool = self
                    .sqlite_pool
                    .as_ref()
                    .context("SQLite pool not initialized")?;
                let queue = SqliteQueue::new(
                    pool.clone(),
                    "llm_requests",
                    self.config.llm_batch_size,
                    self.llm_notifier.clone(),
                )
                .await?;
                Ok(Arc::new(QueueBackendEnum::Sqlite(queue)))
            }
            backend => anyhow::bail!("Unsupported queue backend: {}", backend),
        }
    }

    /// Create a DM action queue
    pub async fn create_dm_action_queue(
        &self,
    ) -> Result<Arc<QueueBackendEnum<DMActionItem>>> {
        match self.config.backend.as_str() {
            "memory" => Ok(Arc::new(QueueBackendEnum::Memory(InMemoryQueue::new("dm_actions", self.dm_action_notifier.clone())))),
            "sqlite" => {
                let pool = self
                    .sqlite_pool
                    .as_ref()
                    .context("SQLite pool not initialized")?;
                let queue = SqliteQueue::new(pool.clone(), "dm_actions", 1, self.dm_action_notifier.clone()).await?;
                Ok(Arc::new(QueueBackendEnum::Sqlite(queue)))
            }
            backend => anyhow::bail!("Unsupported queue backend: {}", backend),
        }
    }

    /// Create an asset generation queue (processing queue)
    pub async fn create_asset_generation_queue(
        &self,
    ) -> Result<Arc<QueueBackendEnum<AssetGenerationItem>>> {
        match self.config.backend.as_str() {
            "memory" => Ok(Arc::new(QueueBackendEnum::Memory(InMemoryQueue::new("asset_generation", self.asset_generation_notifier.clone())))),
            "sqlite" => {
                let pool = self
                    .sqlite_pool
                    .as_ref()
                    .context("SQLite pool not initialized")?;
                let queue = SqliteQueue::new(
                    pool.clone(),
                    "asset_generation",
                    self.config.asset_batch_size,
                    self.asset_generation_notifier.clone(),
                )
                .await?;
                Ok(Arc::new(QueueBackendEnum::Sqlite(queue)))
            }
            backend => anyhow::bail!("Unsupported queue backend: {}", backend),
        }
    }

    /// Create an approval queue (approval queue)
    pub async fn create_approval_queue(
        &self,
    ) -> Result<Arc<QueueBackendEnum<ApprovalItem>>> {
        match self.config.backend.as_str() {
            "memory" => Ok(Arc::new(QueueBackendEnum::Memory(InMemoryQueue::new("approvals", self.approval_notifier.clone())))),
            "sqlite" => {
                let pool = self
                    .sqlite_pool
                    .as_ref()
                    .context("SQLite pool not initialized")?;
                let queue = SqliteQueue::new(pool.clone(), "approvals", 1, self.approval_notifier.clone()).await?;
                Ok(Arc::new(QueueBackendEnum::Sqlite(queue)))
            }
            backend => anyhow::bail!("Unsupported queue backend: {}", backend),
        }
    }

    /// Get queue configuration
    pub fn config(&self) -> &QueueConfig {
        &self.config
    }
}
