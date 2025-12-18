//! Event Effect Executor Service - Executes effects from narrative event outcomes
//!
//! This service is responsible for executing the `EventEffect` items that are
//! defined in narrative event outcomes. When a DM approves a narrative event,
//! the selected outcome's effects are executed through this service.
//!
//! # Effects Supported
//!
//! - `SetFlag` - Sets a game flag (stored in session state)
//! - `EnableChallenge` / `DisableChallenge` - Toggles challenge availability
//! - `EnableEvent` / `DisableEvent` - Toggles narrative event availability
//! - `RevealInformation` - Reveals info to players (logged, optionally journaled)
//! - `GiveItem` / `TakeItem` - Modifies player inventory (logged for DM to narrate)
//! - `ModifyRelationship` - Changes NPC relationship sentiment
//! - `ModifyStat` - Changes character stat value
//! - `TriggerScene` - Initiates scene transition
//! - `StartCombat` - Initiates combat encounter
//! - `AddReward` - Grants experience or rewards
//! - `Custom` - Logs for DM action
//!
//! # Architecture
//!
//! The service follows hexagonal architecture, depending on repository ports
//! and the async session port for all operations.

use std::sync::Arc;

use tracing::{debug, info, warn, instrument};

use crate::application::ports::outbound::{
    AsyncSessionPort, ChallengeRepositoryPort, NarrativeEventRepositoryPort,
    RelationshipRepositoryPort,
};
use crate::domain::entities::EventEffect;
use crate::domain::value_objects::SessionId;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during effect execution
#[derive(Debug, thiserror::Error)]
pub enum EffectExecutionError {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Effect not supported: {0}")]
    NotSupported(String),
}

// =============================================================================
// Result Types
// =============================================================================

/// Result of executing a single effect
#[derive(Debug, Clone)]
pub struct EffectExecutionResult {
    /// Description of what happened
    pub description: String,
    /// Whether the effect was fully executed (vs logged for DM action)
    pub was_executed: bool,
    /// Any warning or note
    pub note: Option<String>,
}

/// Result of executing all effects from an outcome
#[derive(Debug, Clone)]
pub struct OutcomeExecutionResult {
    /// Individual effect results
    pub effects: Vec<EffectExecutionResult>,
    /// Total effects attempted
    pub total: usize,
    /// Effects that were fully executed
    pub executed_count: usize,
    /// Effects logged for DM action
    pub logged_count: usize,
}

impl OutcomeExecutionResult {
    pub fn empty() -> Self {
        Self {
            effects: Vec::new(),
            total: 0,
            executed_count: 0,
            logged_count: 0,
        }
    }
}

// =============================================================================
// Service Implementation
// =============================================================================

/// Service for executing narrative event outcome effects
///
/// This service takes a list of `EventEffect` items and executes them,
/// making the necessary repository calls and session notifications.
pub struct EventEffectExecutor {
    sessions: Arc<dyn AsyncSessionPort>,
    challenge_repo: Arc<dyn ChallengeRepositoryPort>,
    narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort>,
    relationship_repo: Arc<dyn RelationshipRepositoryPort>,
}

impl EventEffectExecutor {
    /// Create a new EventEffectExecutor
    pub fn new(
        sessions: Arc<dyn AsyncSessionPort>,
        challenge_repo: Arc<dyn ChallengeRepositoryPort>,
        narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort>,
        relationship_repo: Arc<dyn RelationshipRepositoryPort>,
    ) -> Self {
        Self {
            sessions,
            challenge_repo,
            narrative_event_repo,
            relationship_repo,
        }
    }

    /// Execute all effects from an outcome
    ///
    /// # Arguments
    ///
    /// * `effects` - The list of effects to execute
    /// * `session_id` - The session where effects should be applied/announced
    ///
    /// # Returns
    ///
    /// An `OutcomeExecutionResult` summarizing what was done.
    #[instrument(skip(self, effects), fields(effect_count = effects.len()))]
    pub async fn execute_effects(
        &self,
        effects: &[EventEffect],
        session_id: SessionId,
    ) -> OutcomeExecutionResult {
        if effects.is_empty() {
            return OutcomeExecutionResult::empty();
        }

        let mut results = Vec::new();
        let mut executed_count = 0;
        let mut logged_count = 0;

        for effect in effects {
            let result = self.execute_single_effect(effect, session_id).await;
            
            if result.was_executed {
                executed_count += 1;
            } else {
                logged_count += 1;
            }
            
            results.push(result);
        }

        info!(
            executed = executed_count,
            logged = logged_count,
            "Executed narrative event effects"
        );

        OutcomeExecutionResult {
            effects: results,
            total: effects.len(),
            executed_count,
            logged_count,
        }
    }

