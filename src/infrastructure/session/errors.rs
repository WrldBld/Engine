//! Session error types

use crate::domain::value_objects::{SessionId};
use super::ClientId;

/// Error types for session operations
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(SessionId),

    #[error("World not found: {0}")]
    WorldNotFound(String),

    #[error("Client not in any session: {0}")]
    #[allow(dead_code)] // Kept for comprehensive error handling
    ClientNotInSession(ClientId),

    #[error("Session already has a DM")]
    DmAlreadyPresent,

    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),
}
