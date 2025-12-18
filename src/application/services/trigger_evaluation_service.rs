//! Trigger Evaluation Service - Evaluates narrative event triggers
//!
//! This service is responsible for checking if any active narrative events
//! have satisfied triggers. It queries the graph for entity-based triggers
//! and checks JSON-stored triggers (flags, stats, custom, time).
//!
//! # Architecture
//!
//! The service follows hexagonal architecture:
//! - Depends on repository ports for data access
//! - Returns domain-level results
//! - Can be called by the DM approval queue or game loop
//!
//! # Trigger Sources
//!
//! Events can be triggered from two sources:
//! 1. **Engine-detected**: This service evaluates game state against trigger conditions
//! 2. **LLM-suggested**: The LLM can suggest triggers via `<narrative_event_suggestion>` tags
//!
//! Both sources feed into the DM approval queue before execution.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{debug, info, instrument, warn};

use crate::application::ports::outbound::{
    ChallengeRepositoryPort, CharacterRepositoryPort, NarrativeEventRepositoryPort,
    PlayerCharacterRepositoryPort, StoryEventRepositoryPort,
};
use crate::domain::entities::{NarrativeEvent, TriggerContext, TriggerEvaluation};
use crate::domain::value_objects::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SessionId, WorldId,
};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during trigger evaluation
#[derive(Debug, thiserror::Error)]
pub enum TriggerEvaluationError {
    #[error("Failed to fetch narrative events: {0}")]
    EventFetch(String),

    #[error("Failed to fetch game state: {0}")]
    StateFetch(String),

    #[error("Failed to build trigger context: {0}")]
    ContextBuild(String),

    #[error("Repository error: {0}")]
    Repository(#[from] anyhow::Error),
}

// =============================================================================
// Result Types
// =============================================================================

/// Source of a trigger suggestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TriggerSource {
    /// Engine detected that trigger conditions are satisfied
    Engine,
    /// LLM suggested this event should trigger
    Llm,
    /// DM manually triggered the event
    DmManual,
}

/// A narrative event that has been evaluated and is ready for triggering
#[derive(Debug, Clone)]
pub struct TriggeredEventCandidate {
    /// The event that may trigger
    pub event: NarrativeEvent,
    /// Evaluation result showing which triggers matched
    pub evaluation: TriggerEvaluation,
    /// Source of this trigger suggestion
    pub source: TriggerSource,
    /// Optional reason (for LLM suggestions)
    pub reason: Option<String>,
}

/// Result of evaluating all active triggers
#[derive(Debug, Clone)]
pub struct TriggerEvaluationResult {
    /// Events that are ready to trigger (all conditions met)
    pub ready_to_trigger: Vec<TriggeredEventCandidate>,
    /// Events that are partially satisfied (for DM visibility)
    pub partially_satisfied: Vec<TriggeredEventCandidate>,
    /// Total events evaluated
    pub total_evaluated: usize,
}

impl TriggerEvaluationResult {
    pub fn empty() -> Self {
        Self {
            ready_to_trigger: Vec::new(),
            partially_satisfied: Vec::new(),
            total_evaluated: 0,
        }
    }
}

// =============================================================================
// Game State for Trigger Context
// =============================================================================

/// Game state snapshot used to build trigger context
///
/// This struct holds the current state of the game session that's needed
/// to evaluate trigger conditions. It can be built from various sources
/// (repositories, session state, etc.)
#[derive(Debug, Clone, Default)]
pub struct GameStateSnapshot {
    /// Current player location
    pub current_location_id: Option<LocationId>,
    
    /// Character the player is currently talking to (if any)
    pub talking_to_character_id: Option<CharacterId>,
    
    /// Challenge that was just completed (if any)
    pub just_completed_challenge: Option<CompletedChallenge>,
    
    /// Narrative event that was just completed (if any)
    pub just_completed_event: Option<CompletedNarrativeEvent>,
    
    /// Game flags (boolean flags set during gameplay)
    pub flags: HashMap<String, bool>,
    
    /// Player inventory (item names)
    pub inventory: Vec<String>,
    
    /// IDs of completed narrative events
    pub completed_event_ids: Vec<NarrativeEventId>,
    
    /// Outcomes of completed events (event_id -> outcome_name)
    pub event_outcomes: HashMap<NarrativeEventId, String>,
    
    /// IDs of completed challenges
    pub completed_challenge_ids: Vec<ChallengeId>,
    
    /// Success status of completed challenges
    pub challenge_successes: HashMap<ChallengeId, bool>,
    
    /// Turns elapsed since event (for TurnCount triggers)
    pub turns_since_event: HashMap<NarrativeEventId, u32>,
    
    /// Total turn count for the session
    pub turn_count: u32,
    