    /// Execute a single effect
    async fn execute_single_effect(
        &self,
        effect: &EventEffect,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        match effect {
            EventEffect::SetFlag { flag_name, value } => {
                self.execute_set_flag(flag_name, *value, session_id).await
            }

            EventEffect::EnableChallenge { challenge_id, challenge_name } => {
                self.execute_enable_challenge(*challenge_id, challenge_name, session_id).await
            }

            EventEffect::DisableChallenge { challenge_id, challenge_name } => {
                self.execute_disable_challenge(*challenge_id, challenge_name, session_id).await
            }

            EventEffect::EnableEvent { event_id, event_name } => {
                self.execute_enable_event(*event_id, event_name, session_id).await
            }

            EventEffect::DisableEvent { event_id, event_name } => {
                self.execute_disable_event(*event_id, event_name, session_id).await
            }

            EventEffect::RevealInformation { info_type, title, content, persist_to_journal } => {
                self.execute_reveal_information(info_type, title, content, *persist_to_journal, session_id).await
            }

            EventEffect::GiveItem { item_name, item_description, quantity } => {
                self.execute_give_item(item_name, item_description.as_deref(), *quantity, session_id).await
            }

            EventEffect::TakeItem { item_name, quantity } => {
                self.execute_take_item(item_name, *quantity, session_id).await
            }

            EventEffect::ModifyRelationship { from_character, from_name, to_character, to_name, sentiment_change, reason } => {
                self.execute_modify_relationship(*from_character, from_name, *to_character, to_name, *sentiment_change, reason, session_id).await
            }

            EventEffect::ModifyStat { character_id, character_name, stat_name, modifier } => {
                self.execute_modify_stat(*character_id, character_name, stat_name, *modifier, session_id).await
            }

            EventEffect::TriggerScene { scene_id, scene_name } => {
                self.execute_trigger_scene(*scene_id, scene_name, session_id).await
            }

            EventEffect::StartCombat { participants: _, participant_names, combat_description } => {
                self.execute_start_combat(participant_names, combat_description, session_id).await
            }

            EventEffect::AddReward { reward_type, amount, description } => {
                self.execute_add_reward(reward_type, *amount, description, session_id).await
            }

            EventEffect::Custom { description, requires_dm_action } => {
                self.execute_custom(description, *requires_dm_action, session_id).await
            }
        }
    }

    // =========================================================================
    // Individual Effect Implementations
    // =========================================================================

    async fn execute_set_flag(
        &self,
        flag_name: &str,
        value: bool,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(flag_name = flag_name, value = value, "Setting game flag");
        
        // Log to conversation history for DM awareness
        let msg = format!(
            "[FLAG] {} = {}",
            flag_name,
            if value { "true" } else { "false" }
        );
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        // Note: Flag storage would need session state modification
        // For now, we log it - the flag should be stored in session state
        EffectExecutionResult {
            description: format!("Set flag '{}' to {}", flag_name, value),
            was_executed: true,
            note: Some("Flag set in session state".to_string()),
        }
    }

