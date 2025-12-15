//! App Event Repository Port - Interface for persisting application events
//!
//! This port abstracts event storage, allowing the infrastructure to provide
//! different implementations (SQLite, PostgreSQL, Redis, etc.)

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::dto::AppEvent;

/// Port for storing and retrieving application events
#[async_trait]
pub trait AppEventRepositoryPort: Send + Sync {
    /// Insert a new event into storage
    ///
    /// Returns the unique ID assigned to the event by the storage backend
    async fn insert(&self, event: &AppEvent) -> Result<i64, AppEventRepositoryError>;

    /// Fetch events since a given ID
    ///
    /// Returns events with ID > last_id, up to `limit` events, ordered by ID ascending
    async fn fetch_since(
        &self,
        last_id: i64,
        limit: u32,
    ) -> Result<Vec<(i64, AppEvent, DateTime<Utc>)>, AppEventRepositoryError>;
}

/// Errors that can occur when accessing the event repository
#[derive(Debug)]
pub enum AppEventRepositoryError {
    /// Database or storage-level error
    StorageError(String),
    /// Serialization/deserialization error
    SerializationError(String),
}

impl std::fmt::Display for AppEventRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEventRepositoryError::StorageError(msg) => {
                write!(f, "Event repository storage error: {}", msg)
            }
            AppEventRepositoryError::SerializationError(msg) => {
                write!(f, "Event serialization error: {}", msg)
            }
        }
    }
}

impl std::error::Error for AppEventRepositoryError {}