    /// Recent dialogue topics (keywords from conversation)
    pub recent_dialogue_topics: Vec<String>,
}

/// Information about a completed challenge
#[derive(Debug, Clone)]
pub struct CompletedChallenge {
    pub challenge_id: ChallengeId,
    pub was_successful: bool,
}

/// Information about a completed narrative event
#[derive(Debug, Clone)]
pub struct CompletedNarrativeEvent {
    pub event_id: NarrativeEventId,
    pub outcome_name: String,
}

impl GameStateSnapshot {
    /// Convert to the domain TriggerContext used by NarrativeEvent::evaluate_triggers
    pub fn to_trigger_context(&self) -> TriggerContext {
        TriggerContext {
            current_location: self.current_location_id,
            current_scene: None, // Scene context could be added if needed
            time_context: None,
            flags: self.flags.clone(),
            inventory: self.inventory.clone(),
            completed_events: self.completed_event_ids.clone(),
            event_outcomes: self.event_outcomes.clone(),
            turns_since_event: self.turns_since_event.clone(),
            completed_challenges: self.completed_challenge_ids.clone(),
            challenge_successes: self.challenge_successes.clone(),
            turn_count: self.turn_count,
            recent_dialogue_topics: self.recent_dialogue_topics.clone(),
            recent_player_action: None,
        }
    }
}

// =============================================================================
// Service Implementation
// =============================================================================

/// Service for evaluating narrative event triggers
///
/// This service checks all active narrative events to see if their trigger
/// conditions are satisfied. Events that pass evaluation are candidates
/// for DM approval before being executed.
pub struct TriggerEvaluationService {
    narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort>,
    challenge_repo: Arc<dyn ChallengeRepositoryPort>,
    character_repo: Arc<dyn CharacterRepositoryPort>,
    player_character_repo: Arc<dyn PlayerCharacterRepositoryPort>,
    story_event_repo: Arc<dyn StoryEventRepositoryPort>,
}

impl TriggerEvaluationService {
    /// Create a new TriggerEvaluationService
    pub fn new(
        narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort>,
        challenge_repo: Arc<dyn ChallengeRepositoryPort>,
        character_repo: Arc<dyn CharacterRepositoryPort>,
        player_character_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        story_event_repo: Arc<dyn StoryEventRepositoryPort>,
    ) -> Self {
        Self {
            narrative_event_repo,
            challenge_repo,
            character_repo,
            player_character_repo,
            story_event_repo,
        }
    }

    /// Evaluate all active narrative events for a world
    ///
    /// This method fetches all active (non-triggered) events and evaluates
    /// their trigger conditions against the provided game state.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to evaluate events for
    /// * `game_state` - Current game state snapshot
    ///
    /// # Returns
    ///
    /// A result containing events ready to trigger and partially satisfied events.
    #[instrument(skip(self, game_state), fields(world_id = %world_id))]
    pub async fn evaluate_triggers(
        &self,
        world_id: WorldId,
        game_state: &GameStateSnapshot,
    ) -> Result<TriggerEvaluationResult, TriggerEvaluationError> {
        // Fetch all active (pending) narrative events
        let active_events = self
            .narrative_event_repo
            .list_pending(world_id)
            .await
            .map_err(|e| TriggerEvaluationError::EventFetch(e.to_string()))?;

        if active_events.is_empty() {
            debug!("No active narrative events to evaluate");
            return Ok(TriggerEvaluationResult::empty());
        }

        info!(
            event_count = active_events.len(),
            "Evaluating narrative event triggers"
        );

        // Convert game state to trigger context
        let trigger_context = game_state.to_trigger_context();

        let mut ready_to_trigger = Vec::new();
        let mut partially_satisfied = Vec::new();

        for event in active_events {
            let evaluation = event.evaluate_triggers(&trigger_context);

            debug!(
                event_id = %event.id,
                event_name = %event.name,
                is_triggered = evaluation.is_triggered,
                confidence = evaluation.confidence,
                matched = evaluation.matched_triggers.len(),
                total = evaluation.total_triggers,
                "Evaluated event triggers"
            );

            let candidate = TriggeredEventCandidate {
                event: event.clone(),
                evaluation: evaluation.clone(),
                source: TriggerSource::Engine,
                reason: None,
            };

            if evaluation.is_triggered {
                ready_to_trigger.push(candidate);
            } else if evaluation.confidence > 0.0 {
                // At least some triggers matched
                partially_satisfied.push(candidate);
            }
        }

        // Sort by priority (higher priority first)
        ready_to_trigger.sort_by(|a, b| b.event.priority.cmp(&a.event.priority));
        partially_satisfied.sort_by(|a, b| b.evaluation.confidence.partial_cmp(&a.evaluation.confidence).unwrap_or(std::cmp::Ordering::Equal));

        let total = ready_to_trigger.len() + partially_satisfied.len();

        info!(
            ready = ready_to_trigger.len(),
            partial = partially_satisfied.len(),
            "Trigger evaluation complete"
        );

        Ok(TriggerEvaluationResult {
            ready_to_trigger,
            partially_satisfied,
            total_evaluated: total,
        })
    }