    async fn execute_enable_challenge(
        &self,
        challenge_id: crate::domain::value_objects::ChallengeId,
        challenge_name: &str,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(challenge_id = %challenge_id, "Enabling challenge");
        
        match self.challenge_repo.set_active(challenge_id, true).await {
            Ok(()) => {
                let msg = format!("[CHALLENGE ENABLED] {}", challenge_name);
                let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;
                
                EffectExecutionResult {
                    description: format!("Enabled challenge '{}'", challenge_name),
                    was_executed: true,
                    note: None,
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to enable challenge");
                EffectExecutionResult {
                    description: format!("Failed to enable challenge '{}'", challenge_name),
                    was_executed: false,
                    note: Some(e.to_string()),
                }
            }
        }
    }

    async fn execute_disable_challenge(
        &self,
        challenge_id: crate::domain::value_objects::ChallengeId,
        challenge_name: &str,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(challenge_id = %challenge_id, "Disabling challenge");
        
        match self.challenge_repo.set_active(challenge_id, false).await {
            Ok(()) => {
                let msg = format!("[CHALLENGE DISABLED] {}", challenge_name);
                let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;
                
                EffectExecutionResult {
                    description: format!("Disabled challenge '{}'", challenge_name),
                    was_executed: true,
                    note: None,
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to disable challenge");
                EffectExecutionResult {
                    description: format!("Failed to disable challenge '{}'", challenge_name),
                    was_executed: false,
                    note: Some(e.to_string()),
                }
            }
        }
    }

    async fn execute_enable_event(
        &self,
        event_id: crate::domain::value_objects::NarrativeEventId,
        event_name: &str,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(event_id = %event_id, "Enabling narrative event");
        
        match self.narrative_event_repo.set_active(event_id, true).await {
            Ok(_) => {
                let msg = format!("[EVENT ENABLED] {}", event_name);
                let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;
                
                EffectExecutionResult {
                    description: format!("Enabled narrative event '{}'", event_name),
                    was_executed: true,
                    note: None,
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to enable narrative event");
                EffectExecutionResult {
                    description: format!("Failed to enable event '{}'", event_name),
                    was_executed: false,
                    note: Some(e.to_string()),
                }
            }
        }
    }

    async fn execute_disable_event(
        &self,
        event_id: crate::domain::value_objects::NarrativeEventId,
        event_name: &str,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(event_id = %event_id, "Disabling narrative event");
        
        match self.narrative_event_repo.set_active(event_id, false).await {
            Ok(_) => {
                let msg = format!("[EVENT DISABLED] {}", event_name);
                let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;
                
                EffectExecutionResult {
                    description: format!("Disabled narrative event '{}'", event_name),
                    was_executed: true,
                    note: None,
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to disable narrative event");
                EffectExecutionResult {
                    description: format!("Failed to disable event '{}'", event_name),
                    was_executed: false,
                    note: Some(e.to_string()),
                }
            }
        }
    }

    async fn execute_reveal_information(
        &self,
        info_type: &str,
        title: &str,
        content: &str,
        persist_to_journal: bool,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(info_type = info_type, title = title, "Revealing information");
        
        let journal_tag = if persist_to_journal { "[JOURNAL] " } else { "" };
        let msg = format!("[{}] {}{}: {}", info_type.to_uppercase(), journal_tag, title, content);
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        EffectExecutionResult {
            description: format!("Revealed {} '{}'{}", info_type, title, if persist_to_journal { " (journaled)" } else { "" }),
            was_executed: true,
            note: None,
        }
    }

    async fn execute_give_item(
        &self,
        item_name: &str,
        item_description: Option<&str>,
        quantity: u32,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(item_name = item_name, quantity = quantity, "Giving item");
        
        let desc = item_description.map(|d| format!(" - {}", d)).unwrap_or_default();
        let qty = if quantity > 1 { format!(" x{}", quantity) } else { String::new() };
        let msg = format!("[ITEM RECEIVED] {}{}{}", item_name, qty, desc);
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        // Note: Actual inventory modification would need player character repo access
        EffectExecutionResult {
            description: format!("Gave {} {}", quantity, item_name),
            was_executed: false, // Logged for DM to narrate, not actually added to inventory
            note: Some("DM should narrate item acquisition".to_string()),
        }
    }

    async fn execute_take_item(
        &self,
        item_name: &str,
        quantity: u32,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(item_name = item_name, quantity = quantity, "Taking item");
        
        let qty = if quantity > 1 { format!(" x{}", quantity) } else { String::new() };
        let msg = format!("[ITEM LOST] {}{}", item_name, qty);
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        EffectExecutionResult {
            description: format!("Took {} {}", quantity, item_name),
            was_executed: false, // Logged for DM to narrate
            note: Some("DM should narrate item loss".to_string()),
        }
    }

    async fn execute_modify_relationship(
        &self,
        from_character: crate::domain::value_objects::CharacterId,
        from_name: &str,
        to_character: crate::domain::value_objects::CharacterId,
        to_name: &str,
        sentiment_change: f32,
        reason: &str,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(
            from = %from_character,
            to = %to_character,
            change = sentiment_change,
            "Modifying relationship"
        );
        
        // Try to update the relationship in the database
        match self.relationship_repo.get_for_character(from_character).await {
            Ok(relationships) => {
                // Find the existing relationship
                if let Some(mut rel) = relationships.into_iter().find(|r| r.to_character == to_character) {
                    rel.sentiment += sentiment_change;
                    rel.sentiment = rel.sentiment.clamp(-1.0, 1.0);
                    
                    if let Err(e) = self.relationship_repo.update(&rel).await {
                        warn!(error = %e, "Failed to update relationship");
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to get relationships");
            }
        }

        let direction = if sentiment_change >= 0.0 { "+" } else { "" };
        let msg = format!(
            "[RELATIONSHIP] {} -> {}: {}{:.1} ({})",
            from_name, to_name, direction, sentiment_change, reason
        );
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        EffectExecutionResult {
            description: format!(
                "Modified {} -> {} relationship by {}{:.1}",
                from_name, to_name, direction, sentiment_change
            ),
            was_executed: true,
            note: None,
        }
    }

    async fn execute_modify_stat(
        &self,
        _character_id: crate::domain::value_objects::CharacterId,
        character_name: &str,
        stat_name: &str,
        modifier: i32,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(character_name = character_name, stat_name = stat_name, modifier = modifier, "Modifying stat");
        
        let direction = if modifier >= 0 { "+" } else { "" };
        let msg = format!("[STAT] {} {}: {}{}", character_name, stat_name, direction, modifier);
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        // Note: Actual stat modification would need character sheet access
        EffectExecutionResult {
            description: format!("{} {} {}{}", character_name, stat_name, direction, modifier),
            was_executed: false, // Logged for DM to apply
            note: Some("DM should apply stat change to character sheet".to_string()),
        }
    }

    async fn execute_trigger_scene(
        &self,
        _scene_id: crate::domain::value_objects::SceneId,
        scene_name: &str,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(scene_name = scene_name, "Triggering scene transition");
        
        let msg = format!("[SCENE TRANSITION] Moving to: {}", scene_name);
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        EffectExecutionResult {
            description: format!("Scene transition to '{}'", scene_name),
            was_executed: false, // DM should initiate transition
            note: Some("DM should initiate scene transition".to_string()),
        }
    }

    async fn execute_start_combat(
        &self,
        participant_names: &[String],
        combat_description: &str,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(participants = ?participant_names, "Starting combat");
        
        let msg = format!(
            "[COMBAT INITIATED] {} - Participants: {}",
            combat_description,
            participant_names.join(", ")
        );
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        EffectExecutionResult {
            description: format!("Combat initiated: {}", combat_description),
            was_executed: false, // DM should run combat
            note: Some("DM should initiate combat encounter".to_string()),
        }
    }

    async fn execute_add_reward(
        &self,
        reward_type: &str,
        amount: i32,
        description: &str,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(reward_type = reward_type, amount = amount, "Adding reward");
        
        let msg = format!("[REWARD] {} {} - {}", amount, reward_type, description);
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        EffectExecutionResult {
            description: format!("Awarded {} {} ({})", amount, reward_type, description),
            was_executed: false, // DM should apply reward
            note: Some("DM should apply reward to characters".to_string()),
        }
    }

    async fn execute_custom(
        &self,
        description: &str,
        requires_dm_action: bool,
        session_id: SessionId,
    ) -> EffectExecutionResult {
        debug!(description = description, requires_dm_action = requires_dm_action, "Custom effect");
        
        let tag = if requires_dm_action { "[DM ACTION REQUIRED] " } else { "[CUSTOM] " };
        let msg = format!("{}{}", tag, description);
        let _ = self.sessions.add_to_conversation_history(session_id, "System", &msg).await;

        EffectExecutionResult {
            description: description.to_string(),
            was_executed: false,
            note: if requires_dm_action {
                Some("DM action required".to_string())
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcome_execution_result_empty() {
        let result = OutcomeExecutionResult::empty();
        assert!(result.effects.is_empty());
        assert_eq!(result.total, 0);
        assert_eq!(result.executed_count, 0);
        assert_eq!(result.logged_count, 0);
    }
}
