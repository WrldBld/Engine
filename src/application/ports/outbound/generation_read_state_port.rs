//! Port for persisting per-user read/unread state for generation queue items.
//!
//! This allows the Engine to keep track of which batches and suggestion tasks
//! have been seen by a given user, so the unified generation queue can be
//! reconstructed consistently across devices and sessions.

use async_trait::async_trait;

/// Kind of generation queue item being marked as read
#[derive(Debug, Clone, Copy)]
pub enum GenerationReadKind {
    /// Image generation batch (identified by batch_id)
    Batch,
    /// Text suggestion task (identified by request_id)
    Suggestion,
}

#[async_trait]
pub trait GenerationReadStatePort: Send + Sync {
    /// Mark a given item as read for a specific user.
    async fn mark_read(
        &self,
        user_id: &str,
        world_id: &str,
        item_id: &str,
        kind: GenerationReadKind,
    ) -> anyhow::Result<()>;

    /// List all read item ids for a user in a specific world.
    ///
    /// Returns a vec of (item_id, kind) tuples.
    async fn list_read_for_user_world(
        &self,
        user_id: &str,
        world_id: &str,
    ) -> anyhow::Result<Vec<(String, GenerationReadKind)>>;
}