    /// Check if a specific event's triggers are satisfied
    ///
    /// This is useful for checking a single event without evaluating all events.
    #[instrument(skip(self, game_state), fields(event_id = %event_id))]
    pub async fn check_event_triggers(
        &self,
        event_id: NarrativeEventId,
        game_state: &GameStateSnapshot,
    ) -> Result<Option<TriggeredEventCandidate>, TriggerEvaluationError> {
        let event = self
            .narrative_event_repo
            .get(event_id)
            .await
            .map_err(|e| TriggerEvaluationError::EventFetch(e.to_string()))?;

        let Some(event) = event else {
            warn!(event_id = %event_id, "Event not found for trigger check");
            return Ok(None);
        };

        // Skip if already triggered (and not repeatable)
        if event.is_triggered && !event.is_repeatable {
            debug!(event_id = %event_id, "Event already triggered and not repeatable");
            return Ok(None);
        }

        // Skip if not active
        if !event.is_active {
            debug!(event_id = %event_id, "Event is not active");
            return Ok(None);
        }

        let trigger_context = game_state.to_trigger_context();
        let evaluation = event.evaluate_triggers(&trigger_context);

        if evaluation.is_triggered {
            Ok(Some(TriggeredEventCandidate {
                event,
                evaluation,
                source: TriggerSource::Engine,
                reason: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Build a game state snapshot from repositories
    ///
    /// This helper method builds a GameStateSnapshot by querying the repositories
    /// for the current state. It's useful when you don't have a pre-built snapshot.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to build state for
    /// * `session_id` - The current session
    /// * `player_character_id` - Optional player character to get location/inventory from
    /// * `immediate_context` - Optional immediate context (just completed challenge, etc.)
    #[instrument(skip(self, immediate_context))]
    pub async fn build_game_state_snapshot(
        &self,
        world_id: WorldId,
        _session_id: SessionId,
        player_character_id: Option<crate::domain::value_objects::PlayerCharacterId>,
        immediate_context: Option<ImmediateContext>,
    ) -> Result<GameStateSnapshot, TriggerEvaluationError> {
        let mut snapshot = GameStateSnapshot::default();

        // Get player character location if provided
        if let Some(pc_id) = player_character_id {
            if let Ok(Some(pc)) = self.player_character_repo.get(pc_id).await {
                snapshot.current_location_id = Some(pc.current_location_id);
                // TODO: Get inventory from player character when implemented
            }
        }

        // Get completed narrative events
        let triggered_events = self
            .narrative_event_repo
            .list_by_world(world_id)
            .await
            .map_err(|e| TriggerEvaluationError::StateFetch(e.to_string()))?;

        for event in triggered_events {
            if event.is_triggered {
                snapshot.completed_event_ids.push(event.id);
                if let Some(outcome) = event.selected_outcome {
                    snapshot.event_outcomes.insert(event.id, outcome);
                }
            }
        }

        // Get completed challenges from story events
        // Note: This is a simplified approach - in production, you might want
        // a dedicated challenge completion tracking system
        let story_events = self
            .story_event_repo
            .list_by_world(world_id)
            .await
            .map_err(|e| TriggerEvaluationError::StateFetch(e.to_string()))?;

        for story_event in story_events {
            // Check if this story event records a challenge
            if let Ok(Some(challenge_id)) = self.story_event_repo.get_recorded_challenge(story_event.id).await {
                snapshot.completed_challenge_ids.push(challenge_id);
                // Determine success from story event tags or data
                let was_success = story_event.tags.iter().any(|t| t == "success" || t == "challenge_success");
                snapshot.challenge_successes.insert(challenge_id, was_success);
            }
        }

        // Apply immediate context if provided
        if let Some(ctx) = immediate_context {
            if let Some(challenge) = ctx.just_completed_challenge {
                snapshot.just_completed_challenge = Some(challenge.clone());
                // Also add to the completed lists
                snapshot.completed_challenge_ids.push(challenge.challenge_id);
                snapshot.challenge_successes.insert(challenge.challenge_id, challenge.was_successful);
            }
            if let Some(event) = ctx.just_completed_event {
                snapshot.just_completed_event = Some(event.clone());
                snapshot.completed_event_ids.push(event.event_id);
                snapshot.event_outcomes.insert(event.event_id, event.outcome_name.clone());
            }
            if let Some(char_id) = ctx.talking_to_character_id {
                snapshot.talking_to_character_id = Some(char_id);
            }
            if !ctx.recent_dialogue_topics.is_empty() {
                snapshot.recent_dialogue_topics = ctx.recent_dialogue_topics;
            }
            snapshot.flags = ctx.game_flags;
            snapshot.turn_count = ctx.turn_count;
        }

        Ok(snapshot)
    }

    /// Create an LLM-suggested trigger candidate
    ///
    /// This method creates a TriggeredEventCandidate from an LLM suggestion.
    /// The event is validated to ensure it exists and is active.
    #[instrument(skip(self))]
    pub async fn create_llm_suggestion(
        &self,
        event_id: NarrativeEventId,
        reason: String,
    ) -> Result<Option<TriggeredEventCandidate>, TriggerEvaluationError> {
        let event = self
            .narrative_event_repo
            .get(event_id)
            .await
            .map_err(|e| TriggerEvaluationError::EventFetch(e.to_string()))?;

        let Some(event) = event else {
            warn!(event_id = %event_id, "LLM suggested event not found");
            return Ok(None);
        };

        // Validate the event is triggerable
        if !event.is_active {
            warn!(event_id = %event_id, "LLM suggested event is not active");
            return Ok(None);
        }

        if event.is_triggered && !event.is_repeatable {
            warn!(event_id = %event_id, "LLM suggested event already triggered");
            return Ok(None);
        }

        // Create a candidate with LLM source
        // Note: We create a "synthetic" evaluation since the LLM bypassed normal trigger checks
        let evaluation = TriggerEvaluation {
            is_triggered: true, // LLM says it should trigger
            matched_triggers: vec!["llm_suggested".to_string()],
            unmatched_triggers: vec![],
            total_triggers: 1,
            confidence: 1.0,
        };

        Ok(Some(TriggeredEventCandidate {
            event,
            evaluation,
            source: TriggerSource::Llm,
            reason: Some(reason),
        }))
    }
}

/// Immediate context for trigger evaluation
///
/// This struct holds context about things that just happened in the current
/// game turn, which may not yet be persisted to repositories.
#[derive(Debug, Clone, Default)]
pub struct ImmediateContext {
    /// Challenge that was just completed this turn
    pub just_completed_challenge: Option<CompletedChallenge>,
    
    /// Narrative event that was just completed this turn
    pub just_completed_event: Option<CompletedNarrativeEvent>,
    
    /// Character being talked to
    pub talking_to_character_id: Option<CharacterId>,
    
    /// Recent dialogue topics from this conversation
    pub recent_dialogue_topics: Vec<String>,
    
    /// Current game flags
    pub game_flags: HashMap<String, bool>,
    
    /// Current turn count
    pub turn_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{NarrativeTrigger, NarrativeTriggerType, TriggerLogic};

    #[test]
    fn test_game_state_to_trigger_context() {
        let mut state = GameStateSnapshot::default();
        state.current_location_id = Some(LocationId::new());
        state.flags.insert("found_key".to_string(), true);
        state.inventory.push("magic_sword".to_string());
        state.turn_count = 5;

        let context = state.to_trigger_context();

        assert!(context.current_location.is_some());
        assert_eq!(context.flags.get("found_key"), Some(&true));
        assert!(context.inventory.contains(&"magic_sword".to_string()));
        assert_eq!(context.turn_count, 5);
    }

    #[test]
    fn test_trigger_evaluation_result_empty() {
        let result = TriggerEvaluationResult::empty();
        assert!(result.ready_to_trigger.is_empty());
        assert!(result.partially_satisfied.is_empty());
        assert_eq!(result.total_evaluated, 0);
    }

    #[test]
    fn test_triggered_event_candidate_creation() {
        let mut event = NarrativeEvent::new(WorldId::new(), "Test Event");
        event.trigger_conditions.push(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::FlagSet {
                flag_name: "test_flag".to_string(),
            },
            description: "Test flag is set".to_string(),
            is_required: false,
            trigger_id: "t1".to_string(),
        });
        event.trigger_logic = TriggerLogic::All;

        let evaluation = TriggerEvaluation {
            is_triggered: true,
            matched_triggers: vec!["t1".to_string()],
            unmatched_triggers: vec![],
            total_triggers: 1,
            confidence: 1.0,
        };

        let candidate = TriggeredEventCandidate {
            event,
            evaluation,
            source: TriggerSource::Engine,
            reason: None,
        };

        assert_eq!(candidate.source, TriggerSource::Engine);
        assert!(candidate.evaluation.is_triggered);
    }
}
