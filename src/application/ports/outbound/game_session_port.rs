//! Game session port - Interface for game session operations
//!
//! This port abstracts the game session operations used by application services,
//! allowing the infrastructure to provide the concrete implementation while
//! maintaining hexagonal architecture boundaries.

/// Port for interacting with a game session
///
/// This trait defines the interface for session operations needed by the
/// application layer, particularly the ToolExecutionService. The infrastructure
/// layer provides the concrete implementation.
///
/// # Purpose
///
/// This port exists to maintain hexagonal architecture boundaries by preventing
/// application services from depending directly on infrastructure types.
///
/// # Examples
///
/// ```ignore
/// fn process_tool<S: GameSessionPort>(session: &mut S) {
///     session.add_npc_response("Guard", "You shall not pass!");
/// }
/// ```
pub trait GameSessionPort: Send + Sync {
    /// Add an NPC response to the conversation history
    ///
    /// # Arguments
    ///
    /// * `speaker` - Name of the NPC speaking
    /// * `text` - The dialogue or response text
    ///
    /// # Examples
    ///
    /// ```ignore
    /// session.add_npc_response("Merchant", "That will cost 50 gold");
    /// ```
    fn add_npc_response(&mut self, speaker: &str, text: &str);

    /// Get the length of the conversation history
    ///
    /// # Returns
    ///
    /// The number of conversation turns currently stored in the session
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let count = session.history_length();
    /// println!("Session has {} turns", count);
    /// ```
    fn history_length(&self) -> usize;
}
